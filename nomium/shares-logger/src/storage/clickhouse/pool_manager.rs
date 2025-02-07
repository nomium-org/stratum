use crate::errors::ClickhouseError;
use crate::config::SETTINGS;
use clickhouse::Client;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::time::Duration;
use std::time::Instant;
use log::error;
use log::debug;

pub struct PooledConnection {
    pub client: Option<Client>,
    in_use: bool,
    last_used: Instant,
}

pub struct ConnectionPool {
    connections: Vec<Arc<Mutex<PooledConnection>>>,
    pool_size: usize,
}

impl ConnectionPool {
    pub fn new(pool_size: usize) -> Self {
        debug!("Initializing connection pool with size {}", pool_size);
        let connections = (0..pool_size)
            .map(|_| Arc::new(Mutex::new(PooledConnection {
                client: None,
                in_use: false,
                last_used: Instant::now(),
            })))
            .collect();
        Self {
            connections,
            pool_size,
        }
    }

    pub async fn get_connection(&self) -> Result<Arc<Mutex<PooledConnection>>, ClickhouseError> {
        debug!("Attempting to get connection from pool");
        
        for conn in &self.connections {
            let mut conn_guard = conn.lock().await;
            if !conn_guard.in_use {
                if let Some(client) = &conn_guard.client {
                    if Self::is_connection_alive(client).await {
                        conn_guard.in_use = true;
                        debug!("Found and reserved alive connection from pool");
                        return Ok(conn.clone());
                    }
                }
            }
        }
        
        let new_conn = self.create_new_connection().await?;
        let mut conn_guard = new_conn.lock().await;
        conn_guard.in_use = true;
        drop(conn_guard);
        Ok(new_conn)
    }

    pub async fn release_connection(&self, conn: Arc<Mutex<PooledConnection>>) {
        let mut conn_guard = conn.lock().await;
        conn_guard.in_use = false;
        conn_guard.last_used = Instant::now();
        debug!("Connection released back to pool");
    }

    async fn create_new_connection(&self) -> Result<Arc<Mutex<PooledConnection>>, ClickhouseError> {
        debug!("Starting new connection creation process");
        let mut delay = Duration::from_millis(SETTINGS.clickhouse.base_retry_delay_ms);
        let max_delay = Duration::from_secs(SETTINGS.clickhouse.max_retry_delay_secs);
        let mut attempts = 0;
        
        while attempts < SETTINGS.clickhouse.max_connection_retries {
            match self.try_create_connection().await {
                Ok(client) => {
                    debug!("Successfully created new connection after {} attempts", attempts + 1);
                    let conn = Arc::new(Mutex::new(PooledConnection {
                        client: Some(client),
                        in_use: false,
                        last_used: Instant::now(),
                    }));
                    return Ok(conn);
                }
                Err(e) => {
                    error!("Failed to create connection: {}", e);
                    debug!("Retrying connection in {}ms (attempt {}/{})", 
                        delay.as_millis(), attempts + 1, SETTINGS.clickhouse.max_connection_retries);
                    tokio::time::sleep(delay).await;
                    delay = std::cmp::min(delay * 2, max_delay);
                    attempts += 1;
                }
            }
        }
        Err(ClickhouseError::ConnectionError("Max retries exceeded".into()))
    }

    async fn try_create_connection(&self) -> Result<Client, ClickhouseError> {
        debug!("Attempting to create connection to {}", SETTINGS.clickhouse.url);
        let client = Client::default()
            .with_url(&SETTINGS.clickhouse.url)
            .with_database(&SETTINGS.clickhouse.database)
            .with_user(&SETTINGS.clickhouse.username)
            .with_password(&SETTINGS.clickhouse.password);
        
        match Self::is_connection_alive(&client).await {
            true => Ok(client),
            false => Err(ClickhouseError::ConnectionError("Client connection check failed".into())),
        }
    }

    async fn is_connection_alive(client: &Client) -> bool {
        debug!("Checking if connection is alive");
        client.query("SELECT 1").execute().await.is_ok()
    }
}