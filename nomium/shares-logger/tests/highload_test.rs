mod common;

use common::mock_storage::MockStorageHighload;
use shares_logger::{ShareLoggerBuilder, models::ShareLog, models::ShareStatus};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn test_highload_share_processing() {
    let mock_storage = MockStorageHighload::new(5);
    let storage_for_verification = mock_storage.clone();

    let logger = ShareLoggerBuilder::new(Box::new(mock_storage))
        .with_primary_channel_size(10)
        .with_backup_check_interval(Duration::from_secs(1))
        .build();

    let total_shares = 100;
    let test_shares: Vec<ShareLog> = (0..total_shares)
        .map(|i| ShareLog::new(
            1,
            i as u32,
            1,
            i as u32,
            0,
            1,
            vec![0; 32],
            ShareStatus::NetworkValid,
            vec![0; 8],
            1.0,
        ))
        .collect();

    for share in test_shares {
        logger.log_share(share);
        sleep(Duration::from_millis(10)).await;
    }

    sleep(Duration::from_secs(5)).await;

    let stored_shares = storage_for_verification.get_stored_shares().await;
    assert_eq!(stored_shares.len(), total_shares as usize, 
        "Expected {} shares, but got {}", total_shares, stored_shares.len());

    let mut sequence_numbers: Vec<u32> = stored_shares.iter()
        .map(|share| share.sequence_number)
        .collect();
    sequence_numbers.sort();

    for (i, &seq) in sequence_numbers.iter().enumerate() {
        assert_eq!(seq, i as u32, 
            "Missing or incorrect sequence number. Expected {}, got {}", i, seq);
    }
}