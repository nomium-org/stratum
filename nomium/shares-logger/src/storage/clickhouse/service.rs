use super::queries::{CREATE_BLOCKS_TABLE, CREATE_SHARES_TABLE};
use crate::config::SETTINGS;
use crate::errors::ClickhouseError;
use crate::models::{BlockFound, ClickhouseBlock, ClickhouseShare, ShareLog};
use crate::storage::clickhouse::ClickhouseConnectionPool;
use crate::traits::ShareStorage;
use async_trait::async_trait;
use clickhouse::Client;
use log::{info, error, warn};
use std::sync::Arc;
use rpc_sv2::mini_rpc_client::RpcError;

use nomium_prometheus::SHALOG_BATCH_SIZE_CURRENT;

#[derive(Clone)]
pub struct ClickhouseStorage {
    batch: Vec<ShareLog>,
    last_flush: std::time::Instant,
    clickhouse_connection_pool: Arc<ClickhouseConnectionPool>,
}

#[derive(Clone)]
pub struct ClickhouseBlockStorage {
    batch: Vec<BlockFound>,
    last_flush: std::time::Instant,
    clickhouse_connection_pool: Arc<ClickhouseConnectionPool>,
}

impl ClickhouseStorage {
    pub fn new() -> Result<Self, ClickhouseError> {
        let clickhouse_connection_pool = Arc::new(ClickhouseConnectionPool::new(SETTINGS.clickhouse.pool_size));
        Ok(Self {
            clickhouse_connection_pool,
            batch: Vec::with_capacity(SETTINGS.clickhouse.batch_size),
            last_flush: std::time::Instant::now(),
        })
    }

    async fn get_client(&self) -> Result<Client, ClickhouseError> {
        let conn = self.clickhouse_connection_pool.get_connection().await?;
        let client = {
            let conn_guard = conn.lock().await;
            conn_guard
                .client
                .clone()
                .ok_or_else(|| ClickhouseError::ConnectionError("No client available".into()))?
        };
        self.clickhouse_connection_pool.release_connection(conn).await;
        Ok(client)
    }

    async fn ensure_table_exists(&self) -> Result<(), ClickhouseError> {
        info!(target: "shares", "Checking existence of shares table");

        let client = self.get_client().await?;
        client
            .query(CREATE_SHARES_TABLE)
            .execute()
            .await
            .map_err(|e| {
                ClickhouseError::TableCreationError(format!("Failed to create shares table: {}", e))
            })?;

        info!(target: "shares", "Table SHARES created or already exist");
        Ok(())
    }
}

impl ClickhouseBlockStorage {
    pub fn new() -> Result<Self, ClickhouseError> {
        info!(target: "shares", "Initializing ClickhouseBlockStorage...");
        let clickhouse_connection_pool = Arc::new(ClickhouseConnectionPool::new(SETTINGS.clickhouse.pool_size));

        Ok(Self {
            clickhouse_connection_pool,
            batch: Vec::with_capacity(SETTINGS.clickhouse.batch_size),
            last_flush: std::time::Instant::now(),
        })
    }

    async fn get_client(&self) -> Result<Client, ClickhouseError> {
        let conn = self.clickhouse_connection_pool.get_connection().await?;
        let client = {
            let conn_guard = conn.lock().await;
            conn_guard
                .client
                .clone()
                .ok_or_else(|| ClickhouseError::ConnectionError("No client available".into()))?
        };
        self.clickhouse_connection_pool.release_connection(conn).await;
        Ok(client)
    }

    async fn ensure_blocks_table_exists(&self) -> Result<(), ClickhouseError> {
        info!(target: "shares", "Checking existence of blocks table");

        let client = self.get_client().await?;
        client
            .query(CREATE_BLOCKS_TABLE)
            .execute()
            .await
            .map_err(|e| {
                ClickhouseError::TableCreationError(format!("Failed to create blocks table: {}", e))
            })?;
        info!(target: "shares", "Blocks table created or already exists");
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
        info!(target: "shares", "Flushing batch of {} records", batch_size);

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
        info!(target: "shares", "Successfully flushed {} records", batch_size);
        Ok(())
    }
}

#[async_trait]
impl ShareStorage<BlockFound> for ClickhouseBlockStorage {
    async fn init(&self) -> Result<(), ClickhouseError> {
        self.ensure_blocks_table_exists().await
    }
    async fn store_share(&mut self, block: BlockFound) -> Result<(), ClickhouseError> {
        let block_hash_hex = hex::encode(&block.block_hash);
        info!(target: "shares", "Processing block {} for storage in DB", block_hash_hex);
        let max_retries = SETTINGS.processing.block_verification_max_retries;
        let retry_delay_ms = SETTINGS.processing.block_verification_retry_delay_ms;
        let rpc_service = match crate::services::bitcoin_rpc_service::BitcoinRpcService::new() {
            Ok(service) => service,
            Err(e) => {
                error!(target: "shares", "Failed to initialize Bitcoin RPC service: {:?}", e);
                match &e {
                    RpcError::Other(msg) => {
                        error!(target: "shares", "Configuration error: {}", msg);
                        info!(target: "shares", "Check environment variables for Bitcoin RPC connection");
                    },
                    _ => {
                        error!(target: "shares", "Other error during initialization: {:?}", e);
                    }
                }
                // Block is not added to batch due to RPC initialization failure
                warn!(target: "shares", "Block {} will not be saved to DB due to RPC initialization failure", block_hash_hex);
                // TODO: Add custom logic for handling RPC initialization failures (e.g., fallback to alternative verification or notify monitoring)
                // Placeholder for future extensions
                return Ok(());
            }
        };

        let mut retry_count = 0;
        let mut block_exists = false;
        let mut last_error = None;
        while retry_count < max_retries {
            match rpc_service.is_block_in_blockchain(&block_hash_hex).await {
                Ok(exists) => {
                    if exists {
                        info!(target: "shares", "Block {} confirmed in blockchain on attempt {}, adding to batch", 
                             block_hash_hex, retry_count + 1);
                        block_exists = true;
                        break;
                    } else if retry_count + 1 < max_retries {
                        info!(target: "shares", "Block {} not found in blockchain on attempt {}, retrying in {}ms", 
                             block_hash_hex, retry_count + 1, retry_delay_ms);
                    } else {
                        info!(target: "shares", "Block {} not found in blockchain after {} attempts, skipping", 
                             block_hash_hex, max_retries);
                    }
                },
                Err(e) => {
                    error!(target: "shares", "Error checking block {} via RPC on attempt {}: {:?}", 
                          block_hash_hex, retry_count + 1, e);
                    last_error = Some(e);
                }
            }
            retry_count += 1;
            if retry_count < max_retries {
                tokio::time::sleep(std::time::Duration::from_millis(retry_delay_ms)).await;
            }
        }
        // Processing the result after all attempts
        if block_exists {
            self.batch.push(block);
        } else if let Some(e) = last_error {
            error!(target: "shares", "All attempts to check block {} failed with error: {:?}", 
                block_hash_hex, e);
            // Block is not added to batch due to verification error
            warn!(target: "shares", "Block {} will not be saved to DB due to verification failure", block_hash_hex);
            // TODO: Add custom logic for handling failed verification (e.g., queue for later re-check or notify monitoring)
            // Placeholder for future extensions
        } else {
            error!(target: "shares", "Block {} not found in blockchain after {} attempts", 
                block_hash_hex, max_retries);
            // Block is not added to batch due to not being found in blockchain
            warn!(target: "shares", "Block {} will not be saved to DB as it was not found in blockchain", block_hash_hex);
            // TODO: Add custom logic for handling blocks not found in blockchain (e.g., queue for later re-check or alternative verification)
            // Placeholder for future extensions
        }
        let should_flush = self.batch.len() >= SETTINGS.clickhouse.batch_size
            || self.last_flush.elapsed() >= std::time::Duration::from_secs(SETTINGS.clickhouse.batch_flush_interval_secs);
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
        info!(target: "shares", "Flushing batch of {} block records", batch_size);
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
        info!(target: "shares", "Successfully flushed {} block records", batch_size);
        Ok(())
    }
}