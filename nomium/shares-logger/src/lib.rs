use log::info;
use once_cell::sync::Lazy;
use tokio::sync::mpsc;

static LOGGER_SENDER: Lazy<mpsc::Sender<Vec<u8>>> = Lazy::new(|| {
    let (tx, rx) = mpsc::channel(100); // Bounded channel with capacity 100
    
    // Spawn background task to process logs
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            process_logs(rx).await;
        });
    });
    
    tx
});

async fn process_logs(mut rx: mpsc::Receiver<Vec<u8>>) {
    while let Some(hash) = rx.recv().await {
        info!("Share hash: {:?}", hex::encode(hash));
    }
}

pub fn hand_shake() {
    info!("!!! SHARES-LOGGER !!!");
}

pub fn log_share(hash: Vec<u8>) {
    if let Err(e) = LOGGER_SENDER.try_send(hash) {
        info!("Failed to send hash to logger: {}", e);
    }
}