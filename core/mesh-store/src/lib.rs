//! SQLite persistence, schema migration, and invariant enforcement.

#![forbid(unsafe_code)]

use core::fmt;

use rusqlite::{Connection, OptionalExtension};

pub const CRATE_NAME: &str = "mesh-store";
pub const SCHEMA_VERSION: i64 = 1;
pub const SCHEMA_V1_SQL: &str = include_str!("../../../schemas/sqlite_v1.sql");
pub const SCHEMA_INVARIANTS_SQL: &str = include_str!("../../../schemas/schema_invariants.sql");

mod contact_store;
mod health;
mod identity_store;
pub use contact_store::*;
pub use health::*;
mod routing_store;
mod transfer_store;
pub use identity_store::*;
pub use routing_store::*;
pub use transfer_store::*;

#[derive(Debug, Eq, PartialEq)]
pub enum StoreError {
    Sqlite(String),
    NewerSchema { found: i64, supported: i64 },
    SchemaVersionMismatch { pragma: i64, metadata: i64 },
    InvariantViolation { statement: usize },
    WaitOnly,
    InvalidGrantTransition,
    UnknownGrant,
    TokenOverflow,
    IntegerOutOfRange,
    Crypto(String),
    KeyMaterialMismatch,
    ContactNotFound,
    BundleNotFound,
    TransferNotFound,
    TransferConflict,
    PartialQuotaExceeded,
    IncompleteTransfer,
    TransferHashMismatch,
    Io(String),
}

impl fmt::Display for StoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "storage error: {self:?}")
    }
}

impl std::error::Error for StoreError {}

impl From<rusqlite::Error> for StoreError {
    fn from(value: rusqlite::Error) -> Self {
        Self::Sqlite(value.to_string())
    }
}

impl From<mesh_crypto::CryptoError> for StoreError {
    fn from(value: mesh_crypto::CryptoError) -> Self {
        Self::Crypto(value.to_string())
    }
}

pub struct Store {
    connection: Connection,
}

impl Store {
    pub fn open(path: impl AsRef<std::path::Path>) -> Result<Self, StoreError> {
        let connection = Connection::open(path)?;
        migrate(&connection)?;
        Ok(Self { connection })
    }

    pub fn open_in_memory() -> Result<Self, StoreError> {
        let connection = Connection::open_in_memory()?;
        migrate(&connection)?;
        Ok(Self { connection })
    }

    #[must_use]
    pub const fn connection(&self) -> &Connection {
        &self.connection
    }

    pub fn validate_invariants(&self) -> Result<(), StoreError> {
        validate_schema_invariants(&self.connection)
    }
}

pub fn migrate(connection: &Connection) -> Result<(), StoreError> {
    connection.pragma_update(None, "foreign_keys", true)?;
    let current: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if current > SCHEMA_VERSION {
        return Err(StoreError::NewerSchema {
            found: current,
            supported: SCHEMA_VERSION,
        });
    }
    if current == 0 {
        connection.execute_batch(SCHEMA_V1_SQL)?;
    }

    let pragma: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
    let metadata: Option<String> = connection
        .query_row(
            "SELECT value FROM schema_meta WHERE key = 'schema_version'",
            [],
            |row| row.get(0),
        )
        .optional()?;
    let metadata = metadata.and_then(|value| value.parse::<i64>().ok()).ok_or(
        StoreError::SchemaVersionMismatch {
            pragma,
            metadata: -1,
        },
    )?;
    if pragma != metadata {
        return Err(StoreError::SchemaVersionMismatch { pragma, metadata });
    }
    validate_schema_invariants(connection)
}

pub fn validate_schema_invariants(connection: &Connection) -> Result<(), StoreError> {
    let without_comments = SCHEMA_INVARIANTS_SQL
        .lines()
        .map(|line| line.split_once("--").map_or(line, |(before, _)| before))
        .collect::<Vec<_>>()
        .join("\n");
    for (index, statement) in without_comments
        .split(';')
        .map(str::trim)
        .filter(|statement| !statement.is_empty())
        .enumerate()
    {
        let mut prepared = connection.prepare(statement)?;
        if prepared.query([])?.next()?.is_some() {
            return Err(StoreError::InvariantViolation {
                statement: index + 1,
            });
        }
    }
    Ok(())
}

#[must_use]
pub const fn bundle_boundary() -> &'static str {
    mesh_bundle::CRATE_NAME
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_v1_executes_and_invariants_are_empty() {
        let store = Store::open_in_memory().unwrap();
        assert_eq!(
            store
                .connection()
                .pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
                .unwrap(),
            1
        );
        store.validate_invariants().unwrap();
    }

    #[test]
    fn migration_is_forward_only_and_idempotent() {
        let connection = Connection::open_in_memory().unwrap();
        migrate(&connection).unwrap();
        migrate(&connection).unwrap();
        connection.pragma_update(None, "user_version", 2).unwrap();
        assert_eq!(
            migrate(&connection),
            Err(StoreError::NewerSchema {
                found: 2,
                supported: 1,
            })
        );
    }

    #[test]
    fn pragma_and_metadata_mismatch_fails_closed() {
        let connection = Connection::open_in_memory().unwrap();
        migrate(&connection).unwrap();
        connection
            .execute(
                "UPDATE schema_meta SET value = '0' WHERE key = 'schema_version'",
                [],
            )
            .unwrap();
        assert_eq!(
            migrate(&connection),
            Err(StoreError::SchemaVersionMismatch {
                pragma: 1,
                metadata: 0,
            })
        );
    }

    #[test]
    fn sqlite_checks_reject_invalid_persisted_state() {
        let store = Store::open_in_memory().unwrap();
        let result = store.connection().execute(
            "INSERT INTO bundles (
                packet_id, bp_identity_hash, destination_slot, random_source_id,
                creation_sequence, message_class_hint, priority, lifetime_ms,
                stored_age_ms, age_anchor_elapsed_ms, received_boot_id,
                hop_count, hop_limit, copy_tokens, payload_size, payload_sha256,
                wire_sha256, state, origin, created_local_ms
             ) VALUES (
                zeroblob(16), zeroblob(32), zeroblob(16), zeroblob(16),
                zeroblob(8), 1, 0, 60000, 0, 0, zeroblob(16),
                0, 1, 0, 1, zeroblob(32), zeroblob(32), 0, 0, 0
             )",
            [],
        );
        assert!(result.is_err());
    }
}
