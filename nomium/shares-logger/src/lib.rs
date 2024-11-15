pub mod config;
pub mod services;

use log::info;
use once_cell::sync::Lazy;
use services::clickhouse::ClickhouseService;
use tokio::sync::mpsc::{self, error::TrySendError};
use std::time::Duration;
use crate::config::CONFIG;

#[derive(Debug, Clone)]
pub struct ShareLog {
    pub channel_id: u32,
    pub sequence_number: u32,
    pub job_id: u32,
    pub nonce: u32, 
    pub ntime: u32,
    pub version: u32,
    pub hash: Vec<u8>,
    pub is_valid: bool,
    pub extranonce: Vec<u8>,
    pub difficulty: f64,
}

impl ShareLog {
    pub fn new(
        channel_id: u32,
        sequence_number: u32,
        job_id: u32,
        nonce: u32,
        ntime: u32,
        version: u32,
        hash: Vec<u8>,
        is_valid: bool,
        extranonce: Vec<u8>,
        difficulty: f64,
    ) -> Self {
        Self {
            channel_id,
            sequence_number,
            job_id,
            nonce,
            ntime,
            version,
            hash,
            is_valid,
            extranonce,
            difficulty
        }
    }
}

struct LogChannels {
    primary: mpsc::Sender<ShareLog>,
    backup: mpsc::UnboundedSender<ShareLog>,
}

static LOGGER_CHANNELS: Lazy<LogChannels> = Lazy::new(|| {
    let (primary_tx, primary_rx) = mpsc::channel(CONFIG.primary_channel_buffer_size);
    let (backup_tx, backup_rx) = mpsc::unbounded_channel();
    
    let clickhouse_service = ClickhouseService::new();
    
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
    let mut backup_interval = tokio::time::interval(Duration::from_secs(CONFIG.backup_check_interval_secs));
    
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

    //services::debug_log::log_share_hash("incoming_share", &share);

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