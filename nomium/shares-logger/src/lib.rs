pub mod config;
pub mod errors;
pub mod models;
pub mod services;
pub mod storage;
pub mod traits;

use crate::config::SETTINGS;
use crate::models::ShareLog;
use crate::traits::ShareStorage;
use crate::storage::clickhouse::ClickhouseStorage;

use log::info;
use once_cell::sync::Lazy;
use std::time::Duration;
use tokio::sync::mpsc::{self, error::TrySendError};

struct LogChannels {
    primary: mpsc::Sender<ShareLog>,
    backup: mpsc::UnboundedSender<ShareLog>,
}

static LOGGER_CHANNELS: Lazy<LogChannels> = Lazy::new(|| {
    let (primary_tx, primary_rx) = mpsc::channel(SETTINGS.processing.primary_channel_buffer_size);
    let (backup_tx, backup_rx) = mpsc::unbounded_channel();
    
    let storage = ClickhouseStorage::new()
        .expect("Failed to initialize ClickhouseStorage");

    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async move {
            process_shares(primary_rx, backup_rx, storage).await;
        });
    });

    LogChannels {
        primary: primary_tx,
        backup: backup_tx,
    }
});

async fn process_shares(
    mut primary_rx: mpsc::Receiver<ShareLog>,
    mut backup_rx: mpsc::UnboundedReceiver<ShareLog>,
    mut storage: impl ShareStorage,
) {
    if let Err(e) = storage.init().await {
        log::error!("Failed to initialize storage: {}", e);
        return;
    }

    let mut backup_interval = tokio::time::interval(Duration::from_secs(
        SETTINGS.processing.backup_check_interval_secs,
    ));

    loop {
        tokio::select! {
            Some(share) = primary_rx.recv() => {
                info!("Processing share from primary channel");
                if let Err(e) = storage.store_share(share).await {
                    info!("Failed to store share: {}", e);
                }
            }
            _ = backup_interval.tick() => {
                let mut backup_shares = Vec::new();
                while let Ok(share) = backup_rx.try_recv() {
                    backup_shares.push(share);
                }
                
                if !backup_shares.is_empty() {
                    if let Err(e) = storage.store_batch(backup_shares).await {
                        info!("Failed to store backup shares: {}", e);
                    }
                }
            }
        }
    }
}

pub fn log_share(share: ShareLog) {
    match LOGGER_CHANNELS.primary.try_send(share.clone()) {
        Ok(_) => (),
        Err(TrySendError::Full(share)) | Err(TrySendError::Closed(share)) => {
            if let Err(e) = LOGGER_CHANNELS.backup.send(share) {
                info!("Failed to send share to backup logger: {}", e);
            }
        }
    }
}