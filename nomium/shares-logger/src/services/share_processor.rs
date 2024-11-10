use crate::ShareLog;
use mining_sv2::Target;

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
        extranonce: Vec<u8>,  // Принимаем Vec<u8>
    ) -> ShareLog {
        let mut hash_bytes = hash;
        hash_bytes.reverse();

        ShareLog::new(
            channel_id,
            sequence_number,
            job_id,
            nonce,
            ntime, 
            version,
            hash_bytes.to_vec(),
            Target::from(hash) <= downstream_target,
            extranonce
        )
    }
}