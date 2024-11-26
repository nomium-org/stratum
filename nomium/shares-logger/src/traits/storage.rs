use async_trait::async_trait;
use crate::errors::ClickhouseError;
use super::share_data::ShareData;
use crate::models::BlockFound;

#[async_trait]
pub trait ShareStorage<T: ShareData>: Send + Sync {
    async fn init(&self) -> Result<(), ClickhouseError>;
    async fn store_share(&mut self, share: T) -> Result<(), ClickhouseError>;
    async fn store_batch(&mut self, shares: Vec<T>) -> Result<(), ClickhouseError>;
    async fn flush(&mut self) -> Result<(), ClickhouseError>;
}