//! Database connection and query execution.

use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};

/// Maximum number of retries for transient failures.
const MAX_RETRIES: u32 = 3;

/// Errors returned by database operations.
#[derive(Debug, Clone)]
pub enum DatabaseError {
    /// Failed to establish a connection to the database.
    ConnectionFailed(String),
    /// A query failed to execute.
    QueryFailed { query: String, reason: String },
    /// The connection pool is exhausted.
    PoolExhausted,
    /// A transaction conflict occurred (retryable).
    TransactionConflict,
}

impl fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DatabaseError::ConnectionFailed(msg) => write!(f, "connection failed: {}", msg),
            DatabaseError::QueryFailed { query, reason } => {
                write!(f, "query '{}' failed: {}", query, reason)
            }
            DatabaseError::PoolExhausted => write!(f, "connection pool exhausted"),
            DatabaseError::TransactionConflict => write!(f, "transaction conflict, retry"),
        }
    }
}

/// A database connection that can execute queries.
pub struct Connection {
    url: String,
    connected: AtomicBool,
    max_retries: u32,
}

impl Connection {
    /// Open a new connection to the database at the given URL.
    pub fn new(url: &str) -> Result<Self, DatabaseError> {
        if url.is_empty() {
            return Err(DatabaseError::ConnectionFailed("empty URL".into()));
        }

        Ok(Self {
            url: url.to_string(),
            connected: AtomicBool::new(true),
            max_retries: MAX_RETRIES,
        })
    }

    /// Check whether the connection is still alive.
    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Relaxed)
    }

    /// Execute a SQL query and return the result rows as strings.
    pub fn query(&self, sql: &str) -> Result<Vec<String>, DatabaseError> {
        if !self.is_connected() {
            return Err(DatabaseError::ConnectionFailed(
                "not connected".into(),
            ));
        }

        if sql.trim().is_empty() {
            return Err(DatabaseError::QueryFailed {
                query: sql.into(),
                reason: "empty query".into(),
            });
        }

        // Simulated query result for fixture purposes.
        Ok(vec![format!("row from: {}", sql)])
    }

    /// Execute a query within a transaction, retrying on conflicts.
    pub fn execute_in_transaction<F, T>(
        &self,
        operation: F,
    ) -> Result<T, DatabaseError>
    where
        F: Fn(&Self) -> Result<T, DatabaseError>,
    {
        let mut attempts = 0;
        loop {
            match operation(self) {
                Ok(result) => return Ok(result),
                Err(DatabaseError::TransactionConflict) if attempts < self.max_retries => {
                    attempts += 1;
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// Close the connection.
    pub fn close(&self) {
        self.connected.store(false, Ordering::Relaxed);
    }

    /// Return the connection URL (with credentials redacted).
    pub fn url_redacted(&self) -> String {
        // Naive redaction: replace password in URL.
        self.url
            .split('@')
            .last()
            .unwrap_or(&self.url)
            .to_string()
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        self.close();
    }
}
