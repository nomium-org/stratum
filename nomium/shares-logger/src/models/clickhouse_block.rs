use crate::models::BlockFound;
use clickhouse::Row;
use serde::Serialize;

#[derive(Row, Serialize, Debug)]
pub struct ClickhouseBlock {
    pub channel_id: u32,
    pub block_hash: String,
    pub ntime: u32,
    pub worker_id: String,
    pub account_name: String,
    found_at: i64,
}

impl From<BlockFound> for ClickhouseBlock {
    
    fn from(block: BlockFound) -> Self {

        Self {
            channel_id: block.channel_id,
            block_hash: hex::encode(&block.block_hash),
            ntime: block.ntime,
            worker_id: block.worker_id,
            account_name: block.account_name,
            found_at: block.found_at.timestamp_millis(),
        }
    }
}