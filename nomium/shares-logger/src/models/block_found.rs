use serde::{Serialize, Deserialize};
use crate::traits::ShareData;
use async_trait::async_trait;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockFound {
    pub channel_id: u32,
    pub block_hash: Vec<u8>,
    pub timestamp: u64,
}

#[async_trait]
impl ShareData for BlockFound {
    fn get_identifier(&self) -> String {
        format!("{}_{}_{}", self.channel_id, hex::encode(&self.block_hash), self.timestamp)
    }

    async fn validate(&self) -> bool {
        self.block_hash.len() == 32
    }

    fn to_storage_format(&self) -> Vec<(String, String)> {
        vec![
            ("channel_id".to_string(), self.channel_id.to_string()),
            ("block_hash".to_string(), hex::encode(&self.block_hash)),
            ("timestamp".to_string(), self.timestamp.to_string()),
        ]
    }

    fn is_block_found(&self) -> bool { true }
}