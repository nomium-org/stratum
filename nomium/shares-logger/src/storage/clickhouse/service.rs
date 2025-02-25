use super::queries::{CREATE_BLOCKS_TABLE, CREATE_SHARES_TABLE};
use crate::config::SETTINGS;
use crate::errors::ClickhouseError;
use crate::models::{BlockFound, ClickhouseBlock, ClickhouseShare, ShareLog};
use crate::storage::clickhouse::ConnectionPool;
use crate::traits::ShareStorage;
use async_trait::async_trait;
use clickhouse::Client;
use log::{info, error};
use std::sync::Arc;

use nomium_prometheus::SHALOG_BATCH_SIZE_CURRENT;

#[derive(Clone)]
pub struct ClickhouseStorage {
    batch: Vec<ShareLog>,
    last_flush: std::time::Instant,
    connection_pool: Arc<ConnectionPool>,
}

#[derive(Clone)]
pub struct ClickhouseBlockStorage {
    batch: Vec<BlockFound>,
    last_flush: std::time::Instant,
    connection_pool: Arc<ConnectionPool>,
}

impl ClickhouseStorage {
    pub fn new() -> Result<Self, ClickhouseError> {
        let connection_pool = Arc::new(ConnectionPool::new(SETTINGS.clickhouse.pool_size));
        Ok(Self {
            connection_pool,
            batch: Vec::with_capacity(SETTINGS.clickhouse.batch_size),
            last_flush: std::time::Instant::now(),
        })
    }

    async fn get_client(&self) -> Result<Client, ClickhouseError> {
        let conn = self.connection_pool.get_connection().await?;
        let client = {
            let conn_guard = conn.lock().await;
            conn_guard
                .client
                .clone()
                .ok_or_else(|| ClickhouseError::ConnectionError("No client available".into()))?
        };
        self.connection_pool.release_connection(conn).await;
        Ok(client)
    }

    async fn ensure_table_exists(&self) -> Result<(), ClickhouseError> {
        info!("Checking existence of shares table");

        let client = self.get_client().await?;
        client
            .query(CREATE_SHARES_TABLE)
            .execute()
            .await
            .map_err(|e| {
                ClickhouseError::TableCreationError(format!("Failed to create shares table: {}", e))
            })?;

        info!("Table SHARES created or already exist");
        Ok(())
    }
}

impl ClickhouseBlockStorage {
    pub fn new() -> Result<Self, ClickhouseError> {
        info!("Initializing ClickhouseBlockStorage...");
        let connection_pool = Arc::new(ConnectionPool::new(SETTINGS.clickhouse.pool_size));

        Ok(Self {
            connection_pool,
            batch: Vec::with_capacity(SETTINGS.clickhouse.batch_size),
            last_flush: std::time::Instant::now(),
        })
    }

    async fn get_client(&self) -> Result<Client, ClickhouseError> {
        let conn = self.connection_pool.get_connection().await?;
        let client = {
            let conn_guard = conn.lock().await;
            conn_guard
                .client
                .clone()
                .ok_or_else(|| ClickhouseError::ConnectionError("No client available".into()))?
        };
        self.connection_pool.release_connection(conn).await;
        Ok(client)
    }

    async fn ensure_blocks_table_exists(&self) -> Result<(), ClickhouseError> {
        info!("Checking existence of blocks table");

        let client = self.get_client().await?;
        client
            .query(CREATE_BLOCKS_TABLE)
            .execute()
            .await
            .map_err(|e| {
                ClickhouseError::TableCreationError(format!("Failed to create blocks table: {}", e))
            })?;
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
        SHALOG_BATCH_SIZE_CURRENT.set(self.batch.len() as i64);
        let should_flush = self.batch.len() >= SETTINGS.clickhouse.batch_size
            || self.last_flush.elapsed()
                >= std::time::Duration::from_secs(SETTINGS.clickhouse.batch_flush_interval_secs);
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

        let batch_to_flush = self.batch.clone();
        let batch_size = batch_to_flush.len();
        info!("Flushing batch of {} records", batch_size);

        let client = self.get_client().await?;
        let mut batch_inserter = client
            .insert("shares")
            .map_err(|e| ClickhouseError::BatchInsertError(e.to_string()))?;

        for share in batch_to_flush.iter() {
            let clickhouse_share = ClickhouseShare::from(share.clone());
            if let Err(e) = batch_inserter.write(&clickhouse_share).await {
                error!("Failed to write share during flush: {}. Retrying later.", e);
                return Err(ClickhouseError::BatchInsertError(e.to_string()));
            }
        }

        if let Err(e) = batch_inserter.end().await {
            error!("Failed to complete batch insert: {}. Retrying later.", e);
            return Err(ClickhouseError::BatchInsertError(e.to_string()));
        }

        self.batch.clear();
        SHALOG_BATCH_SIZE_CURRENT.set(self.batch.len() as i64);
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
        let should_flush = self.batch.len() >= SETTINGS.clickhouse.batch_size
            || self.last_flush.elapsed()
                >= std::time::Duration::from_secs(SETTINGS.clickhouse.batch_flush_interval_secs);
        if should_flush {
            self.flush().await?;
        }
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
        let batch_to_flush = self.batch.clone();
        let batch_size = batch_to_flush.len();
        info!("Flushing batch of {} block records", batch_size);

        let client = self.get_client().await?;
        let mut batch_inserter = client
            .insert("blocks")
            .map_err(|e| ClickhouseError::BatchInsertError(e.to_string()))?;

        for block in batch_to_flush.iter() {
            let clickhouse_block = ClickhouseBlock::from(block.clone());
            if let Err(e) = batch_inserter.write(&clickhouse_block).await {
                error!("Failed to write block during flush: {}. Retrying later.", e);
                return Err(ClickhouseError::BatchInsertError(e.to_string()));
            }
        }

        if let Err(e) = batch_inserter.end().await {
            error!(
                "Failed to complete batch insert for blocks: {}. Retrying later.",
                e
            );
            return Err(ClickhouseError::BatchInsertError(e.to_string()));
        }

        self.batch.clear();
        self.last_flush = std::time::Instant::now();
        info!("Successfully flushed {} block records", batch_size);
        Ok(())
    }
}
