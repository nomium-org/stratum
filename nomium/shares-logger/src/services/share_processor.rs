use crate::models::ShareLog;
use mining_sv2::Target;
use super::difficulty::DifficultyService;
use crate::models::ShareStatus;
use log::info;
use serde_json::Value;
use serde_json::json;
use chrono::{DateTime, Utc};

pub struct ShareProcessor;

impl ShareProcessor {
    pub fn prepare_share_log(
        channel_id: u32,
        sequence_number: u32,
        job_id: u32,
        nonce: u32,
        ntime: u32,
        version: u32,
        hash: [u8; 32],
        downstream_target: Target,
        extranonce: Vec<u8>,
        user_identity_json: String,
        timestamp: DateTime<Utc>,
    ) -> ShareLog {
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

        info!("user_identity from prepare_share_log: {}", user_identity);
        let mut hash_bytes = hash;
        hash_bytes.reverse();
        let difficulty = DifficultyService::calculate_difficulty_from_hash(&hash_bytes);

        let status = ShareStatus::NetworkValid;

        ShareLog::new(
            channel_id,
            sequence_number,
            job_id,
            nonce,
            ntime, 
            version,
            hash_bytes.to_vec(),
            status,
            extranonce,
            difficulty,
            user_identity,
            worker_id,
            timestamp,
        )
    }
}