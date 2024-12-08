use async_trait::async_trait;
use shares_logger::traits::ShareStorage;
use shares_logger::models::ShareLog;
use shares_logger::errors::ClickhouseError;
use shares_logger::traits::ShareData;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct MockStorageHighload {
    shares: Arc<Mutex<Vec<ShareLog>>>,
    store_delay_ms: u64,
}

impl MockStorageHighload {
    pub fn new(store_delay_ms: u64) -> Self {
        Self {
            shares: Arc::new(Mutex::new(Vec::new())),
            store_delay_ms,
        }
    }

    pub async fn get_stored_shares(&self) -> Vec<ShareLog> {
        self.shares.lock().await.clone()
    }
}

#[async_trait]
impl ShareStorage<ShareLog> for MockStorageHighload {
    async fn init(&self) -> Result<(), ClickhouseError> {
        Ok(())
    }

    async fn store_share(&mut self, share: ShareLog) -> Result<(), ClickhouseError> {
        tokio::time::sleep(tokio::time::Duration::from_millis(self.store_delay_ms)).await;
        self.shares.lock().await.push(share);
        Ok(())
    }

    async fn store_batch(&mut self, shares: Vec<ShareLog>) -> Result<(), ClickhouseError> {
        tokio::time::sleep(tokio::time::Duration::from_millis(self.store_delay_ms)).await;
        self.shares.lock().await.extend(shares);
        Ok(())
    }

    async fn flush(&mut self) -> Result<(), ClickhouseError> {
        Ok(())
    }
}