use async_trait::async_trait;
use crate::traits::ShareStorage;
use crate::models::{ShareLog, BlockFound, ClickhouseShare, ClickhouseBlock}; 
use crate::config::SETTINGS;
use crate::errors::ClickhouseError;
use clickhouse::Client;
use log::info;
use std::time::Duration;
use super::queries::{CREATE_SHARES_TABLE, CREATE_BLOCKS_TABLE, CREATE_HASHRATE_VIEW};
use log::error;
use crate::storage::clickhouse::ConnectionPool;
use std::sync::Arc;

#[derive(Clone)]
pub struct ClickhouseStorage {
    //client: Client,
    batch: Vec<ShareLog>,
    last_flush: std::time::Instant,
    pool: Arc<ConnectionPool>,
}

#[derive(Clone)]
pub struct ClickhouseBlockStorage {
    //client: Client,
    batch: Vec<BlockFound>,
    last_flush: std::time::Instant,
    pool: Arc<ConnectionPool>,
}

impl ClickhouseStorage {

    pub fn new() -> Result<Self, ClickhouseError> {
        let pool = Arc::new(ConnectionPool::new(SETTINGS.clickhouse.pool_size));
        Ok(Self {
            pool,
            batch: Vec::with_capacity(SETTINGS.clickhouse.batch_size),
            last_flush: std::time::Instant::now(),
        })
    }

    async fn get_client(&self) -> Result<Client, ClickhouseError> {
        let conn = self.pool.get_connection().await?;
        let client = conn.lock().await.clone().ok_or_else(|| 
            ClickhouseError::ConnectionError("No client available".into())
        )?;
        Ok(client)
    }

    async fn ensure_table_exists(&self) -> Result<(), ClickhouseError> {
        info!("Checking existence of shares table");

        let client = self.get_client().await?;
        client.query(CREATE_SHARES_TABLE)
            .execute()
            .await
            .map_err(|e| ClickhouseError::TableCreationError(format!("Failed to create shares table: {}", e)))?;

        client.query(CREATE_HASHRATE_VIEW)
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
        let pool = Arc::new(ConnectionPool::new(SETTINGS.clickhouse.pool_size));
        
        Ok(Self {
            pool,
            batch: Vec::with_capacity(SETTINGS.clickhouse.batch_size),
            last_flush: std::time::Instant::now(),
        })
    }

    async fn get_client(&self) -> Result<Client, ClickhouseError> {
        let conn = self.pool.get_connection().await?;
        let client = conn.lock().await.clone().ok_or_else(|| 
            ClickhouseError::ConnectionError("No client available".into())
        )?;
        Ok(client)
    }

    async fn ensure_blocks_table_exists(&self) -> Result<(), ClickhouseError> {
        info!("Checking existence of blocks table");

        let client = self.get_client().await?;
        client.query(CREATE_BLOCKS_TABLE)
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
        let should_flush =
            self.batch.len() >= SETTINGS.clickhouse.batch_size ||
            self.last_flush.elapsed() >= std::time::Duration::from_secs(SETTINGS.clickhouse.batch_flush_interval_secs);
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
        log::info!("Flushing batch of {} records", batch_size);

        let client = self.get_client().await?;
        let mut batch_inserter = client.insert("shares")
            .map_err(|e| ClickhouseError::BatchInsertError(e.to_string()))?;

        for share in batch_to_flush.iter() {
            let clickhouse_share = ClickhouseShare::from(share.clone());
            if let Err(e) = batch_inserter.write(&clickhouse_share).await {
                log::error!("Failed to write share during flush: {}. Retrying later.", e);
                return Err(ClickhouseError::BatchInsertError(e.to_string()));
            }
        }

        if let Err(e) = batch_inserter.end().await {
            log::error!("Failed to complete batch insert: {}. Retrying later.", e);
            return Err(ClickhouseError::BatchInsertError(e.to_string()));
        }

        self.batch.clear();
        self.last_flush = std::time::Instant::now();
        log::info!("Successfully flushed {} records", batch_size);
        Ok(())
    }
}

#[async_trait]
impl ShareStorage<BlockFound> for ClickhouseBlockStorage {
    async fn init(&self) -> Result<(), ClickhouseError> {
        self.ensure_blocks_table_exists().await
    }

    async fn store_share(&mut self, block: BlockFound) -> Result<(), ClickhouseError> {
        info!("Storing found block immediately");
        let block_data = ClickhouseBlock::from(block);
        info!("Block data for insert: {:?}", block_data);
        
        let client = self.get_client().await?;
        let mut batch_inserter = match client.insert("blocks") {
            Ok(inserter) => {
                info!("Created batch inserter successfully");
                inserter
            },
            Err(e) => {
                error!("Failed to create batch inserter: {:?}", e);
                return Err(ClickhouseError::BatchInsertError(e.to_string()));
            }
        };
    
        match batch_inserter.write(&block_data).await {
            Ok(_) => info!("Successfully wrote block data"),
            Err(e) => error!("Failed to write block data: {:?}", e),
        }
    
        match batch_inserter.end().await {
            Ok(_) => info!("Successfully ended batch insert"),
            Err(e) => error!("Failed to end batch insert: {:?}", e),
        }
    
        info!("Block storage completed");
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

        let client = self.get_client().await?;
        let mut batch_inserter = client.insert("blocks")
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