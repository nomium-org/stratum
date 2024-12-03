use crate::models::BlockFound;
use clickhouse::Row;
use serde::Serialize;

#[derive(Row, Serialize)]
pub struct ClickhouseBlock {
    pub channel_id: u32,
    pub block_hash: String,
    pub ntime: u32,
}

impl From<BlockFound> for ClickhouseBlock {
    fn from(block: BlockFound) -> Self {
        Self {
            channel_id: block.channel_id,
            block_hash: hex::encode(&block.block_hash),
            ntime: block.ntime,
        }
    }
}