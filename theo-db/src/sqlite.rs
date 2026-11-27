//! The SQLite engine (desktop and phone): one file per congregation. Writes run in a transaction,
//! so the version check and the write cannot be split by another writer. The schema version is
//! SQLite's `user_version`; [`MIGRATIONS`] are applied in order when a file is opened.

use std::cell::RefCell;
use std::path::Path;

use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};

use crate::{next_version, now_minute, DbError, Header, KeyringEntry, NewRecord, Record, RecordId};

/// Schema changes, in order. Never edit a published one: add a new one.
const MIGRATIONS: &[&str] = &["
    CREATE TABLE meta (key TEXT PRIMARY KEY, value INTEGER NOT NULL) STRICT;
    INSERT INTO meta (key, value) VALUES ('seq', 0);
    CREATE TABLE vault_header (id INTEGER PRIMARY KEY CHECK (id = 1), version INTEGER NOT NULL, bytes BLOB NOT NULL) STRICT;
    CREATE TABLE keyring (
        domain INTEGER NOT NULL, key_version INTEGER NOT NULL,
        public_key BLOB NOT NULL, wrapped_secret BLOB NOT NULL,
        PRIMARY KEY (domain, key_version)) STRICT;
    CREATE TABLE record (
        collection TEXT NOT NULL, record_id BLOB NOT NULL, scope_id BLOB,
        version INTEGER NOT NULL, seq INTEGER NOT NULL, updated_at INTEGER NOT NULL,
        deleted INTEGER NOT NULL, envelope BLOB NOT NULL,
        PRIMARY KEY (collection, record_id)) STRICT;
    CREATE INDEX record_seq ON record (seq);
"];

fn storage(e: rusqlite::Error) -> DbError {
    DbError::Storage(e.to_string())
}

fn id_of(bytes: Vec<u8>) -> Result<RecordId, rusqlite::Error> {
    let arr: [u8; 16] = bytes.try_into().map_err(|_| rusqlite::Error::InvalidColumnType(0, "record id".into(), rusqlite::types::Type::Blob))?;
    Ok(RecordId(arr))
}

fn row_to_record(row: &rusqlite::Row<'_>) -> Result<Record, rusqlite::Error> {
    let scope: Option<Vec<u8>> = row.get(2)?;
    Ok(Record {
        collection: row.get(0)?,
        record_id: id_of(row.get(1)?)?,
        scope_id: scope.map(id_of).transpose()?,
        version: row.get::<_, i64>(3)? as u64,
        seq: row.get::<_, i64>(4)? as u64,
        updated_at: row.get(5)?,
        deleted: row.get::<_, i64>(6)? != 0,
        envelope: row.get(7)?,
    })
}

const RECORD_COLUMNS: &str = "collection, record_id, scope_id, version, seq, updated_at, deleted, envelope";

pub(crate) struct Sqlite {
    conn: RefCell<Connection>,
}

impl Sqlite {
    pub(crate) fn open(path: &Path) -> Result<Self, DbError> {
        if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
            std::fs::create_dir_all(dir).map_err(|e| DbError::Storage(e.to_string()))?;
        }
        let mut conn = Connection::open(path).map_err(storage)?;
        conn.pragma_update(None, "journal_mode", "WAL").map_err(storage)?;
        conn.pragma_update(None, "foreign_keys", "ON").map_err(storage)?;
        let applied: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0)).map_err(storage)?;
        for (i, sql) in MIGRATIONS.iter().enumerate().skip(applied.max(0) as usize) {
            let tx = conn.transaction().map_err(storage)?;
            tx.execute_batch(sql).map_err(storage)?;
            tx.pragma_update(None, "user_version", (i + 1) as i64).map_err(storage)?;
            tx.commit().map_err(storage)?;
        }
        if applied as usize > MIGRATIONS.len() {
            return Err(DbError::Storage("This database was made by a newer version of Theo Manager.".into()));
        }
        Ok(Sqlite { conn: RefCell::new(conn) })
    }

    pub(crate) fn destroy(path: &Path) -> Result<(), DbError> {
        for suffix in ["", "-wal", "-shm"] {
            let mut p = path.as_os_str().to_owned();
            p.push(suffix);
            match std::fs::remove_file(&p) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(DbError::Storage(e.to_string())),
            }
        }
        Ok(())
    }

    /// Runs `f` in an immediate (write-locking) transaction.
    fn write<T>(&self, f: impl FnOnce(&rusqlite::Transaction<'_>) -> Result<T, DbError>) -> Result<T, DbError> {
        let mut conn = self.conn.borrow_mut();
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate).map_err(storage)?;
        let out = f(&tx)?;
        tx.commit().map_err(storage)?;
        Ok(out)
    }

    fn next_seq(tx: &rusqlite::Transaction<'_>) -> Result<u64, DbError> {
        tx.execute("UPDATE meta SET value = value + 1 WHERE key = 'seq'", []).map_err(storage)?;
        let seq: i64 = tx.query_row("SELECT value FROM meta WHERE key = 'seq'", [], |r| r.get(0)).map_err(storage)?;
        Ok(seq as u64)
    }

    fn current_version(tx: &rusqlite::Transaction<'_>, collection: &str, id: RecordId) -> Result<Option<u64>, DbError> {
        tx.query_row("SELECT version FROM record WHERE collection = ?1 AND record_id = ?2", params![collection, id.0.as_slice()], |r| r.get::<_, i64>(0))
            .optional()
            .map(|v| v.map(|v| v as u64))
            .map_err(storage)
    }

    pub(crate) async fn read_header(&self) -> Result<Option<Header>, DbError> {
        self.conn
            .borrow()
            .query_row("SELECT version, bytes FROM vault_header WHERE id = 1", [], |r| Ok(Header { version: r.get::<_, i64>(0)? as u64, bytes: r.get(1)? }))
            .optional()
            .map_err(storage)
    }

    pub(crate) async fn write_header(&self, bytes: &[u8], expected: Option<u64>) -> Result<u64, DbError> {
        self.write(|tx| {
            let current: Option<i64> = tx.query_row("SELECT version FROM vault_header WHERE id = 1", [], |r| r.get(0)).optional().map_err(storage)?;
            let version = next_version(current.map(|v| v as u64), expected)?;
            tx.execute("INSERT OR REPLACE INTO vault_header (id, version, bytes) VALUES (1, ?1, ?2)", params![version as i64, bytes]).map_err(storage)?;
            Ok(version)
        })
    }

    pub(crate) async fn keyring(&self) -> Result<Vec<KeyringEntry>, DbError> {
        let conn = self.conn.borrow();
        let mut stmt = conn.prepare("SELECT domain, key_version, public_key, wrapped_secret FROM keyring ORDER BY domain, key_version").map_err(storage)?;
        let rows = stmt
            .query_map([], |r| {
                // Typed reads: an out-of-range value is an error, not a silently cut number.
                Ok(KeyringEntry { domain: r.get::<_, u8>(0)?, key_version: r.get::<_, u32>(1)?, public_key: r.get(2)?, wrapped_secret: r.get(3)? })
            })
            .map_err(storage)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(storage)
    }

    pub(crate) async fn put_keyring(&self, entry: &KeyringEntry) -> Result<(), DbError> {
        self.write(|tx| {
            tx.execute(
                "INSERT OR REPLACE INTO keyring (domain, key_version, public_key, wrapped_secret) VALUES (?1, ?2, ?3, ?4)",
                params![i64::from(entry.domain), i64::from(entry.key_version), entry.public_key, entry.wrapped_secret],
            )
            .map_err(storage)?;
            Ok(())
        })
    }

    pub(crate) async fn put(&self, rec: &NewRecord<'_>, expected: Option<u64>) -> Result<u64, DbError> {
        self.write(|tx| {
            let version = next_version(Self::current_version(tx, rec.collection, rec.record_id)?, expected)?;
            let seq = Self::next_seq(tx)?;
            tx.execute(
                &format!("INSERT OR REPLACE INTO record ({RECORD_COLUMNS}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, ?7)"),
                params![rec.collection, rec.record_id.0.as_slice(), rec.scope_id.map(|s| s.0.to_vec()), version as i64, seq as i64, now_minute(), rec.envelope],
            )
            .map_err(storage)?;
            Ok(version)
        })
    }

    pub(crate) async fn get(&self, collection: &str, id: RecordId) -> Result<Option<Record>, DbError> {
        self.conn
            .borrow()
            .query_row(&format!("SELECT {RECORD_COLUMNS} FROM record WHERE collection = ?1 AND record_id = ?2"), params![collection, id.0.as_slice()], row_to_record)
            .optional()
            .map_err(storage)
    }

    pub(crate) async fn changes(&self, since_seq: u64) -> Result<Vec<Record>, DbError> {
        let conn = self.conn.borrow();
        let mut stmt = conn.prepare(&format!("SELECT {RECORD_COLUMNS} FROM record WHERE seq > ?1 ORDER BY seq")).map_err(storage)?;
        let rows = stmt.query_map(params![since_seq as i64], row_to_record).map_err(storage)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(storage)
    }

    pub(crate) async fn delete(&self, collection: &str, id: RecordId, expected: u64) -> Result<u64, DbError> {
        self.write(|tx| {
            let current = Self::current_version(tx, collection, id)?.ok_or(DbError::NotFound)?;
            let version = next_version(Some(current), Some(expected))?;
            let seq = Self::next_seq(tx)?;
            tx.execute(
                "UPDATE record SET version = ?3, seq = ?4, updated_at = ?5, deleted = 1, envelope = X'' WHERE collection = ?1 AND record_id = ?2",
                params![collection, id.0.as_slice(), version as i64, seq as i64, now_minute()],
            )
            .map_err(storage)?;
            Ok(version)
        })
    }

    pub(crate) async fn purge(&self, collection: &str, id: RecordId) -> Result<(), DbError> {
        self.write(|tx| {
            tx.execute("DELETE FROM record WHERE collection = ?1 AND record_id = ?2", params![collection, id.0.as_slice()]).map_err(storage)?;
            Ok(())
        })
    }

    pub(crate) async fn last_seq(&self) -> Result<u64, DbError> {
        let seq: i64 = self.conn.borrow().query_row("SELECT value FROM meta WHERE key = 'seq'", [], |r| r.get(0)).map_err(storage)?;
        Ok(seq as u64)
    }
}
