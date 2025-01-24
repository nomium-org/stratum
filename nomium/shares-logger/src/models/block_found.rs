use serde::{Serialize, Deserialize};
use serde_json::Value;
use serde_json::json;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockFound {
    pub channel_id: u32,
    pub block_hash: Vec<u8>,
    pub ntime: u32,
    pub worker_id: String,
    pub account_name: String,
    pub found_at: i64,
}

impl BlockFound {
    pub fn prepare_block(
        channel_id: u32,
        mut block_hash: Vec<u8>,
        ntime: u32,
        user_identity_json: String,
        found_at: i64,
    ) -> Self {

        block_hash.reverse();
        
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

        let account_name = user_identity.split('.').next().unwrap_or_default().to_string();

        BlockFound {
            channel_id,
            block_hash,
            ntime,
            worker_id,
            account_name,
            found_at,
        }
    }
}