mod common;

use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use shares_logger::traits::{ShareData, ShareStorage};
use shares_logger::errors::ClickhouseError;
use shares_logger::ShareLoggerBuilder;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::time::Duration;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

fn current_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StandardShare {
    miner_id: String,
    worker_name: String,
    nonce: u32,
    difficulty: f64,
    actual_difficulty: f64,
    timestamp: i64,
    is_valid: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExtendedShare {
    miner_id: String,
    worker_name: String,
    nonce: u32,
    difficulty: f64,
    actual_difficulty: f64,
    timestamp: i64,
    is_valid: bool,

    hashrate: f64,
    client_version: String,
    connection_duration: u64,
    pool_difficulty: f64,
    rejected_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DevShare {
    miner_id: String,
    worker_name: String,
    nonce: u32,
    difficulty: f64,
    actual_difficulty: f64,
    timestamp: i64,
    is_valid: bool,

    debug_info: HashMap<String, String>,
    test_flags: Vec<String>,
    raw_submit_data: String,
    processing_time_ms: u64,
}

#[async_trait]
impl ShareData for StandardShare {
    fn get_identifier(&self) -> String {
        format!("{}:{}:{}", self.miner_id, self.worker_name, self.timestamp)
    }

    async fn validate(&self) -> bool {
        self.difficulty > 0.0 && !self.miner_id.is_empty() && !self.worker_name.is_empty()
    }

    fn to_storage_format(&self) -> Vec<(String, String)> {
        vec![
            ("miner_id".to_string(), self.miner_id.clone()),
            ("worker_name".to_string(), self.worker_name.clone()),
            ("nonce".to_string(), self.nonce.to_string()),
            ("difficulty".to_string(), self.difficulty.to_string()),
            ("actual_difficulty".to_string(), self.actual_difficulty.to_string()),
            ("timestamp".to_string(), self.timestamp.to_string()),
            ("is_valid".to_string(), self.is_valid.to_string()),
        ]
    }
}

#[async_trait]
impl ShareData for ExtendedShare {
    fn get_identifier(&self) -> String {
        format!("{}:{}:{}", self.miner_id, self.worker_name, self.timestamp)
    }

    async fn validate(&self) -> bool {
        self.difficulty > 0.0 && !self.miner_id.is_empty() && 
        !self.worker_name.is_empty() && self.hashrate >= 0.0
    }

    fn to_storage_format(&self) -> Vec<(String, String)> {
        let mut result = vec![
            ("miner_id".to_string(), self.miner_id.clone()),
            ("worker_name".to_string(), self.worker_name.clone()),
            ("nonce".to_string(), self.nonce.to_string()),
            ("difficulty".to_string(), self.difficulty.to_string()),
            ("actual_difficulty".to_string(), self.actual_difficulty.to_string()),
            ("timestamp".to_string(), self.timestamp.to_string()),
            ("is_valid".to_string(), self.is_valid.to_string()),
            ("hashrate".to_string(), self.hashrate.to_string()),
            ("client_version".to_string(), self.client_version.clone()),
            ("connection_duration".to_string(), self.connection_duration.to_string()),
            ("pool_difficulty".to_string(), self.pool_difficulty.to_string()),
        ];
        
        if let Some(reason) = &self.rejected_reason {
            result.push(("rejected_reason".to_string(), reason.clone()));
        }
        
        result
    }
}

#[async_trait]
impl ShareData for DevShare {
    fn get_identifier(&self) -> String {
        format!("{}:{}:{}", self.miner_id, self.worker_name, self.timestamp)
    }

    async fn validate(&self) -> bool {
        self.difficulty > 0.0 && !self.miner_id.is_empty() && 
        !self.worker_name.is_empty() && self.processing_time_ms > 0
    }

    fn to_storage_format(&self) -> Vec<(String, String)> {
        let mut result = vec![
            ("miner_id".to_string(), self.miner_id.clone()),
            ("worker_name".to_string(), self.worker_name.clone()),
            ("nonce".to_string(), self.nonce.to_string()),
            ("difficulty".to_string(), self.difficulty.to_string()),
            ("actual_difficulty".to_string(), self.actual_difficulty.to_string()),
            ("timestamp".to_string(), self.timestamp.to_string()),
            ("is_valid".to_string(), self.is_valid.to_string()),
            ("raw_submit_data".to_string(), self.raw_submit_data.clone()),
            ("processing_time_ms".to_string(), self.processing_time_ms.to_string()),
        ];
        
        for (key, value) in &self.debug_info {
            result.push((format!("debug_{}", key), value.clone()));
        }
        
        for (i, flag) in self.test_flags.iter().enumerate() {
            result.push((format!("test_flag_{}", i), flag.clone()));
        }
        
        result
    }
}

#[derive(Debug, Clone)]
struct TestStorage<T: ShareData> {
    shares: Arc<Mutex<Vec<T>>>,
}

impl<T: ShareData> TestStorage<T> {
    fn new() -> Self {
        Self {
            shares: Arc::new(Mutex::new(Vec::new())),
        }
    }

    async fn get_stored_shares(&self) -> Vec<T> {
        self.shares.lock().await.clone()
    }
}

#[async_trait]
impl<T: ShareData + 'static> ShareStorage<T> for TestStorage<T> {
    async fn init(&self) -> Result<(), ClickhouseError> {
        Ok(())
    }

    async fn store_share(&mut self, share: T) -> Result<(), ClickhouseError> {
        if share.validate().await {
            self.shares.lock().await.push(share);
            Ok(())
        } else {
            Err(ClickhouseError::QueryError("Invalid share".to_string()))
        }
    }

    async fn store_batch(&mut self, shares: Vec<T>) -> Result<(), ClickhouseError> {
        for share in shares {
            self.store_share(share).await?;
        }
        Ok(())
    }

    async fn flush(&mut self) -> Result<(), ClickhouseError> {
        Ok(())
    }
}

#[tokio::test]
async fn test_share_types() {
    // cargo test test_share_types -- --nocapture

    let standard_storage = TestStorage::<StandardShare>::new();
    let standard_storage_clone = standard_storage.clone();
    let standard_logger = ShareLoggerBuilder::new(Box::new(standard_storage))
        .with_primary_channel_size(10)
        .with_backup_check_interval(Duration::from_secs(1))
        .build();

    let extended_storage = TestStorage::<ExtendedShare>::new();
    let extended_storage_clone = extended_storage.clone();
    let extended_logger = ShareLoggerBuilder::new(Box::new(extended_storage))
        .with_primary_channel_size(10)
        .with_backup_check_interval(Duration::from_secs(1))
        .build();

    let dev_storage = TestStorage::<DevShare>::new();
    let dev_storage_clone = dev_storage.clone();
    let dev_logger = ShareLoggerBuilder::new(Box::new(dev_storage))
        .with_primary_channel_size(10)
        .with_backup_check_interval(Duration::from_secs(1))
        .build();


    let standard_share = StandardShare {
        miner_id: "miner1".to_string(),
        worker_name: "worker1".to_string(),
        nonce: 12345,
        difficulty: 2.5,
        actual_difficulty: 2.7,
        timestamp: current_timestamp(),
        is_valid: true,
    };

    let extended_share = ExtendedShare {
        miner_id: "miner1".to_string(),
        worker_name: "worker1".to_string(),
        nonce: 12345,
        difficulty: 2.5,
        actual_difficulty: 2.7,
        timestamp: current_timestamp(),
        is_valid: true,
        hashrate: 1000.0,
        client_version: "ccminer/3.0.0".to_string(),
        connection_duration: 3600,
        pool_difficulty: 2.0,
        rejected_reason: None,
    };

    let mut debug_info = HashMap::new();
    debug_info.insert("submit_source".to_string(), "tcp_handler".to_string());
    debug_info.insert("validation_path".to_string(), "quick_check".to_string());

    let dev_share = DevShare {
        miner_id: "miner1".to_string(),
        worker_name: "worker1".to_string(),
        nonce: 12345,
        difficulty: 2.5,
        actual_difficulty: 2.7,
        timestamp: current_timestamp(),
        is_valid: true,
        debug_info,
        test_flags: vec!["test_mode".to_string(), "debug_validation".to_string()],
        raw_submit_data: "{nonce:12345,diff:2.5}".to_string(),
        processing_time_ms: 150,
    };

    standard_logger.log_share(standard_share.clone());
    extended_logger.log_share(extended_share.clone());
    dev_logger.log_share(dev_share.clone());

    tokio::time::sleep(Duration::from_secs(2)).await;

    let stored_standard_shares = standard_storage_clone.get_stored_shares().await;
    let stored_extended_shares = extended_storage_clone.get_stored_shares().await;
    let stored_dev_shares = dev_storage_clone.get_stored_shares().await;

    assert_eq!(stored_standard_shares.len(), 1);
    assert_eq!(stored_extended_shares.len(), 1);
    assert_eq!(stored_dev_shares.len(), 1);

    let stored_standard = &stored_standard_shares[0];
    assert_eq!(stored_standard.miner_id, "miner1");
    assert_eq!(stored_standard.worker_name, "worker1");
    assert_eq!(stored_standard.difficulty, 2.5);

    let stored_extended = &stored_extended_shares[0];
    assert_eq!(stored_extended.miner_id, "miner1");
    assert_eq!(stored_extended.hashrate, 1000.0);
    assert_eq!(stored_extended.client_version, "ccminer/3.0.0");

    let stored_dev = &stored_dev_shares[0];
    assert_eq!(stored_dev.miner_id, "miner1");
    assert_eq!(stored_dev.processing_time_ms, 150);
    assert!(stored_dev.debug_info.contains_key("submit_source"));

    let standard_format = stored_standard.to_storage_format();
    let extended_format = stored_extended.to_storage_format();
    let dev_format = stored_dev.to_storage_format();

    assert!(standard_format.iter().any(|(k, v)| k == "miner_id" && v == "miner1"));
    assert!(extended_format.iter().any(|(k, v)| k == "hashrate" && v == "1000"));
    assert!(dev_format.iter().any(|(k, v)| k == "processing_time_ms" && v == "150"));
}