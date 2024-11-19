use std::fmt;
use std::error::Error;

#[derive(Debug)]
pub enum ClickhouseError {
    ConnectionError(String),
    QueryError(String),
    BatchInsertError(String),
    TableCreationError(String),
}

impl fmt::Display for ClickhouseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClickhouseError::ConnectionError(msg) => write!(f, "ClickHouse connection error: {}", msg),
            ClickhouseError::QueryError(msg) => write!(f, "ClickHouse query error: {}", msg),
            ClickhouseError::BatchInsertError(msg) => write!(f, "Batch insert error: {}", msg),
            ClickhouseError::TableCreationError(msg) => write!(f, "Table creation error: {}", msg),
        }
    }
}

impl Error for ClickhouseError {}

impl From<clickhouse::error::Error> for ClickhouseError {
    fn from(err: clickhouse::error::Error) -> Self {
        ClickhouseError::QueryError(err.to_string())
    }
}