use crate::errors::ClickhouseError;
use crate::config::SETTINGS;
use clickhouse::Client;
use std::sync::Arc;
use tokio::sync::{Mutex, Semaphore};
use std::time::Instant;
use log::debug;

pub struct PooledConnection {
    pub client: Option<Client>,
    in_use: bool,
    last_used: Instant,
}

pub struct ConnectionPool {
    connections: Vec<Arc<Mutex<PooledConnection>>>,
    pool_size: usize,
    semaphore: Arc<Semaphore>,
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
            semaphore: Arc::new(Semaphore::new(pool_size)),
        }
    }

    pub async fn get_connection(&self) -> Result<Arc<Mutex<PooledConnection>>, ClickhouseError> {
        debug!("Attempting to get connection from pool");
        
        let _permit = self.semaphore.acquire().await.map_err(|e| 
            ClickhouseError::ConnectionError(format!("Failed to acquire semaphore: {}", e))
        )?;
        
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
                
                match self.try_create_connection().await {
                    Ok(client) => {
                        conn_guard.client = Some(client);
                        conn_guard.in_use = true;
                        conn_guard.last_used = Instant::now();
                        debug!("Created new client in existing slot");
                        return Ok(conn.clone());
                    }
                    Err(e) => {
                        self.semaphore.add_permits(1);
                        return Err(e);
                    }
                }
            }
        }
        
        self.semaphore.add_permits(1);
        Err(ClickhouseError::ConnectionError("All pool slots are occupied".into()))
    }

    pub async fn release_connection(&self, conn: Arc<Mutex<PooledConnection>>) {
        let mut conn_guard = conn.lock().await;
        conn_guard.in_use = false;
        conn_guard.last_used = Instant::now();
        debug!("Connection released back to pool");
        self.semaphore.add_permits(1);
    }

    pub fn available_connections(&self) -> usize {
        self.semaphore.available_permits()
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