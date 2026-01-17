use scylla::query::Query;
use scylla::transport::query_result::QueryResult;
use crate::connection::{ConnectionConfig, ConnectionManager};
use crate::error::{CqlError, CqlResult};
use crate::formatter::{format_result, OutputFormat};
use tracing::{info, error};

pub struct QueryExecutor {
    connection: ConnectionManager,
}

impl QueryExecutor {
    pub async fn new(config: ConnectionConfig) -> CqlResult<Self> {
        let connection = ConnectionManager::connect(config).await?;
        Ok(Self { connection })
    }

    pub async fn execute(&self, query_str: &str) -> CqlResult<QueryResult> {
        info!("Executing query: {}", query_str.trim());
        
        let query = Query::new(query_str);
        
        let result = self.connection.session()
            .query(query, &[])
            .await
            .map_err(|e| {
                error!("Query execution failed: {}", e);
                CqlError::QueryError(format!("{}", e))
            })?;

        Ok(result)
    }

    pub async fn execute_and_print(&mut self, query_str: &str, format: &str) -> CqlResult<()> {
        let query_trimmed = query_str.trim();
        
        // Handle USE keyspace command specially
        if query_trimmed.to_lowercase().starts_with("use ") {
            let keyspace = query_trimmed[4..].trim().trim_matches(';').trim();
            self.connection.use_keyspace(keyspace).await?;
            println!("Now using keyspace: {}", keyspace);
            return Ok(());
        }

        // Handle empty queries
        if query_trimmed.is_empty() {
            return Ok(());
        }

        let result = self.execute(query_str).await?;
        
        let output_format = match format.to_lowercase().as_str() {
            "json" => OutputFormat::Json,
            "csv" => OutputFormat::Csv,
            _ => OutputFormat::Table,
        };

        let formatted = format_result(&result, output_format)?;
        println!("{}", formatted);

        Ok(())
    }

    pub fn connection(&self) -> &ConnectionManager {
        &self.connection
    }
}
