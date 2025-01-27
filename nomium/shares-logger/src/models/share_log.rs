use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareLog {
    pub channel_id: u32,
    pub sequence_number: u32,
    pub job_id: u32,
    pub nonce: u32, 
    pub time_from_worker: u32,
    pub version: u32,
    pub hash: Vec<u8>,
    pub share_status: ShareStatus,
    pub extranonce: Vec<u8>,
    pub difficulty: f64,
    pub user_identity: String,
    pub worker_id: String,
    pub received_at: i64,
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
        time_from_worker: u32,
        version: u32,
        hash: Vec<u8>,
        share_status: ShareStatus,
        extranonce: Vec<u8>,
        difficulty: f64,
        user_identity: String,
        worker_id: String,
        received_at: i64,
    ) -> Self {
        Self {
            channel_id,
            sequence_number,
            job_id,
            nonce,
            time_from_worker,
            version,
            hash,
            share_status,
            extranonce,
            difficulty,
            user_identity,
            worker_id,
            received_at,
        }
    }
}