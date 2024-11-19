pub mod config;
pub mod errors;
pub mod models;
pub mod services;

use crate::config::SETTINGS;
use crate::models::ShareLog;
use log::info;
use once_cell::sync::Lazy;
use services::clickhouse::ClickhouseService;
use std::time::Duration;
use tokio::sync::mpsc::{self, error::TrySendError};

struct LogChannels {
    primary: mpsc::Sender<ShareLog>,
    backup: mpsc::UnboundedSender<ShareLog>,
}

static LOGGER_CHANNELS: Lazy<LogChannels> = Lazy::new(|| {
    let (primary_tx, primary_rx) = mpsc::channel(SETTINGS.processing.primary_channel_buffer_size);
    let (backup_tx, backup_rx) = mpsc::unbounded_channel();

    let clickhouse_service =
        ClickhouseService::new().expect("Failed to initialize ClickhouseService");

    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        rt.block_on(async move {
            process_shares(primary_rx, backup_rx, clickhouse_service).await;
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
    mut clickhouse_service: ClickhouseService,
) {
    let mut backup_interval = tokio::time::interval(Duration::from_secs(
        SETTINGS.processing.backup_check_interval_secs,
    ));

    loop {
        tokio::select! {
            Some(share) = primary_rx.recv() => {
                info!("Processing share from primary channel: {:?}", share);
                if let Err(e) = clickhouse_service.process_share(share).await {
                    info!("Failed to process share in ClickHouse: {}", e);
                }
            }
            _ = backup_interval.tick() => {
                while let Ok(share) = backup_rx.try_recv() {
                    info!("Processing share from backup channel: {:?}", share);
                    if let Err(e) = clickhouse_service.process_share(share).await {
                        info!("Failed to process share from backup in ClickHouse: {}", e);
                    }
                }
            }
        }
    }
}

pub fn log_share(share: ShareLog) {
    match LOGGER_CHANNELS.primary.try_send(share.clone()) {
        Ok(_) => (),
        Err(TrySendError::Full(share)) => {
            if let Err(e) = LOGGER_CHANNELS.backup.send(share) {
                info!("Failed to send share to backup logger: {}", e);
            }
        }
        Err(TrySendError::Closed(share)) => {
            if let Err(e) = LOGGER_CHANNELS.backup.send(share) {
                info!("Failed to send share to backup logger: {}", e);
            }
        }
    }
}
