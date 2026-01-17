use clap::{Parser, Subcommand};
use anyhow::Result;
use crate::connection::ConnectionConfig;
use crate::repl::Repl;
use crate::executor::QueryExecutor;
use rpassword;

#[derive(Parser, Debug)]
#[command(name = "cqlrs")]
#[command(author = "Florian")]
#[command(version = "0.1.0")]
#[command(about = "A fully functional Cassandra CLI client", long_about = None)]
pub struct Cli {
    /// Cassandra host(s) to connect to (comma-separated)
    #[arg(short = 'H', long, default_value = "127.0.0.1")]
    pub hosts: String,

    /// Port to connect to
    #[arg(short, long, default_value = "9042")]
    pub port: u16,

    /// Username for authentication
    #[arg(short, long)]
    pub username: Option<String>,

    /// Prompt for password (secure input without showing in terminal/history)
    #[arg(short = 'P', long)]
    pub password_prompt: bool,

    /// Password for authentication (not recommended, use -p instead)
    #[arg(long)]
    pub password: Option<String>,

    /// Keyspace to use
    #[arg(short, long)]
    pub keyspace: Option<String>,

    /// Execute a single query and exit
    #[arg(short = 'e', long)]
    pub execute: Option<String>,

    /// Execute queries from a file
    #[arg(short, long)]
    pub file: Option<String>,

    /// Output format (table, json, csv)
    #[arg(short, long, default_value = "table")]
    pub output_format: String,

    /// Enable verbose logging
    #[arg(short, long)]
    pub verbose: bool,

    /// Enable SSL/TLS connection
    #[arg(long)]
    pub ssl: bool,

    /// Path to CA certificate file for SSL
    #[arg(long)]
    pub ssl_ca_cert: Option<String>,

    /// Verify SSL certificate (default: false)
    #[arg(long, default_value = "false")]
    pub ssl_verify: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Start interactive REPL mode
    Repl,
    /// Show cluster information
    Describe {
        /// What to describe (cluster, keyspaces, keyspace NAME, table NAME)
        #[arg(required = true)]
        target: Vec<String>,
    },
}

impl Cli {
    pub async fn execute(&self) -> Result<()> {
        // Enable debug logging if verbose
        if self.verbose {
            std::env::set_var("RUST_LOG", "debug");
        }
        
        // Handle password input
        let password = if self.password_prompt {
            if self.username.is_none() {
                eprintln!("Warning: Password prompt specified but no username provided");
                None
            } else {
                print!("Password: ");
                use std::io::Write;
                std::io::stdout().flush()?;
                let pwd = rpassword::read_password()?;
                Some(pwd)
            }
        } else {
            self.password.clone()
        };

        // Build connection config
        let config = ConnectionConfig {
            hosts: self.hosts.split(',').map(|s| s.trim().to_string()).collect(),
            port: self.port,
            username: self.username.clone(),
            password,
            keyspace: self.keyspace.clone(),
            ssl_enabled: self.ssl,
            ssl_ca_cert: self.ssl_ca_cert.clone(),
            ssl_verify: self.ssl_verify,
        };

        // Create executor
        let mut executor = QueryExecutor::new(config).await?;

        // Handle different execution modes
        match &self.command {
            Some(Commands::Repl) | None if self.execute.is_none() && self.file.is_none() => {
                // Start REPL mode
                let mut repl = Repl::new(executor, self.output_format.clone());
                repl.run().await?;
            }
            Some(Commands::Describe { target }) => {
                self.handle_describe(&mut executor, target).await?;
            }
            _ => {
                // Execute single query or file
                if let Some(query) = &self.execute {
                    executor.execute_and_print(query, &self.output_format).await?;
                } else if let Some(file_path) = &self.file {
                    let content = std::fs::read_to_string(file_path)?;
                    for query in content.split(';') {
                        let query = query.trim();
                        if !query.is_empty() {
                            executor.execute_and_print(query, &self.output_format).await?;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    async fn handle_describe(&self, executor: &mut QueryExecutor, target: &[String]) -> Result<()> {
        let query = match target.first().map(|s| s.as_str()) {
            Some("cluster") => {
                "SELECT * FROM system.local".to_string()
            }
            Some("keyspaces") => {
                "SELECT keyspace_name FROM system_schema.keyspaces".to_string()
            }
            Some("keyspace") if target.len() > 1 => {
                format!("SELECT * FROM system_schema.keyspaces WHERE keyspace_name = '{}'", target[1])
            }
            Some("table") if target.len() > 1 => {
                format!("SELECT * FROM system_schema.columns WHERE table_name = '{}'", target[1])
            }
            Some("tables") if target.len() > 1 => {
                format!("SELECT table_name FROM system_schema.tables WHERE keyspace_name = '{}'", target[1])
            }
            _ => {
                println!("Usage: describe [cluster|keyspaces|keyspace NAME|table NAME|tables KEYSPACE]");
                return Ok(());
            }
        };

        executor.execute_and_print(&query, &self.output_format).await?;
        Ok(())
    }
}
