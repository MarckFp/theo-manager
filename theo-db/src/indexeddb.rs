//! The IndexedDB engine (browser and installed web app): one IndexedDB database per
//! congregation, with three object stores:
//!
//! | Store | Key | Value |
//! |---|---|---|
//! | `meta` | `"header"`, `"seq"` | the vault header (encoded), the last change number |
//! | `keyring` | `"<domain>:<version>"` | an encoded [`KeyringEntry`] |
//! | `records` | `"<collection>\u{0}<record id hex>"` | an encoded [`Record`] |
//!
//! Values are `Uint8Array`s in a small binary format (below), so no JavaScript object shape has to
//! be kept in step with Rust. A write reads the current version and writes in **one** read-write
//! transaction, so two tabs cannot both pass the version check.

use idb::{Database, Factory, ObjectStoreParams, Query, TransactionMode};
use js_sys::Uint8Array;
use wasm_bindgen::JsValue;

use crate::{next_version, now_minute, DbError, Header, KeyringEntry, NewRecord, Record, RecordId};

const VERSION: u32 = 1;
const META: &str = "meta";
const KEYRING: &str = "keyring";
const RECORDS: &str = "records";

fn storage(e: impl std::fmt::Display) -> DbError {
    DbError::Storage(e.to_string())
}

// ----- the binary format -----

fn put_u64(out: &mut Vec<u8>, v: u64) {
    out.extend_from_slice(&v.to_le_bytes());
}

struct Reader<'a>(&'a [u8]);

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], DbError> {
        if self.0.len() < n {
            return Err(DbError::Storage("A stored value is damaged.".into()));
        }
        let (a, b) = self.0.split_at(n);
        self.0 = b;
        Ok(a)
    }
    fn u64(&mut self) -> Result<u64, DbError> {
        let b = self.take(8)?;
        let mut a = [0u8; 8];
        a.copy_from_slice(b);
        Ok(u64::from_le_bytes(a))
    }
    fn id(&mut self) -> Result<RecordId, DbError> {
        let mut a = [0u8; 16];
        a.copy_from_slice(self.take(16)?);
        Ok(RecordId(a))
    }
    fn bytes(&mut self) -> Result<Vec<u8>, DbError> {
        // A length that does not fit this platform is damage, not a value to cut short.
        let n = usize::try_from(self.u64()?).map_err(|_| DbError::Storage("a stored value is damaged".into()))?;
        Ok(self.take(n)?.to_vec())
    }
}

/// `version | seq | updated_at | deleted | scope? | collection | record id | envelope`.
fn encode_record(r: &Record) -> Vec<u8> {
    let mut out = Vec::with_capacity(64 + r.collection.len() + r.envelope.len());
    put_u64(&mut out, r.version);
    put_u64(&mut out, r.seq);
    put_u64(&mut out, r.updated_at as u64);
    out.push(u8::from(r.deleted));
    match r.scope_id {
        Some(s) => {
            out.push(1);
            out.extend_from_slice(&s.0);
        }
        None => out.push(0),
    }
    put_u64(&mut out, r.collection.len() as u64);
    out.extend_from_slice(r.collection.as_bytes());
    out.extend_from_slice(&r.record_id.0);
    put_u64(&mut out, r.envelope.len() as u64);
    out.extend_from_slice(&r.envelope);
    out
}

fn decode_record(b: &[u8]) -> Result<Record, DbError> {
    let mut r = Reader(b);
    let version = r.u64()?;
    let seq = r.u64()?;
    let updated_at = r.u64()? as i64;
    let deleted = r.take(1)?[0] != 0;
    let scope_id = if r.take(1)?[0] == 1 { Some(r.id()?) } else { None };
    let collection = String::from_utf8(r.bytes()?).map_err(storage)?;
    let record_id = r.id()?;
    let envelope = r.bytes()?;
    Ok(Record { collection, record_id, scope_id, version, seq, updated_at, deleted, envelope })
}

fn encode_entry(e: &KeyringEntry) -> Vec<u8> {
    let mut out = vec![e.domain];
    put_u64(&mut out, u64::from(e.key_version));
    put_u64(&mut out, e.public_key.len() as u64);
    out.extend_from_slice(&e.public_key);
    put_u64(&mut out, e.wrapped_secret.len() as u64);
    out.extend_from_slice(&e.wrapped_secret);
    out
}

fn decode_entry(b: &[u8]) -> Result<KeyringEntry, DbError> {
    let mut r = Reader(b);
    let domain = r.take(1)?[0];
    let key_version = u32::try_from(r.u64()?).map_err(|_| DbError::Storage("a stored key version is damaged".into()))?;
    Ok(KeyringEntry { domain, key_version, public_key: r.bytes()?, wrapped_secret: r.bytes()? })
}

fn js_bytes(b: &[u8]) -> JsValue {
    Uint8Array::from(b).into()
}

fn rust_bytes(v: &JsValue) -> Vec<u8> {
    Uint8Array::new(v).to_vec()
}

fn record_key(collection: &str, id: RecordId) -> JsValue {
    JsValue::from_str(&format!("{collection}\u{0}{}", id.to_hex()))
}

// ----- the engine -----

pub(crate) struct IndexedDb {
    db: Database,
}

/// Closes the connection as soon as the last user is gone. The browser would otherwise keep it
/// open until its garbage collector runs, and deleting the database (leaving a congregation,
/// moving it to the Cloud) waits for every connection to close.
impl Drop for IndexedDb {
    fn drop(&mut self) {
        self.db.close();
    }
}

impl IndexedDb {
    pub(crate) async fn open(name: &str) -> Result<Self, DbError> {
        let factory = Factory::new().map_err(storage)?;
        let mut request = factory.open(name, Some(VERSION)).map_err(storage)?;
        request.on_upgrade_needed(|event| {
            if let Ok(db) = idb::DatabaseEvent::database(&event) {
                let names = db.store_names();
                for store in [META, KEYRING, RECORDS] {
                    if !names.iter().any(|n| n == store) {
                        let _ = db.create_object_store(store, ObjectStoreParams::new());
                    }
                }
            }
        });
        let db = request.await.map_err(storage)?;
        Ok(IndexedDb { db })
    }

    pub(crate) async fn destroy(name: &str) -> Result<(), DbError> {
        let factory = Factory::new().map_err(storage)?;
        factory.delete(name).map_err(storage)?.await.map_err(storage)?;
        Ok(())
    }

    async fn read<T>(&self, store: &str, key: &str, decode: impl FnOnce(&[u8]) -> Result<T, DbError>) -> Result<Option<T>, DbError> {
        let tx = self.db.transaction(&[store], TransactionMode::ReadOnly).map_err(storage)?;
        let s = tx.object_store(store).map_err(storage)?;
        let v = s.get(Query::from(JsValue::from_str(key))).map_err(storage)?.await.map_err(storage)?;
        v.filter(|v| !v.is_undefined()).map(|v| decode(&rust_bytes(&v))).transpose()
    }

    pub(crate) async fn read_header(&self) -> Result<Option<Header>, DbError> {
        self.read(META, "header", |b| {
            let mut r = Reader(b);
            Ok(Header { version: r.u64()?, bytes: r.bytes()? })
        })
        .await
    }

    pub(crate) async fn write_header(&self, bytes: &[u8], expected: Option<u64>) -> Result<u64, DbError> {
        let tx = self.db.transaction(&[META], TransactionMode::ReadWrite).map_err(storage)?;
        let s = tx.object_store(META).map_err(storage)?;
        let current = s.get(Query::from(JsValue::from_str("header"))).map_err(storage)?.await.map_err(storage)?;
        let current = current.filter(|v| !v.is_undefined()).map(|v| Reader(&rust_bytes(&v)).u64()).transpose()?;
        let version = match next_version(current, expected) {
            Ok(v) => v,
            Err(e) => {
                let _ = tx.abort();
                return Err(e);
            }
        };
        let mut value = Vec::new();
        put_u64(&mut value, version);
        put_u64(&mut value, bytes.len() as u64);
        value.extend_from_slice(bytes);
        s.put(&js_bytes(&value), Some(&JsValue::from_str("header"))).map_err(storage)?.await.map_err(storage)?;
        tx.commit().map_err(storage)?.await.map_err(storage)?;
        Ok(version)
    }

    pub(crate) async fn keyring(&self) -> Result<Vec<KeyringEntry>, DbError> {
        let tx = self.db.transaction(&[KEYRING], TransactionMode::ReadOnly).map_err(storage)?;
        let s = tx.object_store(KEYRING).map_err(storage)?;
        let all = s.get_all(None, None).map_err(storage)?.await.map_err(storage)?;
        all.iter().map(|v| decode_entry(&rust_bytes(v))).collect()
    }

    pub(crate) async fn put_keyring(&self, entry: &KeyringEntry) -> Result<(), DbError> {
        let tx = self.db.transaction(&[KEYRING], TransactionMode::ReadWrite).map_err(storage)?;
        let s = tx.object_store(KEYRING).map_err(storage)?;
        let key = JsValue::from_str(&format!("{}:{}", entry.domain, entry.key_version));
        s.put(&js_bytes(&encode_entry(entry)), Some(&key)).map_err(storage)?.await.map_err(storage)?;
        tx.commit().map_err(storage)?.await.map_err(storage)?;
        Ok(())
    }

    /// Reads the record and the last change number, lets `f` decide the new record, and writes
    /// both, all in one transaction. `f` gives None to write nothing.
    async fn update(&self, collection: &str, id: RecordId, f: impl FnOnce(Option<Record>, u64) -> Result<Option<Record>, DbError>) -> Result<Option<Record>, DbError> {
        let tx = self.db.transaction(&[META, RECORDS], TransactionMode::ReadWrite).map_err(storage)?;
        let meta = tx.object_store(META).map_err(storage)?;
        let records = tx.object_store(RECORDS).map_err(storage)?;
        let key = record_key(collection, id);
        let current = records.get(Query::from(key.clone())).map_err(storage)?.await.map_err(storage)?;
        let current = current.filter(|v| !v.is_undefined()).map(|v| decode_record(&rust_bytes(&v))).transpose()?;
        let seq = meta.get(Query::from(JsValue::from_str("seq"))).map_err(storage)?.await.map_err(storage)?;
        let seq = seq.and_then(|v| v.as_f64()).unwrap_or(0.0) as u64 + 1;
        let next = match f(current, seq) {
            Ok(n) => n,
            Err(e) => {
                let _ = tx.abort();
                return Err(e);
            }
        };
        if let Some(r) = &next {
            records.put(&js_bytes(&encode_record(r)), Some(&key)).map_err(storage)?.await.map_err(storage)?;
            meta.put(&JsValue::from_f64(seq as f64), Some(&JsValue::from_str("seq"))).map_err(storage)?.await.map_err(storage)?;
        }
        tx.commit().map_err(storage)?.await.map_err(storage)?;
        Ok(next)
    }

    pub(crate) async fn put(&self, rec: &NewRecord<'_>, expected: Option<u64>) -> Result<u64, DbError> {
        let written = self
            .update(rec.collection, rec.record_id, |current, seq| {
                let version = next_version(current.map(|r| r.version), expected)?;
                Ok(Some(Record {
                    collection: rec.collection.into(),
                    record_id: rec.record_id,
                    scope_id: rec.scope_id,
                    version,
                    seq,
                    updated_at: now_minute(),
                    deleted: false,
                    envelope: rec.envelope.to_vec(),
                }))
            })
            .await?;
        written.map(|r| r.version).ok_or(DbError::NotFound)
    }

    pub(crate) async fn get(&self, collection: &str, id: RecordId) -> Result<Option<Record>, DbError> {
        let tx = self.db.transaction(&[RECORDS], TransactionMode::ReadOnly).map_err(storage)?;
        let s = tx.object_store(RECORDS).map_err(storage)?;
        let v = s.get(Query::from(record_key(collection, id))).map_err(storage)?.await.map_err(storage)?;
        v.filter(|v| !v.is_undefined()).map(|v| decode_record(&rust_bytes(&v))).transpose()
    }

    pub(crate) async fn changes(&self, since_seq: u64) -> Result<Vec<Record>, DbError> {
        let tx = self.db.transaction(&[RECORDS], TransactionMode::ReadOnly).map_err(storage)?;
        let s = tx.object_store(RECORDS).map_err(storage)?;
        let all = s.get_all(None, None).map_err(storage)?.await.map_err(storage)?;
        let mut out = Vec::new();
        for v in all {
            let r = decode_record(&rust_bytes(&v))?;
            if r.seq > since_seq {
                out.push(r);
            }
        }
        Ok(out)
    }

    pub(crate) async fn delete(&self, collection: &str, id: RecordId, expected: u64) -> Result<u64, DbError> {
        let written = self
            .update(collection, id, |current, seq| {
                let mut r = current.ok_or(DbError::NotFound)?;
                r.version = next_version(Some(r.version), Some(expected))?;
                r.seq = seq;
                r.updated_at = now_minute();
                r.deleted = true;
                r.envelope.clear();
                Ok(Some(r))
            })
            .await?;
        written.map(|r| r.version).ok_or(DbError::NotFound)
    }

    pub(crate) async fn purge(&self, collection: &str, id: RecordId) -> Result<(), DbError> {
        let tx = self.db.transaction(&[RECORDS], TransactionMode::ReadWrite).map_err(storage)?;
        let s = tx.object_store(RECORDS).map_err(storage)?;
        s.delete(Query::from(record_key(collection, id))).map_err(storage)?.await.map_err(storage)?;
        tx.commit().map_err(storage)?.await.map_err(storage)?;
        Ok(())
    }

    pub(crate) async fn last_seq(&self) -> Result<u64, DbError> {
        // `seq` is stored as a number, not as bytes.
        let tx = self.db.transaction(&[META], TransactionMode::ReadOnly).map_err(storage)?;
        let s = tx.object_store(META).map_err(storage)?;
        let v = s.get(Query::from(JsValue::from_str("seq"))).map_err(storage)?.await.map_err(storage)?;
        Ok(v.and_then(|v| v.as_f64()).unwrap_or(0.0) as u64)
    }
}
