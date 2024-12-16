use crate::models::ShareLog;
use mining_sv2::Target;
use super::difficulty::DifficultyService;
use crate::models::ShareStatus;
use log::info;

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
        user_identity: String,
    ) -> ShareLog {
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
        )
    }
}