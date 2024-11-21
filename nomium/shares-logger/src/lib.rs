pub mod config;
pub mod errors;
pub mod models;
pub mod services;
pub mod storage;
pub mod traits;

use crate::config::SETTINGS;
use crate::models::ShareLog;
use crate::traits::ShareStorage;
use log::info;
use std::sync::Arc;
use tokio::sync::{mpsc::{self, error::TrySendError}, Mutex};
use tokio::time::Duration;
use lazy_static::lazy_static;
use crate::storage::clickhouse::ClickhouseStorage;

lazy_static! {
    static ref GLOBAL_LOGGER: ShareLogger = {
        let storage = ClickhouseStorage::new()
            .expect("Failed to create ClickHouse storage");
        ShareLoggerBuilder::new(Box::new(storage))
            .build()
    };
}

pub fn log_share(share: ShareLog) {
    GLOBAL_LOGGER.log_share(share);
}

pub struct ShareLogger {
    primary_tx: mpsc::Sender<ShareLog>,
    backup_tx: mpsc::UnboundedSender<ShareLog>,
}

pub struct ShareLoggerBuilder {
    storage: Arc<Mutex<Box<dyn ShareStorage>>>,
    primary_channel_size: Option<usize>,
    backup_check_interval: Option<Duration>,
}

impl ShareLoggerBuilder {
    pub fn new(storage: Box<dyn ShareStorage>) -> Self {
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

    pub fn build(self) -> ShareLogger {
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

impl ShareLogger {
    pub fn log_share(&self, share: ShareLog) {
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

async fn process_shares(
    mut primary_rx: mpsc::Receiver<ShareLog>,
    mut backup_rx: mpsc::UnboundedReceiver<ShareLog>,
    storage: Arc<Mutex<Box<dyn ShareStorage>>>,
    backup_check_interval: Duration,
) {
    // Инициализация хранилища
    if let Err(e) = storage.lock().await.as_ref().init().await {
        log::error!("Failed to initialize storage: {}", e);
        return;
    }

    let mut backup_interval = tokio::time::interval(backup_check_interval);

    loop {
        tokio::select! {
            Some(share) = primary_rx.recv() => {
                info!("Processing share from primary channel");
                if let Err(e) = storage.lock().await.store_share(share).await {
                    info!("Failed to store share: {}", e);
                }
            }
            _ = backup_interval.tick() => {
                let mut backup_shares = Vec::new();
                while let Ok(share) = backup_rx.try_recv() {
                    backup_shares.push(share);
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