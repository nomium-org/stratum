use crate::models::ShareLog;
use clickhouse::Row;
use serde::Serialize;
use chrono::{DateTime, Utc, SubsecRound};
use serde_json::json;

#[derive(Row, Serialize)]
pub struct ClickhouseShare {
    pub account_name: String,
    pub worker_id: String,
    pub sequence_number: u32,
    pub job_id: u32,
    pub nonce: u32,
    pub ntime: u32,
    pub version: u32,
    pub hash: String,
    pub share_status: u8,
    pub extranonce: String,
    pub difficulty: f64,
    pub channel_id: u32,
    pub is_pps_reward_calculated: bool,
    pub timestamp: DateTime<Utc>,
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

        let account_name = share.user_identity.split('.').next().unwrap_or_default().to_string();

        let timestamp = share.timestamp.trunc_subsecs(3);    

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
            worker_id: share.worker_id,
            account_name: account_name,
            is_pps_reward_calculated: false,
            timestamp: timestamp,
        }
    }
}

impl ClickhouseShare {
    pub fn to_clickhouse_json(&self) -> String {

        let formatted_timestamp = self.timestamp.format("%Y-%m-%d %H:%M:%S.%3f").to_string();
        
        json!({
            "account_name": self.account_name,
            "worker_id": self.worker_id,
            "sequence_number": self.sequence_number,
            "job_id": self.job_id,
            "nonce": self.nonce,
            "ntime": self.ntime,
            "version": self.version,
            "hash": self.hash,
            "share_status": self.share_status,
            "extranonce": self.extranonce,
            "difficulty": self.difficulty,
            "channel_id": self.channel_id,
            "is_pps_reward_calculated": self.is_pps_reward_calculated,
            "timestamp": formatted_timestamp
        }).to_string()
    }
}