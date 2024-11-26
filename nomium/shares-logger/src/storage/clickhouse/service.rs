use async_trait::async_trait;
use crate::traits::ShareStorage;
use crate::models::{ShareLog, BlockFound, ClickhouseShare, ClickhouseBlock}; 
use crate::config::SETTINGS;
use crate::errors::ClickhouseError;
use clickhouse::Client;
use log::info;
use std::time::Duration;

#[derive(Clone)]
pub struct ClickhouseStorage {
    client: Client,
    batch: Vec<ShareLog>,
    last_flush: std::time::Instant,
}

#[derive(Clone)]
pub struct ClickhouseBlockStorage {
    client: Client,
    batch: Vec<BlockFound>,
    last_flush: std::time::Instant,
}

impl ClickhouseStorage {
    pub fn new() -> Result<Self, ClickhouseError> {
        info!("Initializing ClickhouseStorage...");
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

impl ClickhouseBlockStorage {
    pub fn new() -> Result<Self, ClickhouseError> {
        info!("Initializing ClickhouseBlockStorage...");
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

    async fn ensure_blocks_table_exists(&self) -> Result<(), ClickhouseError> {
        info!("Checking existence of blocks table");
        let create_table_query = r#"
            CREATE TABLE IF NOT EXISTS blocks (
                channel_id UInt32,
                block_hash String,
                timestamp UInt64,
                found_at DateTime64(3) DEFAULT now64(3)
            ) ENGINE = MergeTree()
            ORDER BY (channel_id, timestamp)
            SETTINGS index_granularity = 8192
        "#;
        self.client.query(create_table_query)
            .execute()
            .await
            .map_err(|e| ClickhouseError::TableCreationError(format!("Failed to create blocks table: {}", e)))?;
        info!("Blocks table created or already exists");
        Ok(())
    }
}

#[async_trait]
impl ShareStorage<ShareLog> for ClickhouseStorage {
    async fn init(&self) -> Result<(), ClickhouseError> {
        self.ensure_table_exists().await
    }

    async fn store_share(&mut self, share: ShareLog) -> Result<(), ClickhouseError> {
        self.batch.push(share);
        let should_flush = self.batch.len() >= SETTINGS.clickhouse.batch_size || 
                          self.last_flush.elapsed() >= Duration::from_secs(SETTINGS.clickhouse.batch_flush_interval_secs);
        if should_flush {
            ShareStorage::<ShareLog>::flush(self).await?;
        }
        Ok(())
    }

    async fn store_batch(&mut self, shares: Vec<ShareLog>) -> Result<(), ClickhouseError> {
        for share in shares {
            ShareStorage::<ShareLog>::store_share(self, share).await?;
        }
        Ok(())
    }

    async fn flush(&mut self) -> Result<(), ClickhouseError> {
        if self.batch.is_empty() {
            return Ok(());
        }

        let batch_size = self.batch.len();
        info!("Flushing batch of {} records", batch_size);

        let mut batch_inserter = self.client.insert("shares")
            .map_err(|e| ClickhouseError::BatchInsertError(e.to_string()))?;

        for share in self.batch.drain(..) {
            let clickhouse_share = ClickhouseShare::from(share);
            batch_inserter.write(&clickhouse_share).await
                .map_err(|e| ClickhouseError::BatchInsertError(e.to_string()))?;
        }

        batch_inserter.end().await
            .map_err(|e| ClickhouseError::BatchInsertError(e.to_string()))?;

        self.last_flush = std::time::Instant::now();
        info!("Successfully flushed {} records", batch_size);
        Ok(())
    }
}

#[async_trait]
impl ShareStorage<BlockFound> for ClickhouseBlockStorage {
    async fn init(&self) -> Result<(), ClickhouseError> {
        self.ensure_blocks_table_exists().await
    }

    async fn store_share(&mut self, block: BlockFound) -> Result<(), ClickhouseError> {
        self.batch.push(block);
        let should_flush = self.batch.len() >= SETTINGS.clickhouse.batch_size || 
                          self.last_flush.elapsed() >= Duration::from_secs(SETTINGS.clickhouse.batch_flush_interval_secs);
        if should_flush {
            ShareStorage::<BlockFound>::flush(self).await?;
        }
        Ok(())
    }

    async fn store_batch(&mut self, blocks: Vec<BlockFound>) -> Result<(), ClickhouseError> {
        for block in blocks {
            ShareStorage::<BlockFound>::store_share(self, block).await?;
        }
        Ok(())
    }

    async fn flush(&mut self) -> Result<(), ClickhouseError> {
        if self.batch.is_empty() {
            return Ok(());
        }

        let batch_size = self.batch.len();
        info!("Flushing batch of {} block records", batch_size);

        let mut batch_inserter = self.client.insert("blocks")
            .map_err(|e| ClickhouseError::BatchInsertError(e.to_string()))?;

        for block in self.batch.drain(..) {
            let clickhouse_block = ClickhouseBlock::from(block);
            batch_inserter.write(&clickhouse_block).await
                .map_err(|e| ClickhouseError::BatchInsertError(e.to_string()))?;
        }

        batch_inserter.end().await
            .map_err(|e| ClickhouseError::BatchInsertError(e.to_string()))?;

        self.last_flush = std::time::Instant::now();
        info!("Successfully flushed {} block records", batch_size);
        Ok(())
    }
}