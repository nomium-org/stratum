use serde::{Serialize, Deserialize};
use crate::traits::ShareData;
use async_trait::async_trait;
use serde_json::Value;
use serde_json::json;
use log::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockFound {
    pub channel_id: u32,
    pub block_hash: Vec<u8>,
    pub ntime: u32,
    pub user_identity: String,
    pub worker_id: String,
}

impl BlockFound {
    pub fn prepare_block(
        channel_id: u32,
        block_hash: Vec<u8>,
        ntime: u32,
        user_identity_json: String,
    ) -> Self {
        info!("Preparing block with user_identity_json: {}", user_identity_json);
        
        let worker_identity: Value = serde_json::from_str(&user_identity_json)
            .unwrap_or_else(|_| json!({
                "worker_name": user_identity_json.clone(),
                "worker_id": "unknown"
            }));

        let user_identity = worker_identity["worker_name"]
            .as_str()
            .unwrap_or(&user_identity_json)
            .to_string();

        let worker_id = worker_identity["worker_id"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();

        info!("Block prepared with worker_id: {}", worker_id);

        BlockFound {
            channel_id,
            block_hash,
            ntime,
            user_identity,
            worker_id,
        }
    }
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
            ("worker_id".to_string(), self.worker_id.clone()),
        ]
    }

    fn is_block_found(&self) -> bool { 
        true 
    }
}