pub mod config;
pub mod errors;
pub mod models;
pub mod services;
pub mod storage;
pub mod traits;

use crate::config::SETTINGS;
use crate::traits::ShareData;
use log::info;
use std::sync::Arc;
use tokio::sync::{mpsc::{self, error::TrySendError}, Mutex};
use tokio::time::Duration;
use lazy_static::lazy_static;
use crate::storage::clickhouse::ClickhouseStorage;
use crate::models::ShareLog;
use crate::traits::ShareStorage;
use crate::storage::clickhouse::ClickhouseBlockStorage;
use crate::models::BlockFound;
use std::time::Instant;
use crate::models::AuthorizationLog;
use crate::services::authorization_processor::AuthorizationProcessor;
use crate::services::external_api::ExternalApiService;
use anyhow::Error;

const API_BASE_URL: &str = "https://qa.redrockpool.com/equipment-api/v1";
const API_KEY: &str = "ZcU8z5W87ufe";

lazy_static! {
    static ref GLOBAL_LOGGER: ShareLogger<ShareLog> = {
        let storage = ClickhouseStorage::new()
            .expect("Failed to create ClickHouse storage");
        ShareLoggerBuilder::<ShareLog>::new(Box::new(storage))
            .build()
    };
}

lazy_static! {
    static ref BLOCK_LOGGER: ShareLogger<BlockFound> = {
        let storage = ClickhouseBlockStorage::new()
            .expect("Failed to create ClickHouse block storage");
        ShareLoggerBuilder::<BlockFound>::new(Box::new(storage))
            .build()
    };
}

lazy_static! {
    static ref AUTHORIZATION_SENDER: mpsc::Sender<AuthorizationLog> = {
        let (sender, receiver) = mpsc::channel(100);
        let api_service = ExternalApiService::new(API_KEY, API_BASE_URL);
        tokio::spawn(async move {
            let mut processor = AuthorizationProcessor::new(receiver, api_service);
            if let Err(e) = processor.run().await {
                log::error!("Authorization processor error: {}", e);
            }
        });
        sender
    };
}

pub fn log_share(share: ShareLog) {
    GLOBAL_LOGGER.log_share(share);
}

pub fn log_block(block: BlockFound) {
    BLOCK_LOGGER.log_share(block);
}

pub fn log_authorize(name: &str, password: &str) {
    let auth_log = AuthorizationLog {
        name: name.to_string(),
        password: password.to_string(),
    };
    if let Err(e) = AUTHORIZATION_SENDER.try_send(auth_log) {
        log::error!("Failed to send authorization log: {}", e);
    }
}

pub struct ShareLogger<T: ShareData> {
    primary_tx: mpsc::Sender<T>,
    backup_tx: mpsc::UnboundedSender<T>,
}

pub struct ShareLoggerBuilder<T: ShareData> {
    storage: Arc<Mutex<Box<dyn ShareStorage<T>>>>,
    primary_channel_size: Option<usize>,
    backup_check_interval: Option<Duration>,
}

impl<T: ShareData + 'static> ShareLoggerBuilder<T> {

    pub fn new(storage: Box<dyn ShareStorage<T>>) -> Self {
        Self {
            storage: Arc::new(Mutex::new(storage)),
            primary_channel_size: None,
            backup_check_interval: None,
        }
    }

    pub fn with_primary_channel_size(mut self, size: usize) -> Self {
        self.primary_channel_size = Some(size);
        self
    }

    pub fn with_backup_check_interval(mut self, interval: Duration) -> Self {
        self.backup_check_interval = Some(interval);
        self
    }

    pub fn build(self) -> ShareLogger<T> {
        let primary_channel_size = self.primary_channel_size
            .unwrap_or(SETTINGS.processing.primary_channel_buffer_size);
        let backup_check_interval = self.backup_check_interval
            .unwrap_or(Duration::from_secs(SETTINGS.processing.backup_check_interval_secs));

        let (primary_tx, primary_rx) = mpsc::channel(primary_channel_size);
        let (backup_tx, backup_rx) = mpsc::unbounded_channel();

        let storage = self.storage.clone();

        tokio::spawn(async move {
            process_shares(
                primary_rx,
                backup_rx,
                storage,
                backup_check_interval
            ).await;
        });

        ShareLogger {
            primary_tx,
            backup_tx,
        }
    }
}

impl<T: ShareData + 'static> ShareLogger<T> {
    pub fn log_share(&self, share: T) {
        match self.primary_tx.try_send(share.clone()) {
            Ok(_) => (),
            Err(TrySendError::Full(share)) | Err(TrySendError::Closed(share)) => {
                if let Err(e) = self.backup_tx.send(share) {
                    info!("Failed to send share to backup logger: {}", e);
                }
            }
        }
    }
}

async fn process_shares<T: ShareData>(
    mut primary_rx: mpsc::Receiver<T>,
    mut backup_rx: mpsc::UnboundedReceiver<T>,
    storage: Arc<Mutex<Box<dyn ShareStorage<T>>>>,
    backup_check_interval: Duration,
) {
    let init_start = Instant::now();
    if let Err(e) = storage.lock().await.as_ref().init().await {
        log::error!("Failed to initialize storage: {}", e);
        return;
    }
    let init_duration = init_start.elapsed();
    info!("Storage initialized in: {:?}", init_duration);

    let mut backup_interval = tokio::time::interval(backup_check_interval);

    loop {
        tokio::select! {
            Some(share) = primary_rx.recv() => {
                info!("Processing share from primary channel");

                if share.is_block_found() {
                    if let Err(e) = storage.lock().await.store_share(share).await {
                        info!("Failed to store block: {}", e);
                    }
                } else {
                    if let Err(e) = storage.lock().await.store_share(share).await {
                        info!("Failed to store share: {}", e);
                    }
                }
            }
            _ = backup_interval.tick() => {
                let mut backup_shares = Vec::new();
                while let Ok(share) = backup_rx.try_recv() {
                    if share.is_block_found() {
                        if let Err(e) = storage.lock().await.store_share(share).await {
                            info!("Failed to store backup block: {}", e);
                        }
                    } else {
                        backup_shares.push(share);
                    }
                }

                if !backup_shares.is_empty() {
                    if let Err(e) = storage.lock().await.store_batch(backup_shares).await {
                        info!("Failed to store backup shares: {}", e);
                    }
                }
            }
        }
    }
}