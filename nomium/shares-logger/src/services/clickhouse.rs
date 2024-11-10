use crate::ShareLog;
use crate::config::CONFIG;
use clickhouse::{Client, Row};
use log::{error, info};
use std::time::Duration;
use tokio::sync::mpsc::{self, Receiver};
use serde::Serialize;

#[derive(Row, Serialize)]
struct ClickhouseShare {
    pub channel_id: u32,
    pub sequence_number: u32,
    pub job_id: u32,
    pub nonce: u32,
    pub ntime: u32,
    pub version: u32,
    pub hash: String,
    pub is_valid: u8,
    pub extranonce: String,
    pub difficulty: f64,
}

impl From<ShareLog> for ClickhouseShare {
    fn from(share: ShareLog) -> Self {
        Self {
            channel_id: share.channel_id,
            sequence_number: share.sequence_number,
            job_id: share.job_id,
            nonce: share.nonce,
            ntime: share.ntime,
            version: share.version,
            hash: hex::encode(share.hash),
            is_valid: if share.is_valid { 1 } else { 0 },
            extranonce: hex::encode(share.extranonce),
            difficulty: share.difficulty,
        }
    }
}

pub struct ClickhouseService {
    client: Client,
    batch_receiver: Receiver<ShareLog>,
}

impl ClickhouseService {
    pub fn new(batch_receiver: Receiver<ShareLog>) -> Self {
        info!("Initializing ClickhouseService with config: url={}, database={}, user={}", 
            CONFIG.url, CONFIG.database, CONFIG.username);
        
        let client = Client::default()
            .with_url(&CONFIG.url)
            .with_database(&CONFIG.database)
            .with_user(&CONFIG.username)
            .with_password(&CONFIG.password);
        
        Self {
            client,
            batch_receiver,
        }
    }

    pub async fn run(&mut self) {
        info!("ClickhouseService started running");
        let mut batch = Vec::with_capacity(CONFIG.batch_size);
        let mut interval = tokio::time::interval(Duration::from_secs(5));

        // Попробуем создать таблицу при старте сервиса
        if let Err(e) = self.ensure_table_exists().await {
            error!("Failed to ensure table exists: {}", e);
            return;
        }

        loop {
            tokio::select! {
                Some(share) = self.batch_receiver.recv() => {
                    batch.push(share);
                    info!("Received share, batch size: {}/{}", batch.len(), CONFIG.batch_size);
                    
                    if batch.len() >= CONFIG.batch_size {
                        info!("Batch size reached, attempting to insert batch");
                        match self.insert_batch(&batch).await {
                            Ok(_) => info!("Successfully inserted batch of {} shares", batch.len()),
                            Err(e) => error!("Failed to insert batch: {}", e),
                        }
                        batch.clear();
                    }
                }
                _ = interval.tick() => {
                    if !batch.is_empty() {
                        info!("Timer triggered, inserting partial batch of {} shares", batch.len());
                        match self.insert_batch(&batch).await {
                            Ok(_) => info!("Successfully inserted partial batch of {} shares", batch.len()),
                            Err(e) => error!("Failed to insert partial batch: {}", e),
                        }
                        batch.clear();
                    }
                }
            }
        }
    }

    async fn ensure_table_exists(&self) -> Result<(), clickhouse::error::Error> {
        info!("Ensuring shares table exists");
        let query = r#"
            CREATE TABLE IF NOT EXISTS shares (
                channel_id UInt32,
                sequence_number UInt32,
                job_id UInt32,
                nonce UInt32,
                ntime UInt32,
                version UInt32,
                hash String,
                is_valid UInt8,
                extranonce String,
                difficulty Float64,
                timestamp DateTime DEFAULT now()
            ) ENGINE = MergeTree()
            ORDER BY (timestamp, channel_id)
        "#;

        self.client.query(query).execute().await?;
        info!("Shares table created or already exists");
        Ok(())
    }

    async fn insert_batch(&self, batch: &[ShareLog]) -> Result<(), clickhouse::error::Error> {
        info!("Starting batch insert of {} shares", batch.len());
        
        let mut insert = self.client.insert("shares")?;
        
        for share in batch {
            let clickhouse_share = ClickhouseShare::from(share.clone());
            insert.write(&clickhouse_share).await?;
        }
        
        insert.end().await?;
        
        info!("Successfully committed batch to Clickhouse");
        Ok(())
    }
}

pub fn create_clickhouse_service() -> (mpsc::Sender<ShareLog>, ClickhouseService) {
    info!("Creating new ClickhouseService instance");
    let (tx, rx) = mpsc::channel(1000);
    let service = ClickhouseService::new(rx);
    (tx, service)
}