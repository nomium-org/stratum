use crate::ShareLog;
use crate::config::CONFIG;
use clickhouse::{Client, Row};
use log::{info, error};
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
        // Гарантируем корректное hex-кодирование
        let hash_hex = share.hash.iter()
            .fold(String::with_capacity(share.hash.len() * 2), |mut acc, &b| {
                acc.push_str(&format!("{:02x}", b));
                acc
            });
            
        let extranonce_hex = share.extranonce.iter()
            .fold(String::with_capacity(share.extranonce.len() * 2), |mut acc, &b| {
                acc.push_str(&format!("{:02x}", b));
                acc
            });

        Self {
            channel_id: share.channel_id,
            sequence_number: share.sequence_number,
            job_id: share.job_id,
            nonce: share.nonce,
            ntime: share.ntime,
            version: share.version,
            hash: hash_hex,
            is_valid: if share.is_valid { 1 } else { 0 },
            extranonce: extranonce_hex,
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
        info!("Initializing ClickhouseService with configuration: url={}, database={}, user={}", 
        CONFIG.clickhouse_url, CONFIG.clickhouse_database, CONFIG.clickhouse_username);

        let client = Client::default()
            .with_url(&CONFIG.clickhouse_url)
            .with_database(&CONFIG.clickhouse_database)
            .with_user(&CONFIG.clickhouse_username)
            .with_password(&CONFIG.clickhouse_password);
        Self {
            client,
            batch: Vec::with_capacity(CONFIG.clickhouse_batch_size),
            last_flush: std::time::Instant::now(),
        }
    }

    pub async fn process_share(&mut self, share: ShareLog) -> Result<(), clickhouse::error::Error> {
        self.batch.push(share);
        let should_flush = self.batch.len() >= CONFIG.clickhouse_batch_size || 
                   self.last_flush.elapsed() >= Duration::from_secs(CONFIG.batch_flush_interval_secs);
        if should_flush {
            self.flush_batch().await?;
        }
        Ok(())
    }

    async fn flush_batch(&mut self) -> Result<(), clickhouse::error::Error> {
        if self.batch.is_empty() {
            return Ok(());
        }

        self.ensure_table_exists().await?;
        let batch_size = self.batch.len();
        info!("Starting to write batch of {} records to ClickHouse", batch_size);

        let mut insert = self.client.insert("shares")?;
        
        for (index, share) in self.batch.drain(..).enumerate() {
            let clickhouse_share = ClickhouseShare::from(share);
            info!("Prepared record {}/{}: hash={}, extranonce={}", 
                index + 1, batch_size, clickhouse_share.hash, clickhouse_share.extranonce);
                
            if let Err(e) = insert.write(&clickhouse_share).await {
                error!("Error writing share to ClickHouse: {:?}", e);
                return Err(e);
            }
        }

        if let Err(e) = insert.end().await {
            error!("Error finalizing batch insert: {:?}", e);
            return Err(e);
        }

        self.last_flush = std::time::Instant::now();
        info!("Успешно записан батч из {} записей в ClickHouse", batch_size);
        Ok(())
    }

    async fn ensure_table_exists(&self) -> Result<(), clickhouse::error::Error> {
        info!("Checking existence of shares table");
        
        let create_table_query = r#"
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
                timestamp DateTime64(3) DEFAULT now64(3)
            ) ENGINE = MergeTree()
            PARTITION BY toYYYYMMDD(timestamp)
            PRIMARY KEY (channel_id, timestamp)
            ORDER BY (channel_id, timestamp, sequence_number)
            SETTINGS index_granularity = 8192
        "#;
        self.client.query(create_table_query).execute().await?;
        
        let create_mv_query = r#"
            CREATE MATERIALIZED VIEW IF NOT EXISTS mv_hash_rate_stats
            ENGINE = SummingMergeTree()
            PARTITION BY toYYYYMMDD(period_start)
            ORDER BY (channel_id, period_start)
            AS
            SELECT
                channel_id,
                toStartOfMinute(timestamp) as period_start,
                count() as share_count,
                sum(difficulty * pow(2, 32)) as total_hashes,
                min(timestamp) as min_timestamp,
                max(timestamp) as max_timestamp
            FROM shares
            GROUP BY channel_id, period_start
        "#;
        self.client.query(create_mv_query).execute().await?;
        
        info!("Table and materialized view created or already exist");
        Ok(())
    }
}