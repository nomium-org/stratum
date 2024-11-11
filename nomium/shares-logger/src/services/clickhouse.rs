use crate::ShareLog;
use crate::config::CONFIG;
use clickhouse::{Client, Row};
use log::info;
use std::time::Duration;
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
    batch: Vec<ShareLog>,
    last_flush: std::time::Instant,
}

impl ClickhouseService {
    pub fn new() -> Self {
        info!("Initializing ClickhouseService with config: url={}, database={}, user={}", 
            CONFIG.url, CONFIG.database, CONFIG.username);
            
        let client = Client::default()
            .with_url(&CONFIG.url)
            .with_database(&CONFIG.database)
            .with_user(&CONFIG.username)
            .with_password(&CONFIG.password);

        Self {
            client,
            batch: Vec::with_capacity(CONFIG.batch_size),
            last_flush: std::time::Instant::now(),
        }
    }

    pub async fn process_share(&mut self, share: ShareLog) -> Result<(), clickhouse::error::Error> {
        //info!("Начинаем process_share");
        //services::debug_log::log_share_hash("clickhouse_process", &share);
        //info!("Закончили логирование хэша");
        self.batch.push(share);

        let should_flush = self.batch.len() >= CONFIG.batch_size || 
                          self.last_flush.elapsed() >= Duration::from_secs(5);

        if should_flush {
            self.flush_batch().await?;
        }

        Ok(())
    }

    async fn flush_batch(&mut self) -> Result<(), clickhouse::error::Error> {
        if self.batch.is_empty() {
            return Ok(());
        }

        // Ensure table exists before insertion
        self.ensure_table_exists().await?;

        let batch_size = self.batch.len();
        info!("Flushing batch of {} shares to ClickHouse", batch_size);

        let mut insert = self.client.insert("shares")?;
        for share in self.batch.drain(..) {
            let clickhouse_share = ClickhouseShare::from(share);
            insert.write(&clickhouse_share).await?;
        }
        insert.end().await?;

        self.last_flush = std::time::Instant::now();
        info!("Successfully flushed {} shares to ClickHouse", batch_size);

        Ok(())
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
}