use scylla::{Session, SessionBuilder};
use crate::error::{CqlError, CqlResult};
use tracing::info;
use openssl::ssl::{SslContext, SslMethod, SslVerifyMode};

#[derive(Debug, Clone)]
pub struct ConnectionConfig {
    pub hosts: Vec<String>,
    pub port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
    pub keyspace: Option<String>,
    pub ssl_enabled: bool,
    pub ssl_ca_cert: Option<String>,
    pub ssl_verify: bool,
}

pub struct ConnectionManager {
    session: Session,
    config: ConnectionConfig,
}

impl ConnectionManager {
    /// Create SSL context with configurable certificate verification
    fn create_ssl_context(verify_cert: bool) -> CqlResult<SslContext> {
        let mut ssl_builder = SslContext::builder(SslMethod::tls())
            .map_err(|e| CqlError::ConnectionError(format!("Failed to create SSL context: {}", e)))?;

        if verify_cert {
            info!("SSL certificate verification enabled (SslVerifyMode::PEER)");
            ssl_builder.set_verify(SslVerifyMode::PEER);
        } else {
            info!("SSL certificate verification disabled (SslVerifyMode::NONE)");
            ssl_builder.set_verify(SslVerifyMode::NONE);
        }

        Ok(ssl_builder.build())
    }

    pub async fn connect(config: ConnectionConfig) -> CqlResult<Self> {
        info!("Connecting to Cassandra cluster at {:?}:{}", config.hosts, config.port);
        
        // Build contact points with port
        let contact_points: Vec<String> = config.hosts.iter()
            .map(|host| {
                if host.contains(':') {
                    host.clone()
                } else {
                    format!("{}:{}", host, config.port)
                }
            })
            .collect();
        
        info!("Contact points: {:?}", contact_points);
        
        let mut builder = SessionBuilder::new()
            .known_nodes(&contact_points);

        // Add authentication if provided
        if let (Some(username), Some(password)) = (&config.username, &config.password) {
            info!("Using authentication with username: {}", username);
            builder = builder.user(username, password);
        }

        // Configure SSL/TLS if enabled
        if config.ssl_enabled {
            info!("SSL/TLS enabled with verification: {}", config.ssl_verify);
            
            // Create custom SSL context with configurable verification
            let ssl_context = Self::create_ssl_context(config.ssl_verify)?;
            
            // Apply SSL context to session builder
            builder = builder.ssl_context(Some(ssl_context));
            
            if let Some(ref ca_cert) = config.ssl_ca_cert {
                info!("CA certificate path specified: {}", ca_cert);
                // Note: If using custom CA cert, it should be loaded into the SslContext
                eprintln!("Info: Custom CA certificate loading can be added to create_ssl_context()");
            }
        }

        // Build session
        info!("Building session...");
        let session = builder.build().await
            .map_err(|e| {
                let error_msg = format!(
                    "Failed to connect to Cassandra at {:?}\n\nPossible causes:\n\
                    1. Cassandra is not running\n\
                    2. Wrong host/port (current: {:?})\n\
                    3. SSL/TLS mismatch (SSL enabled: {})\n\
                    4. Firewall blocking connection\n\
                    5. Authentication required but not provided\n\n\
                    Original error: {}",
                    contact_points, contact_points, config.ssl_enabled, e
                );
                CqlError::ConnectionError(error_msg)
            })?;

        // Use keyspace if specified
        if let Some(keyspace) = &config.keyspace {
            info!("Using keyspace: {}", keyspace);
            session.use_keyspace(keyspace, false).await
                .map_err(|e| CqlError::ConnectionError(format!("Failed to use keyspace: {}", e)))?;
        }

        info!("Successfully connected to Cassandra");
        
        Ok(Self { session, config })
    }

    pub fn session(&self) -> &Session {
        &self.session
    }

    pub fn config(&self) -> &ConnectionConfig {
        &self.config
    }

    pub async fn use_keyspace(&self, keyspace: &str) -> CqlResult<()> {
        self.session.use_keyspace(keyspace, false).await
            .map_err(|e| CqlError::ConnectionError(format!("Failed to use keyspace: {}", e)))?;
        Ok(())
    }
}
