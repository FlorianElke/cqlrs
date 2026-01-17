use thiserror::Error;

#[derive(Error, Debug)]
pub enum CqlError {
    #[error("Connection error: {0}")]
    ConnectionError(String),
    
    #[error("Query execution error: {0}")]
    QueryError(String),
    
    #[error("Invalid query: {0}")]
    InvalidQuery(String),
    
    #[error("Configuration error: {0}")]
    ConfigError(String),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Scylla error: {0}")]
    ScyllaError(#[from] scylla::transport::errors::QueryError),
    
    #[error("New session error: {0}")]
    NewSessionError(#[from] scylla::transport::errors::NewSessionError),
}

pub type CqlResult<T> = Result<T, CqlError>;
