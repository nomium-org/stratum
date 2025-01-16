use async_trait::async_trait;
use crate::traits::ShareStorage;
use crate::models::{ShareLog, BlockFound, ClickhouseShare, ClickhouseBlock}; 
use crate::config::SETTINGS;
use crate::errors::ClickhouseError;
use clickhouse::Client;
use log::info;
use std::time::Duration;
use super::queries::{CREATE_SHARES_TABLE, CREATE_BLOCKS_TABLE, CREATE_HASHRATE_VIEW};
use crate::services::retry::{retry_operation, RetryConfig};
use std::future::Future;

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

        self.client.query(CREATE_SHARES_TABLE)
            .execute()
            .await
            .map_err(|e| ClickhouseError::TableCreationError(format!("Failed to create shares table: {}", e)))?;

        self.client.query(CREATE_HASHRATE_VIEW)
            .execute()
            .await
            .map_err(|e| ClickhouseError::TableCreationError(format!("Failed to create materialized view: {}", e)))?;

        info!("Table and materialized view created or already exist");
        Ok(())
    }

    async fn execute_with_retry<F, Fut, T>(
        &self,
        operation: F,
        operation_name: &str
    ) -> Result<T, ClickhouseError>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = Result<T, ClickhouseError>>,
    {
        let config = RetryConfig {
            max_retries: SETTINGS.retry.max_retries,
            initial_delay: Duration::from_millis(SETTINGS.retry.initial_delay_ms),
            max_delay: Duration::from_millis(SETTINGS.retry.max_delay_ms),
        };

        retry_operation(operation, &config, operation_name).await
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

        self.client.query(CREATE_BLOCKS_TABLE)
            .execute()
            .await
            .map_err(|e| ClickhouseError::TableCreationError(format!("Failed to create blocks table: {}", e)))?;
        info!("Blocks table created or already exists");
        Ok(())
    }

    async fn execute_with_retry<F, Fut, T>(
        &self,
        operation: F,
        operation_name: &str
    ) -> Result<T, ClickhouseError>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = Result<T, ClickhouseError>>,
    {
        let config = RetryConfig {
            max_retries: SETTINGS.retry.max_retries,
            initial_delay: Duration::from_millis(SETTINGS.retry.initial_delay_ms),
            max_delay: Duration::from_millis(SETTINGS.retry.max_delay_ms),
        };

        retry_operation(operation, &config, operation_name).await
    }
}

#[async_trait]
impl ShareStorage<ShareLog> for ClickhouseStorage {
    async fn init(&self) -> Result<(), ClickhouseError> {
        self.execute_with_retry(
            || async { self.ensure_table_exists().await },
            "init_tables"
        ).await
    }

    async fn store_share(&mut self, share: ShareLog) -> Result<(), ClickhouseError> {
        self.batch.push(share);
        
        let should_flush = self.batch.len() >= SETTINGS.clickhouse.batch_size || 
            self.last_flush.elapsed() >= Duration::from_secs(SETTINGS.clickhouse.batch_flush_interval_secs);

        if should_flush {
            self.flush().await?;
        }

        Ok(())
    }

    async fn store_batch(&mut self, shares: Vec<ShareLog>) -> Result<(), ClickhouseError> {
        for share in shares {
            self.store_share(share).await?;
        }
        Ok(())
    }

    async fn flush(&mut self) -> Result<(), ClickhouseError> {
        if self.batch.is_empty() {
            return Ok(());
        }

        let batch_size = self.batch.len();
        info!("Flushing batch of {} records", batch_size);

        let batch = std::mem::take(&mut self.batch);
        
        self.execute_with_retry(
            || async {
                let mut batch_inserter = self.client.insert("shares")
                    .map_err(|e| ClickhouseError::BatchInsertError(e.to_string()))?;

                for share in batch.iter() {
                    let clickhouse_share = ClickhouseShare::from(share.clone());
                    batch_inserter.write(&clickhouse_share).await
                        .map_err(|e| ClickhouseError::BatchInsertError(e.to_string()))?;
                }

                batch_inserter.end().await
                    .map_err(|e| ClickhouseError::BatchInsertError(e.to_string()))
            },
            "batch_flush"
        ).await?;

        self.last_flush = std::time::Instant::now();
        info!("Successfully flushed {} records", batch_size);
        Ok(())
    }
}

#[async_trait]
impl ShareStorage<BlockFound> for ClickhouseBlockStorage {
    async fn init(&self) -> Result<(), ClickhouseError> {
        self.execute_with_retry(
            || async { self.ensure_blocks_table_exists().await },
            "init_blocks_table"
        ).await
    }

    async fn store_share(&mut self, block: BlockFound) -> Result<(), ClickhouseError> {
        info!("Storing found block immediately");
        let block_data = ClickhouseBlock::from(block);
        info!("Block data for insert: {:?}", block_data);
        
        self.execute_with_retry(
            || async {
                let mut batch_inserter = self.client.insert("blocks")
                    .map_err(|e| ClickhouseError::BatchInsertError(e.to_string()))?;

                batch_inserter.write(&block_data).await
                    .map_err(|e| ClickhouseError::BatchInsertError(e.to_string()))?;

                batch_inserter.end().await
                    .map_err(|e| ClickhouseError::BatchInsertError(e.to_string()))
            },
            "store_block"
        ).await?;

        info!("Block storage completed");
        Ok(())
    }

    async fn store_batch(&mut self, blocks: Vec<BlockFound>) -> Result<(), ClickhouseError> {
        for block in blocks {
            self.store_share(block).await?;
        }
        Ok(())
    }

    async fn flush(&mut self) -> Result<(), ClickhouseError> {
        if self.batch.is_empty() {
            return Ok(());
        }

        let batch_size = self.batch.len();
        info!("Flushing batch of {} block records", batch_size);

        let batch = std::mem::take(&mut self.batch);
        
        self.execute_with_retry(
            || async {
                let mut batch_inserter = self.client.insert("blocks")
                    .map_err(|e| ClickhouseError::BatchInsertError(e.to_string()))?;

                for block in batch.iter() {
                    let clickhouse_block = ClickhouseBlock::from(block.clone());
                    batch_inserter.write(&clickhouse_block).await
                        .map_err(|e| ClickhouseError::BatchInsertError(e.to_string()))?;
                }

                batch_inserter.end().await
                    .map_err(|e| ClickhouseError::BatchInsertError(e.to_string()))
            },
            "batch_flush_blocks"
        ).await?;

        self.last_flush = std::time::Instant::now();
        info!("Successfully flushed {} block records", batch_size);
        Ok(())
    }
}