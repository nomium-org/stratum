use serde::{Serialize, Deserialize};
use crate::traits::ShareData;
use async_trait::async_trait;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockFound {
    pub channel_id: u32,
    pub block_hash: Vec<u8>,
    pub ntime: u32,
    pub user_identity: String,
}

#[async_trait]
impl ShareData for BlockFound {
    fn get_identifier(&self) -> String {
        format!("{}_{}_{}", self.user_identity, hex::encode(&self.block_hash), self.ntime)
    }

    async fn validate(&self) -> bool {
        self.block_hash.len() == 32
    }

    fn to_storage_format(&self) -> Vec<(String, String)> {
        vec![
            ("channel_id".to_string(), self.channel_id.to_string()),
            ("block_hash".to_string(), hex::encode(&self.block_hash)),
            ("ntime".to_string(), self.ntime.to_string()),
            ("user_identity".to_string(), self.user_identity.clone()),
        ]
    }

    fn is_block_found(&self) -> bool { true }
}