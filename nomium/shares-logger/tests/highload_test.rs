mod common;

use common::mock_storage::MockStorageHighload;
use shares_logger::{ShareLoggerBuilder, models::ShareLog, models::ShareStatus};
use std::time::{Duration, Instant};
use tokio::time::sleep;

#[tokio::test]
async fn test_highload_share_processing() {
    println!("\n=== Starting highload test ===");
    let start_time = Instant::now();
    
    let mock_storage = MockStorageHighload::new(5);
    let storage_for_verification = mock_storage.clone();

    let logger = ShareLoggerBuilder::new(Box::new(mock_storage))
        .with_primary_channel_size(10)
        .with_backup_check_interval(Duration::from_secs(1))
        .build();

    let total_shares = 100;
    println!("Generating {} test shares...", total_shares);
    
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

    println!("Starting share submission...");
    let submit_start = Instant::now();
    
    for (i, share) in test_shares.into_iter().enumerate() {
        logger.log_share(share);
        sleep(Duration::from_millis(10)).await;
        
        if (i + 1) % 20 == 0 {
            println!("Submitted {} shares...", i + 1);
        }
    }

    let submit_duration = submit_start.elapsed();
    println!("All shares submitted in {:.2?}", submit_duration);
    println!("Waiting for processing completion...");

    sleep(Duration::from_secs(5)).await;

    let stored_shares = storage_for_verification.get_stored_shares().await;
    let total_duration = start_time.elapsed();
    
    println!("\n=== Test Results ===");
    println!("Total shares submitted: {}", total_shares);
    println!("Total shares stored: {}", stored_shares.len());
    println!("Processing rate: {:.2} shares/second", 
        stored_shares.len() as f64 / total_duration.as_secs_f64());
    
    let mut sequence_numbers: Vec<u32> = stored_shares.iter()
        .map(|share| share.sequence_number)
        .collect();
    sequence_numbers.sort();

    let mut gaps = Vec::new();
    for i in 0..total_shares {
        if !sequence_numbers.contains(&(i as u32)) {
            gaps.push(i);
        }
    }

    println!("\n=== Sequence Analysis ===");
    if gaps.is_empty() {
        println!("✅ All sequence numbers present and accounted for");
    } else {
        println!("❌ Missing sequence numbers: {:?}", gaps);
    }

    assert_eq!(stored_shares.len(), total_shares as usize,
        "Expected {} shares, but got {}", total_shares, stored_shares.len());

    for (i, &seq) in sequence_numbers.iter().enumerate() {
        assert_eq!(seq, i as u32,
            "Sequence mismatch at position {}. Expected {}, got {}", i, i, seq);
    }

    println!("\n=== Test completed in {:.2?} ===", total_duration);
}