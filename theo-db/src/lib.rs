//! # theo-db
//!
//! The single storage wrapper of Theo Manager (`REQUIREMENTS.md` 5.1, 5.2). One [`Db`] API, several
//! engines:
//!
//! | Engine | Where | Module |
//! |---|---|---|
//! | Memory | tests, demos | `memory` |
//! | SQLite | desktop and phone: one file per congregation | `sqlite` |
//! | IndexedDB | browser and installed web app: one database per congregation | `indexeddb` |
//! | SurrealDB | Cloud: one server database for all congregations, kept apart by permissions | [`surreal`] |
//!
//! `theo-db` stores **only ciphertext**: the vault header, the wrapped keyring entries and record
//! envelopes are opaque bytes here (R-204). It never depends on `theo-crypto` or `theo-core`. In
//! plaintext it keeps only what a server needs to route data: the collection name, the record id,
//! an optional scope id, the version, a change number and a minute-rounded time.
//!
//! **Versions** (R-224): every record has a `version`, 1 on first write and +1 on every write.
//! A write names the version it expects; a different current version is a [`DbError::Conflict`].
//! **Change numbers:** every write also gets the next `seq` of the database, so a reader can ask
//! for "everything after the last change I saw" ([`Db::changes`]).
//!
//! Every engine passes the same [`conformance`] suite.

pub mod conformance;
mod http;
#[cfg(target_arch = "wasm32")]
mod indexeddb;
mod memory;
#[cfg(not(target_arch = "wasm32"))]
mod sqlite;
pub mod surreal;

use std::fmt;

/// A 128-bit random identifier (records, congregations, scopes). Random, not time-ordered, so it
/// does not reveal when or in which order records were created (5.1).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct RecordId(pub [u8; 16]);

impl RecordId {
    /// A fresh random id.
    pub fn random() -> Result<Self, DbError> {
        let mut b = [0u8; 16];
        getrandom::fill(&mut b).map_err(|_| DbError::Storage("The device could not create random numbers.".into()))?;
        Ok(RecordId(b))
    }

    /// 32 lowercase hex digits.
    pub fn to_hex(&self) -> String {
        self.0.iter().map(|b| format!("{b:02x}")).collect()
    }

    /// Reads [`RecordId::to_hex`].
    pub fn from_hex(s: &str) -> Option<Self> {
        if s.len() != 32 || !s.is_ascii() {
            return None;
        }
        let mut b = [0u8; 16];
        for (i, out) in b.iter_mut().enumerate() {
            *out = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).ok()?;
        }
        Some(RecordId(b))
    }
}

impl fmt::Debug for RecordId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RecordId({})", self.to_hex())
    }
}

impl fmt::Display for RecordId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_hex())
    }
}

/// A stored record (5.1). `envelope` is empty for a tombstone.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Record {
    /// The collection (module) name, in plaintext.
    pub collection: String,
    /// The record's id.
    pub record_id: RecordId,
    /// An optional group or person id, for "own" and "own group" rules in Cloud mode (R-242).
    pub scope_id: Option<RecordId>,
    /// 1 on the first write, +1 on every write (also the delete).
    pub version: u64,
    /// The database's change number of the last write.
    pub seq: u64,
    /// Seconds since 1970, rounded down to the minute.
    pub updated_at: i64,
    /// Deleted (a tombstone: the envelope is removed).
    pub deleted: bool,
    /// The encrypted record (6.4).
    pub envelope: Vec<u8>,
}

/// What a write gives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NewRecord<'a> {
    /// The collection name.
    pub collection: &'a str,
    /// The record's id.
    pub record_id: RecordId,
    /// An optional group or person id (R-242).
    pub scope_id: Option<RecordId>,
    /// The encrypted record.
    pub envelope: &'a [u8],
}

/// One change of a batch ([`Db::write_many`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Write<'a> {
    /// Writes a record; `expected` as in [`Db::put`].
    Put(NewRecord<'a>, Option<u64>),
    /// Deletes a record (keeps a tombstone), as in [`Db::delete`].
    Delete {
        /// The collection name.
        collection: &'a str,
        /// The record's id.
        record_id: RecordId,
        /// The version it must have.
        expected: u64,
    },
}

/// The stored vault header (opaque bytes) and its version.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Header {
    /// 1 when written first, +1 on every write.
    pub version: u64,
    /// The header (`theo_crypto::Vault::to_bytes`).
    pub bytes: Vec<u8>,
}

/// One keyring entry (5.1): a key domain version, its public key and its wrapped secret.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyringEntry {
    /// The key domain id.
    pub domain: u8,
    /// The version of the domain's key.
    pub key_version: u32,
    /// The domain's X-Wing public key (sealed submissions).
    pub public_key: Vec<u8>,
    /// The domain secret wrapped by the root key.
    pub wrapped_secret: Vec<u8>,
}

/// Why a storage call failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DbError {
    /// The current version is not the expected one (R-224). `current` is 0 when there is none.
    Conflict {
        /// The current version.
        current: u64,
    },
    /// No such record.
    NotFound,
    /// The engine failed (disk, browser storage, corrupt data).
    Storage(String),
    /// This engine cannot do this (for example SQLite in a browser).
    Unsupported,
    /// The server refused the account (wrong credentials, signed out, username taken).
    Auth(String),
    /// The server's permissions do not allow this account to do this.
    Denied,
    /// The server could not be reached (no network, wrong address, or blocked by the browser).
    Unreachable(String),
    /// The server answered, but cannot serve the app now (not set up, busy, failing). The text
    /// says what to do.
    Service(String),
}

impl fmt::Display for DbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DbError::Conflict { .. } => f.write_str("The data was changed somewhere else. Open it again and repeat the change."),
            DbError::NotFound => f.write_str("This data no longer exists."),
            DbError::Storage(e) => write!(f, "The data could not be saved or read: {e}"),
            DbError::Unsupported => f.write_str("This storage is not available on this device."),
            DbError::Auth(e) => f.write_str(e),
            DbError::Unreachable(e) => f.write_str(e),
            DbError::Service(e) => f.write_str(e),
            DbError::Denied => f.write_str("Your role in this congregation does not allow this change."),
        }
    }
}

impl std::error::Error for DbError {}

/// Where a database is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DbConfig {
    /// In memory only (tests and demos); lost when dropped.
    Memory,
    /// A SQLite file (desktop and phone). Created when missing.
    Sqlite {
        /// The file.
        path: std::path::PathBuf,
    },
    /// An IndexedDB database of this name (browser). Created when missing.
    IndexedDb {
        /// The database name.
        name: String,
    },
    /// A congregation on a SurrealDB server, as a signed-in account.
    Surreal {
        /// The signed-in account.
        session: surreal::Session,
        /// The congregation id (32 hex digits).
        congregation: String,
    },
}

enum Engine {
    Memory(memory::Memory),
    #[cfg(not(target_arch = "wasm32"))]
    Sqlite(sqlite::Sqlite),
    #[cfg(target_arch = "wasm32")]
    IndexedDb(indexeddb::IndexedDb),
    Surreal(surreal::Surreal),
}

/// A congregation's database. Cheap to clone (the clones share the connection). Not `Send`:
/// use it on the thread that opened it.
#[derive(Clone)]
pub struct Db {
    engine: std::rc::Rc<Engine>,
}

impl fmt::Debug for Db {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let kind = match &*self.engine {
            Engine::Memory(_) => "memory",
            #[cfg(not(target_arch = "wasm32"))]
            Engine::Sqlite(_) => "sqlite",
            #[cfg(target_arch = "wasm32")]
            Engine::IndexedDb(_) => "indexeddb",
            Engine::Surreal(_) => "surrealdb",
        };
        write!(f, "Db({kind})")
    }
}

/// Seconds since 1970, rounded down to the minute (the time a record keeps, 5.1).
pub(crate) fn now_minute() -> i64 {
    let secs = web_time::SystemTime::now().duration_since(web_time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
    secs - secs % 60
}

/// The version check of every write: `expected` None means "must not exist". Returns the new
/// version. A tombstone counts as existing (re-creating it names its version).
pub(crate) fn next_version(current: Option<u64>, expected: Option<u64>) -> Result<u64, DbError> {
    match (current, expected) {
        (None, None) => Ok(1),
        (Some(c), Some(e)) if c == e => Ok(c + 1),
        (c, _) => Err(DbError::Conflict { current: c.unwrap_or(0) }),
    }
}

macro_rules! dispatch {
    ($self:ident, $e:ident => $body:expr) => {
        match &*$self.engine {
            Engine::Memory($e) => $body,
            #[cfg(not(target_arch = "wasm32"))]
            Engine::Sqlite($e) => $body,
            #[cfg(target_arch = "wasm32")]
            Engine::IndexedDb($e) => $body,
            Engine::Surreal($e) => $body,
        }
    };
}

impl Db {
    /// Opens (and creates when missing) a database.
    pub async fn open(config: DbConfig) -> Result<Db, DbError> {
        let engine = match config {
            DbConfig::Memory => Engine::Memory(memory::Memory::default()),
            #[cfg(not(target_arch = "wasm32"))]
            DbConfig::Sqlite { path } => Engine::Sqlite(sqlite::Sqlite::open(&path)?),
            #[cfg(target_arch = "wasm32")]
            DbConfig::IndexedDb { name } => Engine::IndexedDb(indexeddb::IndexedDb::open(&name).await?),
            DbConfig::Surreal { session, congregation } => Engine::Surreal(surreal::Surreal::open(session, &congregation).await?),
            #[allow(unreachable_patterns)]
            _ => return Err(DbError::Unsupported),
        };
        Ok(Db { engine: std::rc::Rc::new(engine) })
    }

    /// Deletes a whole database (leaving a congregation on this device), or on a server a whole
    /// congregation (its owner only). Missing is fine.
    pub async fn destroy(config: DbConfig) -> Result<(), DbError> {
        match config {
            DbConfig::Memory => Ok(()),
            #[cfg(not(target_arch = "wasm32"))]
            DbConfig::Sqlite { path } => sqlite::Sqlite::destroy(&path),
            #[cfg(target_arch = "wasm32")]
            DbConfig::IndexedDb { name } => indexeddb::IndexedDb::destroy(&name).await,
            DbConfig::Surreal { session, congregation } => {
                let id = RecordId::from_hex(&congregation).ok_or(DbError::NotFound)?;
                session.delete_congregation(id).await
            }
            #[allow(unreachable_patterns)]
            _ => Err(DbError::Unsupported),
        }
    }

    /// The vault header, if the congregation has one.
    pub async fn read_header(&self) -> Result<Option<Header>, DbError> {
        dispatch!(self, e => e.read_header().await)
    }

    /// Writes the vault header. `expected` None: there must be none yet. Returns the new version.
    pub async fn write_header(&self, bytes: &[u8], expected: Option<u64>) -> Result<u64, DbError> {
        dispatch!(self, e => e.write_header(bytes, expected).await)
    }

    /// Every keyring entry.
    pub async fn keyring(&self) -> Result<Vec<KeyringEntry>, DbError> {
        dispatch!(self, e => e.keyring().await)
    }

    /// Adds or replaces a keyring entry (one per domain and key version).
    pub async fn put_keyring(&self, entry: &KeyringEntry) -> Result<(), DbError> {
        dispatch!(self, e => e.put_keyring(entry).await)
    }

    /// Writes a record. `expected` None: it must not exist yet. Returns the new version.
    pub async fn put(&self, rec: &NewRecord<'_>, expected: Option<u64>) -> Result<u64, DbError> {
        dispatch!(self, e => e.put(rec, expected).await)
    }

    /// One record (also a tombstone).
    pub async fn get(&self, collection: &str, id: RecordId) -> Result<Option<Record>, DbError> {
        dispatch!(self, e => e.get(collection, id).await)
    }

    /// The records of a collection written after change number `since_seq` (0: all), with
    /// tombstones, by change number.
    pub async fn list(&self, collection: &str, since_seq: u64) -> Result<Vec<Record>, DbError> {
        let mut v = self.changes(since_seq).await?;
        v.retain(|r| r.collection == collection);
        Ok(v)
    }

    /// Every record written after change number `since_seq` (0: all), with tombstones, by change
    /// number.
    pub async fn changes(&self, since_seq: u64) -> Result<Vec<Record>, DbError> {
        let mut v = dispatch!(self, e => e.changes(since_seq).await)?;
        v.sort_by_key(|r| r.seq);
        Ok(v)
    }

    /// Deletes a record: it becomes a tombstone (R-013). Returns the tombstone's version.
    pub async fn delete(&self, collection: &str, id: RecordId, expected: u64) -> Result<u64, DbError> {
        dispatch!(self, e => e.delete(collection, id, expected).await)
    }

    /// Applies several changes in order and returns their new versions. On SurrealDB they travel
    /// in **one request and one transaction**: all are applied or none (the first error is
    /// returned), so a save over the network is not one round trip per record. The engines on
    /// this device apply them one by one.
    pub async fn write_many(&self, writes: &[Write<'_>]) -> Result<Vec<u64>, DbError> {
        if let Engine::Surreal(s) = &*self.engine {
            return s.write_many(writes).await;
        }
        let mut out = Vec::with_capacity(writes.len());
        for w in writes {
            out.push(match w {
                Write::Put(rec, expected) => self.put(rec, *expected).await?,
                Write::Delete { collection, record_id, expected } => self.delete(collection, *record_id, *expected).await?,
            });
        }
        Ok(out)
    }

    /// Removes a record for good, tombstone included (erasure, R-021).
    pub async fn purge(&self, collection: &str, id: RecordId) -> Result<(), DbError> {
        dispatch!(self, e => e.purge(collection, id).await)
    }

    /// The last change number given (0 for a new database).
    pub async fn last_seq(&self) -> Result<u64, DbError> {
        dispatch!(self, e => e.last_seq().await)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_hex_and_random() -> Result<(), DbError> {
        let a = RecordId::random()?;
        assert_ne!(a, RecordId::random()?);
        assert_eq!(RecordId::from_hex(&a.to_hex()), Some(a));
        assert_eq!(RecordId::from_hex("zz"), None);
        assert_eq!(RecordId::from_hex(&"g".repeat(32)), None);
        Ok(())
    }

    #[test]
    fn versions_are_checked() {
        assert_eq!(next_version(None, None), Ok(1));
        assert_eq!(next_version(Some(3), Some(3)), Ok(4));
        assert_eq!(next_version(Some(3), None), Err(DbError::Conflict { current: 3 }));
        assert_eq!(next_version(None, Some(1)), Err(DbError::Conflict { current: 0 }));
        assert_eq!(next_version(Some(3), Some(2)), Err(DbError::Conflict { current: 3 }));
    }
}
