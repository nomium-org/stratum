use crate::errors::ClickhouseError;
use crate::config::SETTINGS;
use clickhouse::Client;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::time::{Duration, Instant};
use log::{info, error};

const MAX_RETRIES: u32 = 5;
const BASE_DELAY: Duration = Duration::from_millis(100);
const MAX_DELAY: Duration = Duration::from_secs(30);

pub struct ConnectionPool {
    connections: Vec<Arc<Mutex<Option<Client>>>>,
    pool_size: usize,
}

impl ConnectionPool {
    pub fn new(pool_size: usize) -> Self {
        let connections = (0..pool_size)
            .map(|_| Arc::new(Mutex::new(None)))
            .collect();
        Self {
            connections,
            pool_size,
        }
    }

    pub async fn get_connection(&self) -> Result<Arc<Mutex<Option<Client>>>, ClickhouseError> {
        for conn in &self.connections {
            if let Some(client) = &*conn.lock().await {
                if Self::is_connection_alive(client).await {
                    return Ok(conn.clone());
                }
            }
        }
        self.create_new_connection().await
    }

    async fn create_new_connection(&self) -> Result<Arc<Mutex<Option<Client>>>, ClickhouseError> {
        let mut delay = BASE_DELAY;
        let mut attempts = 0;

        while attempts < MAX_RETRIES {
            match self.try_create_connection().await {
                Ok(client) => {
                    let conn = Arc::new(Mutex::new(Some(client)));
                    return Ok(conn);
                }
                Err(e) => {
                    error!("Failed to create connection: {}", e);
                    tokio::time::sleep(delay).await;
                    delay = std::cmp::min(delay * 2, MAX_DELAY);
                    attempts += 1;
                }
            }
        }
        Err(ClickhouseError::ConnectionError("Max retries exceeded".into()))
    }

    async fn try_create_connection(&self) -> Result<Client, ClickhouseError> {
        Ok(Client::default()
            .with_url(&SETTINGS.clickhouse.url)
            .with_database(&SETTINGS.clickhouse.database)
            .with_user(&SETTINGS.clickhouse.username)
            .with_password(&SETTINGS.clickhouse.password))
    }

    async fn is_connection_alive(client: &Client) -> bool {
        client.query("SELECT 1").execute().await.is_ok()
    }
}