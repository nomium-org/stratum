use crate::models::ShareLog;
use clickhouse::Row;
use serde::Serialize;

#[derive(Row, Serialize)]
pub struct ClickhouseShare {
    pub channel_id: u32,
    pub sequence_number: u32,
    pub job_id: u32,
    pub nonce: u32,
    pub ntime: u32,
    pub version: u32,
    pub hash: String,
    pub share_status: u8,
    pub extranonce: String,
    pub difficulty: f64,
    pub user_identity: String,
}

impl From<ShareLog> for ClickhouseShare {
    fn from(share: ShareLog) -> Self {
        // Гарантируем корректное hex-кодирование
        let hash_hex = share.hash.iter()
            .fold(String::with_capacity(share.hash.len() * 2), |mut acc, &b| {
                acc.push_str(&format!("{:02x}", b));
                acc
            });
        
        let extranonce_hex = share.extranonce.iter()
            .fold(String::with_capacity(share.extranonce.len() * 2), |mut acc, &b| {
                acc.push_str(&format!("{:02x}", b));
                acc
            });

        Self {
            channel_id: share.channel_id,
            sequence_number: share.sequence_number,
            job_id: share.job_id,
            nonce: share.nonce,
            ntime: share.ntime,
            version: share.version,
            hash: hash_hex,
            share_status: share.share_status as u8,
            extranonce: extranonce_hex,
            difficulty: share.difficulty,
            user_identity: share.user_identity,
        }
    }
}