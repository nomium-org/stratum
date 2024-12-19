use crate::models::AuthorizationLog;
use crate::services::external_api::ExternalApiService;
use tokio::sync::mpsc;
use log::info;

pub struct AuthorizationProcessor {
    receiver: mpsc::Receiver<AuthorizationLog>,
    api_service: ExternalApiService,
}

impl AuthorizationProcessor {
    pub fn new(receiver: mpsc::Receiver<AuthorizationLog>, api_service: ExternalApiService) -> Self {
        Self { receiver, api_service }
    }

    pub async fn run(&mut self) {
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
                    }
                    Err(err) => {
                        info!("API Request failed: {:?}", err);
                    }
                }
            } else {
                info!("Failed to parse name: {}", auth_log.name);
            }
        }
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