use log::info;
use once_cell::sync::Lazy;
use tokio::sync::mpsc::{self, error::TrySendError};
use std::time::Duration;

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
    pub extranonce: Vec<u8>
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
        extranonce: Vec<u8>
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
            extranonce
        }
    }
}

struct LogChannels {
    primary: mpsc::Sender<ShareLog>,
    backup: mpsc::UnboundedSender<ShareLog>,
}

static LOGGER_CHANNELS: Lazy<LogChannels> = Lazy::new(|| {
    let (primary_tx, primary_rx) = mpsc::channel(100); // Bounded primary channel
    let (backup_tx, backup_rx) = mpsc::unbounded_channel(); // Unbounded backup channel

    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async move {
            process_logs(primary_rx, backup_rx).await;
        });
    });

    LogChannels {
        primary: primary_tx,
        backup: backup_tx,
    }
});

async fn process_logs(
    mut primary_rx: mpsc::Receiver<ShareLog>, 
    mut backup_rx: mpsc::UnboundedReceiver<ShareLog>
) {
    let mut backup_interval = tokio::time::interval(Duration::from_secs(1));
    
    loop {
        tokio::select! {
            Some(share) = primary_rx.recv() => {
                info!("Share details (primary): {:?}", share);
            }
            _ = backup_interval.tick() => {
                while let Ok(share) = backup_rx.try_recv() {
                    info!("Share details (backup): {:?}", share);
                }
            }
        }
    }
}

pub fn hand_shake() {
    info!("!!! SHARES-LOGGER !!!");
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