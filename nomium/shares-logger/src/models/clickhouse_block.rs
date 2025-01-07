use crate::models::BlockFound;
use clickhouse::Row;
use serde::Serialize;

#[derive(Row, Serialize, Debug)]
pub struct ClickhouseBlock {
    pub channel_id: u32,
    pub block_hash: String,
    pub ntime: u32,
    pub user_identity: String,
    pub worker_id: String,
    pub account_name: String,
}

impl From<BlockFound> for ClickhouseBlock {
    fn from(block: BlockFound) -> Self {
        Self {
            channel_id: block.channel_id,
            block_hash: hex::encode(&block.block_hash),
            ntime: block.ntime,
            user_identity: block.user_identity,
            worker_id: block.worker_id,
            account_name: block.account_name,
        }
    }
}