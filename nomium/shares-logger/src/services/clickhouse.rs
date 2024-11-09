use crate::ShareLog;
use crate::config::CONFIG;
use clickhouse::{Client, Row};
use log::{error, debug};
use std::time::Duration;
use tokio::sync::mpsc::{self, Receiver};

#[derive(Row)]
struct ClickhouseShare {
    channel_id: u32,
    sequence_number: u32,
    job_id: u32,
    nonce: u32,
    ntime: u32,
    version: u32,
    hash: String,
    is_valid: bool,
    extranonce: String,
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
            hash: hex::encode(&share.hash),
            is_valid: share.is_valid,
            extranonce: hex::encode(&share.extranonce),
        }
    }
}

pub struct ClickhouseService {
    client: Client,
    batch_receiver: Receiver<ShareLog>,
}

impl ClickhouseService {
    pub fn new(batch_receiver: Receiver<ShareLog>) -> Self {
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
        let mut batch = Vec::with_capacity(CONFIG.batch_size);
        let mut interval = tokio::time::interval(Duration::from_secs(5));
        loop {
            tokio::select! {
                Some(share) = self.batch_receiver.recv() => {
                    batch.push(share);
                    if batch.len() >= CONFIG.batch_size {
                        if let Err(e) = self.insert_batch(&batch).await {
                            error!("Failed to insert batch: {}", e);
                        }
                        batch.clear();
                    }
                }
                _ = interval.tick() => {
                    if !batch.is_empty() {
                        if let Err(e) = self.insert_batch(&batch).await {
                            error!("Failed to insert batch: {}", e);
                        }
                        batch.clear();
                    }
                }
            }
        }
    }

    async fn insert_batch(&self, batch: &[ShareLog]) -> Result<(), clickhouse::error::Error> {
        debug!("Starting to write batch of {} shares", batch.len());

        let mut values = Vec::with_capacity(batch.len());
        for share in batch {
            let hash_str = format!("[{}]", share.hash.iter().map(|b| b.to_string()).collect::<Vec<_>>().join(","));
            let extranonce_str = format!("[{}]", share.extranonce.iter().map(|b| b.to_string()).collect::<Vec<_>>().join(","));
            
            values.push(format!(
                "({}, {}, {}, {}, {}, {}, '{}', {}, '{}')",
                share.channel_id,
                share.sequence_number,
                share.job_id,
                share.nonce,
                share.ntime,
                share.version,
                hash_str,
                if share.is_valid { 1 } else { 0 },
                extranonce_str
            ));
        }

        let query = format!(
            "INSERT INTO shares (
                channel_id, sequence_number, job_id, nonce, 
                ntime, version, hash, is_valid, extranonce
            ) VALUES {}",
            values.join(",")
        );

        debug!("Executing query: {}", query);
        match self.client.query(&query).execute().await {
            Ok(_) => {
                debug!("Successfully wrote batch to database");
                Ok(())
            },
            Err(e) => {
                error!("Failed to write batch to database: {}", e);
                Err(e)
            }
        }
    }
}

pub fn create_clickhouse_service() -> (mpsc::Sender<ShareLog>, ClickhouseService) {
    let (tx, rx) = mpsc::channel(1000);
    let service = ClickhouseService::new(rx);
    (tx, service)
}