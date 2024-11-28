use serde::{Serialize, Deserialize};
use crate::traits::ShareData;
use async_trait::async_trait;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareLog {
    pub channel_id: u32,
    pub sequence_number: u32,
    pub job_id: u32,
    pub nonce: u32, 
    pub ntime: u32,
    pub version: u32,
    pub hash: Vec<u8>,
    pub share_status: ShareStatus,
    pub extranonce: Vec<u8>,
    pub difficulty: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ShareStatus {
    Invalid = 0,
    NetworkValid = 1,
    PoolValid = 2,
    MinerValid = 3,
}

impl ShareLog {
    pub fn new(
        channel_id: u32,
        sequence_number: u32,
        job_id: u32,
        nonce: u32,
        ntime: u32,
        version: u32,
        hash: Vec<u8>,
        share_status: ShareStatus,
        extranonce: Vec<u8>,
        difficulty: f64,
    ) -> Self {
        Self {
            channel_id,
            sequence_number,
            job_id,
            nonce,
            ntime,
            version,
            hash,
            share_status,
            extranonce,
            difficulty
        }
    }
}

#[async_trait]
impl ShareData for ShareLog {
    fn get_identifier(&self) -> String {
        format!("{}_{}", self.channel_id, self.sequence_number)
    }

    async fn validate(&self) -> bool {
        true
    }

    fn to_storage_format(&self) -> Vec<(String, String)> {
        vec![
            ("channel_id".to_string(), self.channel_id.to_string()),
            ("sequence_number".to_string(), self.sequence_number.to_string()),
            ("job_id".to_string(), self.job_id.to_string()),
            ("nonce".to_string(), self.nonce.to_string()),
            ("ntime".to_string(), self.ntime.to_string()),
            ("version".to_string(), self.version.to_string()),
            ("hash".to_string(), hex::encode(&self.hash)),
            ("share_status".to_string(), (self.share_status as u8).to_string()),
            ("extranonce".to_string(), hex::encode(&self.extranonce)),
            ("difficulty".to_string(), self.difficulty.to_string()),
        ]
    }

    fn is_block_found(&self) -> bool { false }
}