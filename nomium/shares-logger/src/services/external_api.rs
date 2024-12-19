use reqwest::Client;
use serde::{Deserialize, Serialize};
use log::info;

#[derive(Debug, Serialize)]
pub struct WorkerAuthenticationRequest {
    accountName: String,
    workerNumber: u32,
}

#[derive(Debug, Deserialize)]
pub struct WorkerAuthenticationResponse {
    pub isSuccess: bool,
    pub workerId: String,
    pub workerName: String,
    pub userId: String,
    pub accountId: String,
}

#[derive(Clone)]
pub struct ExternalApiService {
    client: Client,
    api_key: String,
    base_url: String,
}

impl ExternalApiService {
    pub fn new(api_key: &str, base_url: &str) -> Self {
        Self {
            client: Client::new(),
            api_key: api_key.to_string(),
            base_url: base_url.to_string(),
        }
    }

    pub async fn authenticate_worker(
        &self,
        account_name: &str,
        worker_number: u32,
    ) -> Result<WorkerAuthenticationResponse, Box<dyn std::error::Error + Send + Sync>> {
        let request_body = WorkerAuthenticationRequest {
            accountName: account_name.to_string(),
            workerNumber: worker_number,
        };

        let response = self
            .client
            .post(format!("{}/worker-authentication", self.base_url))
            .header("accept", "text/plain")
            .header("X-Api-Key", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        if response.status().is_success() {
            let response_body = response.json::<WorkerAuthenticationResponse>().await?;
            Ok(response_body)
        } else {
            let status = response.status();
            let error_message = response.text().await?;
            Err(format!("HTTP {}: {}", status, error_message).into())
        }
    }
}