use crate::errors::ClickhouseError;
use crate::config::SETTINGS;
use clickhouse::Client;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::time::Duration;
use log::error;
use log::debug;

const MAX_RETRIES: u32 = 100500;
const BASE_DELAY: Duration = Duration::from_millis(100);
const MAX_DELAY: Duration = Duration::from_secs(1);

pub struct ConnectionPool {
    connections: Vec<Arc<Mutex<Option<Client>>>>,
    pool_size: usize,
}

impl ConnectionPool {
    pub fn new(pool_size: usize) -> Self {
        debug!("Initializing connection pool with size {}", pool_size);
        let connections = (0..pool_size)
            .map(|_| Arc::new(Mutex::new(None)))
            .collect();
        Self {
            connections,
            pool_size,
        }
    }

    pub async fn get_connection(&self) -> Result<Arc<Mutex<Option<Client>>>, ClickhouseError> {
        debug!("Attempting to get connection from pool");
        for conn in &self.connections {
            if let Some(client) = &*conn.lock().await {
                if Self::is_connection_alive(client).await {
                    debug!("Found alive connection in pool");
                    return Ok(conn.clone());
                }
            }
        }
        self.create_new_connection().await
    }

    async fn create_new_connection(&self) -> Result<Arc<Mutex<Option<Client>>>, ClickhouseError> {
        debug!("Starting new connection creation process");
        let mut delay = BASE_DELAY;
        let mut attempts = 0;

        while attempts < MAX_RETRIES {
            match self.try_create_connection().await {
                Ok(client) => {
                    debug!("Successfully created new connection after {} attempts", attempts + 1);
                    let conn = Arc::new(Mutex::new(Some(client)));
                    return Ok(conn);
                }
                Err(e) => {
                    error!("Failed to create connection: {}", e);
                    debug!("Retrying connection in {}ms (attempt {}/{})", 
                        delay.as_millis(), attempts + 1, MAX_RETRIES);
                    tokio::time::sleep(delay).await;
                    delay = std::cmp::min(delay * 2, MAX_DELAY);
                    attempts += 1;
                }
            }
        }
        Err(ClickhouseError::ConnectionError("Max retries exceeded".into()))
    }

    async fn try_create_connection(&self) -> Result<Client, ClickhouseError> {
        debug!("Attempting to create connection to {}", SETTINGS.clickhouse.url);
        Ok(Client::default()
            .with_url(&SETTINGS.clickhouse.url)
            .with_database(&SETTINGS.clickhouse.database)
            .with_user(&SETTINGS.clickhouse.username)
            .with_password(&SETTINGS.clickhouse.password))
    }

    async fn is_connection_alive(client: &Client) -> bool {
        debug!("Checking if connection is alive");
        client.query("SELECT 1").execute().await.is_ok()
    }
}