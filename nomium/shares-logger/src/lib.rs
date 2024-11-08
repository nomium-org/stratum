use log::info;
use once_cell::sync::Lazy;
use tokio::sync::mpsc::{self, error::TrySendError};
use std::time::Duration;

struct LogChannels {
    primary: mpsc::Sender<Vec<u8>>,
    backup: mpsc::UnboundedSender<Vec<u8>>,
}

static LOGGER_CHANNELS: Lazy<LogChannels> = Lazy::new(|| {
    let (primary_tx, primary_rx) = mpsc::channel(100); // Bounded primary channel
    let (backup_tx, backup_rx) = mpsc::unbounded_channel(); // Unbounded backup channel
    
    // Spawn background task to process logs
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
    mut primary_rx: mpsc::Receiver<Vec<u8>>, 
    mut backup_rx: mpsc::UnboundedReceiver<Vec<u8>>
) {
    let mut backup_interval = tokio::time::interval(Duration::from_secs(1));

    loop {
        tokio::select! {
            // Приоритетная обработка основного канала
            Some(hash) = primary_rx.recv() => {
                info!("Share hash (primary): {:?}", hex::encode(hash));
            }
            
            // Проверка резервного канала с интервалом
            _ = backup_interval.tick() => {
                while let Ok(hash) = backup_rx.try_recv() {
                    info!("Share hash (backup): {:?}", hex::encode(hash));
                }
            }
        }
    }
}

pub fn hand_shake() {
    info!("!!! SHARES-LOGGER !!!");
}

pub fn log_share(hash: Vec<u8>) {
    match LOGGER_CHANNELS.primary.try_send(hash.clone()) {
        Ok(_) => (),
        Err(TrySendError::Full(hash)) => {
            // Если основной канал переполнен, отправляем в резервный
            if let Err(e) = LOGGER_CHANNELS.backup.send(hash) {
                info!("Failed to send hash to backup logger: {}", e);
            }
        }
        Err(TrySendError::Closed(hash)) => {
            // Если основной канал закрыт, пытаемся использовать резервный
            if let Err(e) = LOGGER_CHANNELS.backup.send(hash) {
                info!("Failed to send hash to backup logger: {}", e);
            }
        }
    }
}