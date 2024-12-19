use crate::models::AuthorizationLog;
use crate::services::external_api::ExternalApiService;
use crate::storage::clickhouse::ClickhouseAuthStorage;
use crate::models::ClickhouseAuthRecord;
use tokio::sync::mpsc;
use log::{info, error};
use anyhow::Error;

pub struct AuthorizationProcessor {
    receiver: mpsc::Receiver<AuthorizationLog>,
    api_service: ExternalApiService,
    storage: ClickhouseAuthStorage,
}

impl AuthorizationProcessor {
    pub fn new(
        receiver: mpsc::Receiver<AuthorizationLog>, 
        api_service: ExternalApiService,
    ) -> Self {
        let storage = ClickhouseAuthStorage::new()
            .expect("Failed to create ClickHouse auth storage");
        Self { 
            receiver, 
            api_service,
            storage,
        }
    }

    pub async fn run(&mut self) -> Result<(), Error> {
        if let Err(e) = self.storage.init().await {
            return Err(Error::new(e));
        }
        while let Some(auth_log) = self.receiver.recv().await {
            info!(
                "Processing authorization: name = {}, password = {}",
                auth_log.name, auth_log.password
            );
            
            if let Some((account_name, worker_number)) = Self::parse_name(&auth_log.name) {
                match self
                    .api_service
                    .authenticate_worker(&account_name, worker_number)
                    .await
                {
                    Ok(response) => {
                        info!("API Response: {:?}", response);
                        let record = ClickhouseAuthRecord::new(
                            account_name,
                            worker_number,
                            response.isSuccess,
                            response.workerId,
                            response.workerName,
                            response.userId,
                            response.accountId,
                        );
                        if let Err(e) = self.storage.store_auth_record(record).await {
                            error!("Failed to store auth record: {}", e);
                        }
                    }
                    Err(err) => {
                        error!("API Request failed: {:?}", err);
                    }
                }
            } else {
                error!("Failed to parse name: {}", auth_log.name);
            }
        }
        Ok(())
    }

    fn parse_name(full_name: &str) -> Option<(String, u32)> {
        let parts: Vec<&str> = full_name.split('.').collect();
        if parts.len() == 2 {
            if let Ok(worker_number) = parts[1].parse::<u32>() {
                return Some((parts[0].to_string(), worker_number));
            }
        }
        None
    }
}