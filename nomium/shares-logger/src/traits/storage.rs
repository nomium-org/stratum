use async_trait::async_trait;
use crate::models::ShareLog;
use crate::errors::ClickhouseError;

#[async_trait]
pub trait ShareStorage: Send + Sync {
    async fn init(&self) -> Result<(), ClickhouseError>;
    async fn store_share(&mut self, share: ShareLog) -> Result<(), ClickhouseError>;
    async fn store_batch(&mut self, shares: Vec<ShareLog>) -> Result<(), ClickhouseError>;
    async fn flush(&mut self) -> Result<(), ClickhouseError>;
}