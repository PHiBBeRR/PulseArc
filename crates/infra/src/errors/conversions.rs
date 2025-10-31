//! Conversions from external infrastructure errors into domain errors.

use keyring::Error as KeyringError;
use pulsearc_common::storage::error::StorageError;
use pulsearc_domain::PulseArcError;
use reqwest::Error as HttpError;
use rusqlite::Error as SqlError;

/// Error newtype that keeps conversions on the infrastructure side and can be
/// converted back into the domain error.
#[derive(Debug)]
pub struct InfraError(pub PulseArcError);

impl From<InfraError> for PulseArcError {
    fn from(value: InfraError) -> Self {
        value.0
    }
}

impl From<PulseArcError> for InfraError {
    fn from(value: PulseArcError) -> Self {
        InfraError(value)
    }
}

/// Extension trait to make the conversion logic explicit in tests and within
/// this module.
trait IntoPulseArcError {
    fn into_pulsearc(self) -> PulseArcError;
}

/* -------------------------------------------------------------------------- */
/* rusqlite::Error → PulseArcError */
/* -------------------------------------------------------------------------- */

impl IntoPulseArcError for SqlError {
    fn into_pulsearc(self) -> PulseArcError {
        use rusqlite::ffi::ErrorCode;
        use rusqlite::Error as RE;

        fn looks_like_wrong_key(message: &str) -> bool {
            let lower = message.to_ascii_lowercase();
            lower.contains("not a database") || lower.contains("encrypted")
        }

        match self {
            RE::SqliteFailure(err, maybe_message) => {
                let message = maybe_message.unwrap_or_default();
                match (err.code, err.extended_code) {
                    (ErrorCode::DatabaseBusy, _) => {
                        PulseArcError::Database("database is busy".into())
                    }
                    (ErrorCode::DatabaseLocked, _) => {
                        PulseArcError::Database("database is locked".into())
                    }
                    (ErrorCode::ConstraintViolation, 2067) => {
                        PulseArcError::Database("unique constraint violation".into())
                    }
                    (ErrorCode::ConstraintViolation, 787) => {
                        PulseArcError::Database("foreign key constraint violation".into())
                    }
                    (_, _) if looks_like_wrong_key(&message) => PulseArcError::Security(
                        "SQLCipher key rejected or database not encrypted".into(),
                    ),
                    _ => PulseArcError::Database(format!(
                        "sqlite failure {:?} (code {}): {}",
                        err.code, err.extended_code, message
                    )),
                }
            }
            RE::QueryReturnedNoRows => PulseArcError::NotFound("no rows returned by query".into()),
            RE::FromSqlConversionFailure(_, _, cause) => {
                PulseArcError::Database(format!("failed to convert sqlite value: {cause}"))
            }
            RE::InvalidColumnType(_, _, ty) => {
                PulseArcError::Database(format!("invalid column type: {ty}"))
            }
            RE::Utf8Error(_) => {
                PulseArcError::Database("invalid UTF-8 returned from sqlite".into())
            }
            RE::InvalidParameterName(parameter_name) => {
                PulseArcError::Database(format!("invalid parameter name: {parameter_name}"))
            }
            RE::InvalidPath(path) => PulseArcError::Database(format!(
                "invalid database path: {}",
                path.to_string_lossy()
            )),
            RE::InvalidQuery => PulseArcError::Database("invalid SQL query".into()),
            other => PulseArcError::Database(other.to_string()),
        }
    }
}

impl From<SqlError> for InfraError {
    fn from(value: SqlError) -> Self {
        InfraError(value.into_pulsearc())
    }
}

/* -------------------------------------------------------------------------- */
/* keyring::Error → PulseArcError */
/* -------------------------------------------------------------------------- */

impl IntoPulseArcError for KeyringError {
    fn into_pulsearc(self) -> PulseArcError {
        use KeyringError::*;

        let description = self.to_string();

        match self {
            NoEntry => PulseArcError::NotFound("keychain entry not found".into()),
            BadEncoding(_) => {
                PulseArcError::Security("credential in keychain is not valid UTF-8".into())
            }
            TooLong(name, limit) => PulseArcError::Security(format!(
                "keychain attribute '{name}' exceeds platform limit ({limit})"
            )),
            Invalid(attr, reason) => {
                PulseArcError::Security(format!("keychain attribute '{attr}' is invalid: {reason}"))
            }
            Ambiguous(entries) => PulseArcError::Security(format!(
                "multiple keychain entries matched request ({} results)",
                entries.len()
            )),
            PlatformFailure(err) => {
                PulseArcError::Security(format!("keychain platform error: {err}"))
            }
            NoStorageAccess(err) => {
                PulseArcError::Security(format!("unable to access secure storage: {err}"))
            }
            _ => PulseArcError::Security(description),
        }
    }
}

impl From<KeyringError> for InfraError {
    fn from(value: KeyringError) -> Self {
        InfraError(value.into_pulsearc())
    }
}

/* -------------------------------------------------------------------------- */
/* reqwest::Error → PulseArcError */
/* -------------------------------------------------------------------------- */

impl IntoPulseArcError for HttpError {
    fn into_pulsearc(self) -> PulseArcError {
        if self.is_timeout() {
            return PulseArcError::Network("HTTP request timed out".into());
        }

        #[cfg(not(target_arch = "wasm32"))]
        if self.is_connect() {
            return PulseArcError::Network("HTTP connection failure".into());
        }

        if let Some(status) = self.status() {
            let code = status.as_u16();
            let message =
                format!("HTTP {} {}", code, status.canonical_reason().unwrap_or("unknown status"));

            return match code {
                401 | 403 => PulseArcError::Auth(message),
                404 => PulseArcError::NotFound(message),
                429 => PulseArcError::Network(message),
                400..=499 => PulseArcError::InvalidInput(message),
                500..=599 => PulseArcError::Network(message),
                _ => PulseArcError::Network(message),
            };
        }

        PulseArcError::Network(self.to_string())
    }
}

impl From<HttpError> for InfraError {
    fn from(value: HttpError) -> Self {
        InfraError(value.into_pulsearc())
    }
}

/* -------------------------------------------------------------------------- */
/* StorageError → PulseArcError */
/* -------------------------------------------------------------------------- */

impl IntoPulseArcError for StorageError {
    fn into_pulsearc(self) -> PulseArcError {
        match self {
            StorageError::Connection(msg) => {
                PulseArcError::Database(format!("connection error: {}", msg))
            }
            StorageError::Query(msg) => PulseArcError::Database(format!("query error: {}", msg)),
            StorageError::DatabaseError(msg) => {
                PulseArcError::Database(format!("database error: {}", msg))
            }
            StorageError::Encryption(msg) => {
                PulseArcError::Security(format!("encryption error: {}", msg))
            }
            StorageError::Migration(msg) => {
                PulseArcError::Database(format!("migration error: {}", msg))
            }
            StorageError::Keychain(msg) => {
                PulseArcError::Security(format!("keychain error: {}", msg))
            }
            StorageError::WrongKeyOrNotEncrypted => {
                PulseArcError::Security("wrong encryption key or database not encrypted".into())
            }
            StorageError::PoolExhausted => {
                PulseArcError::Database("connection pool exhausted".into())
            }
            StorageError::Timeout(seconds) => {
                PulseArcError::Database(format!("database timeout after {}s", seconds))
            }
            StorageError::InvalidConfig(msg) => {
                PulseArcError::Config(format!("invalid config: {}", msg))
            }
            StorageError::SchemaVersionMismatch { expected, found } => PulseArcError::Database(
                format!("schema version mismatch (expected {}, found {})", expected, found),
            ),
            StorageError::Common(common_err) => PulseArcError::Database(common_err.to_string()),
            StorageError::Io(io_err) => PulseArcError::Database(format!("IO error: {}", io_err)),
            StorageError::Rusqlite(sql_err) => sql_err.into_pulsearc(),
            StorageError::R2d2(r2d2_err) => {
                PulseArcError::Database(format!("connection pool error: {}", r2d2_err))
            }
            StorageError::SerdeJson(json_err) => {
                PulseArcError::Database(format!("JSON error: {}", json_err))
            }
        }
    }
}

impl From<StorageError> for InfraError {
    fn from(value: StorageError) -> Self {
        InfraError(value.into_pulsearc())
    }
}

/* -------------------------------------------------------------------------- */
/* Tests */
/* -------------------------------------------------------------------------- */

#[cfg(test)]
mod tests {
    use reqwest::{Client, StatusCode};
    use rusqlite::ffi::{Error as FfiError, ErrorCode};
    use rusqlite::Error as SqlError;
    use tokio::runtime::Runtime;
    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;

    #[test]
    fn sqlite_busy_maps_to_database_error() {
        let err = SqlError::SqliteFailure(
            FfiError { code: ErrorCode::DatabaseBusy, extended_code: 5 },
            Some("database is locked".into()),
        );

        let mapped: PulseArcError = InfraError::from(err).into();
        match mapped {
            PulseArcError::Database(msg) => {
                assert!(msg.contains("busy") || msg.contains("locked"));
            }
            other => panic!("expected database error, got {:?}", other),
        }
    }

    #[test]
    fn keyring_no_entry_maps_to_not_found() {
        let err = KeyringError::NoEntry;
        let mapped: PulseArcError = InfraError::from(err).into();
        match mapped {
            PulseArcError::NotFound(msg) => assert!(msg.contains("keychain")),
            other => panic!("expected not found, got {:?}", other),
        }
    }

    #[test]
    fn http_status_401_maps_to_auth_error() {
        Runtime::new().unwrap().block_on(async {
            let server = MockServer::start().await;
            Mock::given(method("GET"))
                .respond_with(ResponseTemplate::new(StatusCode::UNAUTHORIZED))
                .mount(&server)
                .await;

            let client = Client::builder().no_proxy().build().unwrap();
            let error =
                client.get(server.uri()).send().await.unwrap().error_for_status().unwrap_err();

            let mapped: PulseArcError = InfraError::from(error).into();
            match mapped {
                PulseArcError::Auth(msg) => assert!(msg.contains("401")),
                other => panic!("expected auth error, got {:?}", other),
            }
        });
    }
}
