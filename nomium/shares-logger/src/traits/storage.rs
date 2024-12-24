use async_trait::async_trait;
use crate::errors::ClickhouseError;
use serde::Serialize;
use serde::de::DeserializeOwned;

#[async_trait]
pub trait ShareStorage<T: Send + Sync + Clone + Serialize + DeserializeOwned>: Send {
    async fn init(&self) -> Result<(), ClickhouseError>;
    async fn store_share(&mut self, share: T) -> Result<(), ClickhouseError>;
    async fn store_batch(&mut self, shares: Vec<T>) -> Result<(), ClickhouseError>;
    async fn flush(&mut self) -> Result<(), ClickhouseError>;
}