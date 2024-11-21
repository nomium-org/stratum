use async_trait::async_trait;
use crate::traits::ShareStorage;
use crate::models::ShareLog;
use crate::errors::ClickhouseError;

pub struct MockStorage {
    shares: Vec<ShareLog>,
}

impl MockStorage {
    pub fn new() -> Self {
        Self {
            shares: Vec::new(),
        }
    }
}

#[async_trait]
impl ShareStorage for MockStorage {
    async fn init(&self) -> Result<(), ClickhouseError> {
        Ok(())
    }

    async fn store_share(&mut self, share: ShareLog) -> Result<(), ClickhouseError> {
        self.shares.push(share);
        Ok(())
    }

    async fn store_batch(&mut self, shares: Vec<ShareLog>) -> Result<(), ClickhouseError> {
        self.shares.extend(shares);
        Ok(())
    }

    async fn flush(&mut self) -> Result<(), ClickhouseError> {
        Ok(())
    }
}