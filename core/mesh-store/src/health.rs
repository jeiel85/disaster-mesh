//! Read-only database startup diagnostics. Corrupt data is never silently reset.

use std::path::Path;

use rusqlite::{Connection, OpenFlags, OptionalExtension};

use crate::SCHEMA_VERSION;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DatabaseHealth {
    Healthy,
    Corrupt,
    NewerSchema,
    SchemaMismatch,
}

#[must_use]
pub fn inspect_database(path: impl AsRef<Path>) -> DatabaseHealth {
    inspect(path.as_ref()).unwrap_or(DatabaseHealth::Corrupt)
}

fn inspect(path: &Path) -> rusqlite::Result<DatabaseHealth> {
    let connection = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    let quick: String = connection.pragma_query_value(None, "quick_check", |row| row.get(0))?;
    if quick != "ok" {
        return Ok(DatabaseHealth::Corrupt);
    }
    let foreign_key_error = connection
        .prepare("PRAGMA foreign_key_check")?
        .query([])?
        .next()?
        .is_some();
    if foreign_key_error {
        return Ok(DatabaseHealth::Corrupt);
    }
    let pragma: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if pragma > SCHEMA_VERSION {
        return Ok(DatabaseHealth::NewerSchema);
    }
    let metadata: Option<String> = connection
        .query_row(
            "SELECT value FROM schema_meta WHERE key = 'schema_version'",
            [],
            |row| row.get(0),
        )
        .optional()?;
    if metadata
        .as_deref()
        .and_then(|value| value.parse::<i64>().ok())
        != Some(pragma)
    {
        return Ok(DatabaseHealth::SchemaMismatch);
    }
    Ok(DatabaseHealth::Healthy)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::Store;

    use super::*;

    fn path(label: &str) -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "dm-health-{label}-{}-{nonce}.db",
            std::process::id()
        ))
    }

    #[test]
    fn healthy_database_is_recognized_without_mutation() {
        let path = path("ok");
        drop(Store::open(&path).unwrap());
        assert_eq!(inspect_database(&path), DatabaseHealth::Healthy);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn corrupt_database_is_reported_and_not_replaced() {
        let path = path("bad");
        let original = b"not a sqlite database";
        fs::write(&path, original).unwrap();
        assert_eq!(inspect_database(&path), DatabaseHealth::Corrupt);
        assert_eq!(fs::read(&path).unwrap(), original);
        fs::remove_file(path).unwrap();
    }
}
