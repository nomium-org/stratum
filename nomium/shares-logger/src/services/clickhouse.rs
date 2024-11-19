use crate::config::SETTINGS;
use crate::errors::ClickhouseError;
use clickhouse::Client;
use log::info;
use std::time::Duration;
use crate::models::{ShareLog, ClickhouseShare};

pub struct ClickhouseService {
    client: Client,
    batch: Vec<ShareLog>,
    last_flush: std::time::Instant,
}

impl ClickhouseService {
    pub fn new() -> Result<Self, ClickhouseError> {
        info!("Initializing ClickhouseService with configuration: url={}, database={}, user={}", 
            SETTINGS.clickhouse.url, SETTINGS.clickhouse.database, SETTINGS.clickhouse.username);

        let client = Client::default()
            .with_url(&SETTINGS.clickhouse.url)
            .with_database(&SETTINGS.clickhouse.database)
            .with_user(&SETTINGS.clickhouse.username)
            .with_password(&SETTINGS.clickhouse.password);

        Ok(Self {
            client,
            batch: Vec::with_capacity(SETTINGS.clickhouse.batch_size),
            last_flush: std::time::Instant::now(),
        })
    }

    pub async fn process_share(&mut self, share: ShareLog) -> Result<(), ClickhouseError> {
        self.batch.push(share);
        
        let should_flush = self.batch.len() >= SETTINGS.clickhouse.batch_size || 
                          self.last_flush.elapsed() >= Duration::from_secs(SETTINGS.clickhouse.batch_flush_interval_secs);
        
        if should_flush {
            self.flush_batch().await?;
        }
        Ok(())
    }

    async fn flush_batch(&mut self) -> Result<(), ClickhouseError> {
        if self.batch.is_empty() {
            return Ok(());
        }

        self.ensure_table_exists().await?;
        
        let batch_size = self.batch.len();
        info!("Starting to write batch of {} records to ClickHouse", batch_size);

        let mut batch_inserter = self.client.insert("shares")
            .map_err(|e| ClickhouseError::BatchInsertError(e.to_string()))?;

        for (index, share) in self.batch.drain(..).enumerate() {
            let clickhouse_share = ClickhouseShare::from(share);
            info!("Prepared record {}/{}: hash={}, extranonce={}", 
                index + 1, batch_size, clickhouse_share.hash, clickhouse_share.extranonce);
                
            batch_inserter.write(&clickhouse_share).await
                .map_err(|e| ClickhouseError::BatchInsertError(format!("Failed to write share: {}", e)))?;
        }

        batch_inserter.end().await
            .map_err(|e| ClickhouseError::BatchInsertError(format!("Failed to finalize batch insert: {}", e)))?;

        self.last_flush = std::time::Instant::now();
        info!("Successfully written batch of {} records to ClickHouse", batch_size);
        Ok(())
    }

    async fn ensure_table_exists(&self) -> Result<(), ClickhouseError> {
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
                share_status UInt8,
                extranonce String,
                difficulty Float64,
                timestamp DateTime64(3) DEFAULT now64(3)
            ) ENGINE = MergeTree()
            PARTITION BY toYYYYMMDD(timestamp)
            PRIMARY KEY (channel_id, timestamp)
            ORDER BY (channel_id, timestamp, sequence_number)
            SETTINGS index_granularity = 8192
        "#;

        self.client.query(create_table_query)
            .execute()
            .await
            .map_err(|e| ClickhouseError::TableCreationError(format!("Failed to create shares table: {}", e)))?;

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

        self.client.query(create_mv_query)
            .execute()
            .await
            .map_err(|e| ClickhouseError::TableCreationError(format!("Failed to create materialized view: {}", e)))?;

        info!("Table and materialized view created or already exist");
        Ok(())
    }
}