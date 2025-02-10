pub mod config;
pub mod errors;
pub mod models;
pub mod services;
pub mod storage;
pub mod traits;
pub mod worker_name_store;

use crate::config::SETTINGS;
use crate::models::BlockFound;
use crate::models::ShareLog;
use crate::storage::clickhouse::ClickhouseBlockStorage;
use crate::storage::clickhouse::ClickhouseStorage;
use crate::traits::ShareStorage;
use lazy_static::lazy_static;
use log::info;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::sync::Arc;
use std::time::Instant;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{
    mpsc::{self, error::TrySendError},
    Mutex,
};
use tokio::time::Duration;

use nomium_prometheus::{
    SHALOG_BACKUP_CHANNEL_CURRENT, SHALOG_BACKUP_CHANNEL_SHARES_TOTAL,
    SHALOG_BACKUP_STORE_FAILED_TOTAL, SHALOG_BACKUP_TRY_STORED_TOTAL,
    SHALOG_PRIMARY_CHANNEL_CURRENT, SHALOG_PRIMARY_CHANNEL_SHARES_TOTAL,
    SHALOG_PRIMARY_STORE_FAILED_TOTAL, SHALOG_PRIMARY_TRY_STORED_TOTAL,
    SHALOG_SHARES_RECEIVED_TOTAL,
};

lazy_static! {
    static ref GLOBAL_LOGGER: ShareLogger<ShareLog> = {
        let storage = ClickhouseStorage::new().expect("Failed to create ClickHouse storage");
        ShareLoggerBuilder::<ShareLog>::new(Box::new(storage)).build()
    };
}

lazy_static! {
    static ref BLOCK_LOGGER: ShareLogger<BlockFound> = {
        let storage =
            ClickhouseBlockStorage::new().expect("Failed to create ClickHouse block storage");
        ShareLoggerBuilder::<BlockFound>::new(Box::new(storage)).build()
    };
}

pub fn log_share(share: ShareLog) {
    GLOBAL_LOGGER.log_share(share);
}

pub fn log_block(block: BlockFound) {
    BLOCK_LOGGER.log_share(block);
}

pub struct ShareLogger<T: Send + Sync + Clone + Serialize + DeserializeOwned> {
    primary_tx: mpsc::Sender<T>,
    backup_tx: mpsc::UnboundedSender<T>,
}

pub struct ShareLoggerBuilder<T: Send + Sync + Clone + Serialize + DeserializeOwned> {
    storage: Arc<Mutex<Box<dyn ShareStorage<T>>>>,
    primary_channel_size: Option<usize>,
    backup_check_interval: Option<Duration>,
}

impl<T: Send + Sync + Clone + Serialize + DeserializeOwned + 'static> ShareLoggerBuilder<T> {
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
        let primary_channel_size = self
            .primary_channel_size
            .unwrap_or(SETTINGS.processing.primary_channel_buffer_size);
        let backup_check_interval = self.backup_check_interval.unwrap_or(Duration::from_secs(
            SETTINGS.processing.backup_check_interval_secs,
        ));

        let (primary_tx, primary_rx) = mpsc::channel(primary_channel_size);
        let (backup_tx, backup_rx) = mpsc::unbounded_channel();

        let storage = self.storage.clone();

        tokio::spawn(async move {
            process_shares(primary_rx, backup_rx, storage, backup_check_interval).await;
        });

        ShareLogger {
            primary_tx,
            backup_tx,
        }
    }
}

impl<T: Send + Sync + Clone + Serialize + DeserializeOwned + 'static> ShareLogger<T> {
    pub fn log_share(&self, share: T) {
        match self.primary_tx.try_send(share.clone()) {
            Ok(_) => {
                SHALOG_PRIMARY_CHANNEL_SHARES_TOTAL.inc();
                SHALOG_PRIMARY_CHANNEL_CURRENT.inc();
            }
            Err(TrySendError::Full(share)) | Err(TrySendError::Closed(share)) => {
                SHALOG_BACKUP_CHANNEL_SHARES_TOTAL.inc();
                SHALOG_BACKUP_CHANNEL_CURRENT.inc();
                if let Err(e) = self.backup_tx.send(share) {
                    info!("Failed to send share to backup logger: {}", e);
                }
            }
        }
    }
}

async fn process_shares<T: Send + Sync + Clone + Serialize + DeserializeOwned>(
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
                SHALOG_PRIMARY_TRY_STORED_TOTAL.inc();
                SHALOG_PRIMARY_CHANNEL_CURRENT.dec();
                if let Err(e) = storage.lock().await.store_share(share).await {
                    SHALOG_PRIMARY_STORE_FAILED_TOTAL.inc();
                    info!("Failed to store share: {}", e);
                }
            }
            _ = backup_interval.tick() => {
                let mut backup_shares = Vec::new();
                while let Ok(share) = backup_rx.try_recv() {
                    backup_shares.push(share);
                    SHALOG_BACKUP_CHANNEL_CURRENT.dec();
                }
                if !backup_shares.is_empty() {
                    let shares_count = backup_shares.len() as u64;
                    SHALOG_BACKUP_TRY_STORED_TOTAL.inc_by(shares_count);
                    if let Err(e) = storage.lock().await.store_batch(backup_shares).await {
                        SHALOG_BACKUP_STORE_FAILED_TOTAL.inc_by(shares_count);
                        info!("Failed to store backup shares: {}", e);
                    }
                }
            }
        }
    }
}

pub fn get_utc_now() -> i64 {
    let now = SystemTime::now();
    let duration_since_epoch = now.duration_since(UNIX_EPOCH).expect("Time went backwards");
    duration_since_epoch.as_millis() as i64
}
