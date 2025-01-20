use crate::models::BlockFound;
use clickhouse::Row;
use serde::Serialize;
use chrono::{DateTime, Utc, SubsecRound};
use serde_json::json;

#[derive(Row, Serialize, Debug)]
pub struct ClickhouseBlock {
    pub channel_id: u32,
    pub block_hash: String,
    pub ntime: u32,
    pub worker_id: String,
    pub found_at: DateTime<Utc>,
    pub account_name: String,
}

impl From<BlockFound> for ClickhouseBlock {
    fn from(block: BlockFound) -> Self {
        Self {
            channel_id: block.channel_id,
            block_hash: hex::encode(&block.block_hash),
            ntime: block.ntime,
            worker_id: block.worker_id,
            found_at: block.found_at.trunc_subsecs(3),
            account_name: block.account_name,
        }
    }
}

impl ClickhouseBlock {
    pub fn to_clickhouse_json(&self) -> String {
        let formatted_timestamp = self.found_at.format("%Y-%m-%d %H:%M:%S.%3f").to_string();
        json!({
            "account_name": self.account_name,
            "worker_id": self.worker_id,
            "channel_id": self.channel_id,
            "block_hash": self.block_hash,
            "ntime": self.ntime,
            "found_at": formatted_timestamp,
            "is_rewards_calculated": false
        }).to_string()
    }
}