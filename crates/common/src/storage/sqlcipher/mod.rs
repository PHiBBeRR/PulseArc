//! SQLCipher backend implementation
//!
//! Provides an r2d2-based connection pool for SQLCipher encrypted databases.

pub mod cipher;
pub mod config;
pub mod connection;
pub mod pool;
pub mod pragmas;

pub use cipher::{configure_sqlcipher, verify_encryption, SqlCipherConfig};
pub use config::SqlCipherPoolConfig;
pub use connection::SqlCipherConnection;
pub use pool::SqlCipherPool;
pub use pragmas::apply_connection_pragmas;
