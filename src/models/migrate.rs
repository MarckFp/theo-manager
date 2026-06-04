/// Migration utilities: export, import, mode-switch, and data deletion.
///
/// | Scenario                                               | Function(s)                               |
/// |--------------------------------------------------------|-------------------------------------------|
/// | Back up local data / restore on another device         | [`export`] + [`import`]                   |
/// | Switch from offline (IndexedDB / mem) to cloud         | [`migrate_to_online`]                     |
/// | Switch from cloud back to offline                      | [`migrate_to_offline`]                    |
/// | Wipe all user data ("factory reset" / account delete)  | [`wipe`]                                  |
///
/// ## Multi-user cloud isolation
///
/// [`wipe`] and [`migrate_to_offline`] delete every record in [`TABLES`]
/// within the **currently selected namespace / database only**. They never
/// drop the namespace itself, so other users on the same SurrealDB instance
/// are unaffected — provided each user configures a distinct `namespace` or
/// `database` in their [`OnlineConfig`][crate::database::OnlineConfig].
/// Using the congregation name (or a UUID assigned at first setup) as the
/// namespace is a simple way to achieve per-congregation isolation.
use serde_json::Value;

use crate::database::Db;

/// All known tables in **parent-first** dependency order.
/// Add new table constants here when you add new models.
pub const TABLES: &[&str] = &[
    "_keystore", // encryption metadata — must migrate with data
    super::congregation::TABLE,
    super::user::TABLE,
    super::absence::TABLE,
];

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum MigrateError {
    Database(surrealdb::Error),
    Serialization(serde_json::Error),
}

impl std::fmt::Display for MigrateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MigrateError::Database(e) => write!(f, "Database error: {e}"),
            MigrateError::Serialization(e) => write!(f, "Serialisation error: {e}"),
        }
    }
}

impl std::error::Error for MigrateError {}

impl From<surrealdb::Error> for MigrateError {
    fn from(e: surrealdb::Error) -> Self {
        MigrateError::Database(e)
    }
}

impl From<serde_json::Error> for MigrateError {
    fn from(e: serde_json::Error) -> Self {
        MigrateError::Serialization(e)
    }
}

// ---------------------------------------------------------------------------
// Export / Import  (device-to-device backup & restore)
// ---------------------------------------------------------------------------

/// Dump every record from every known table as a JSON object:
/// `{ "congregation": [...], "user": [...], "absence": [...] }`.
///
/// The snapshot is self-contained; save it to a file or transfer it to
/// another device for [`import`].
pub async fn export(db: &Db) -> Result<Value, MigrateError> {
    let mut out = serde_json::Map::new();
    for &table in TABLES {
        let records: Vec<Value> = db.select(table).await?;
        out.insert(table.to_string(), Value::Array(records));
    }
    Ok(Value::Object(out))
}

/// Restore a snapshot (produced by [`export`]) into `target`.
///
/// **Full overwrite**: every table in `target` is cleared before inserting
/// the snapshot records so that original record IDs are preserved and no
/// duplicates are created. Tables are cleared child-first (reverse dependency
/// order); records are inserted parent-first.
///
/// Call [`export`] on `target` first if you need a backup before overwriting.
pub async fn import(target: &Db, data: Value) -> Result<(), MigrateError> {
    let map = match data {
        Value::Object(m) => m,
        _ => return Ok(()),
    };

    // Clear child-first to respect (soft) FK ordering.
    for &table in TABLES.iter().rev() {
        target.query(format!("DELETE {table}")).await?;
    }

    // Insert parent-first; INSERT preserves the `id` field in the payload.
    for &table in TABLES {
        let Some(records) = map.get(table).and_then(Value::as_array) else {
            continue;
        };
        for record in records {
            target
                .query(format!("INSERT INTO {table} $data"))
                .bind(("data", record.clone()))
                .await?;
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Mode switch: offline → online
// ---------------------------------------------------------------------------

/// Copy all local (offline) data to `online`, then wipe the local database.
///
/// The online database is fully overwritten (same semantics as [`import`]).
/// After this returns, update `AppDatabase` to use `online` and drop the
/// local connection — no stale copy remains locally.
pub async fn migrate_to_online(local: &Db, online: &Db) -> Result<(), MigrateError> {
    let snapshot = export(local).await?;
    import(online, snapshot).await?;
    wipe(local).await
}

// ---------------------------------------------------------------------------
// Mode switch: online → offline
// ---------------------------------------------------------------------------

/// Copy all cloud (online) data to `local`, then delete the user's records
/// from the cloud database.
///
/// The local database is fully overwritten (same semantics as [`import`]).
/// After this returns, update `AppDatabase` to use `local` and close the
/// online connection — no stale copy remains in the cloud.
pub async fn migrate_to_offline(online: &Db, local: &Db) -> Result<(), MigrateError> {
    let snapshot = export(online).await?;
    import(local, snapshot).await?;
    wipe(online).await
}

// ---------------------------------------------------------------------------
// Data deletion
// ---------------------------------------------------------------------------

/// Delete every record in every known table (child-first / reverse dependency
/// order).
///
/// - **Offline**: clears the entire embedded database.
/// - **Online**: clears only the records inside the configured
///   namespace / database. The namespace itself is never dropped, so other
///   users on the same SurrealDB instance are unaffected (see module-level
///   note on isolation).
pub async fn wipe(db: &Db) -> Result<(), MigrateError> {
    for &table in TABLES.iter().rev() {
        db.query(format!("DELETE {table}")).await?;
    }
    Ok(())
}
