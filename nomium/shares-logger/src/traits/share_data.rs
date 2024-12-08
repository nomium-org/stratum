use async_trait::async_trait;
use serde::{Serialize, de::DeserializeOwned};

#[async_trait]
pub trait ShareData: Send + Sync + Clone + Serialize + DeserializeOwned {
    fn get_identifier(&self) -> String;
    async fn validate(&self) -> bool;
    fn to_storage_format(&self) -> Vec<(String, String)>;
    fn is_block_found(&self) -> bool;
}