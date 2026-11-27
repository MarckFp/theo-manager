//! The encrypted store of one congregation: it turns a [`Directory`] (and the congregation's
//! details) into encrypted records in a `theo_db::Db`, and back.
//!
//! **Records.** Every collection (a `Directory` field, see `collections.rs`) is split into records:
//! - A list whose items all have a unique `id` becomes **one record per item**, plus one
//!   `#order` record with the order of the ids.
//! - Anything else (settings, sets, maps, lists without ids) is **one record** for the whole
//!   collection.
//!
//! Each record holds `CBOR({ "k": item key, "v": item })` (R-306), padded and encrypted with its
//! collection's key domain ([`theo_crypto::envelope`]). The AAD binds it to the congregation, the
//! collection, the record id and the version (R-304). The record id is
//! [`theo_crypto::RootKey::record_id`] of the collection and the item key, so it looks random and
//! stays the same on every save. A member without the root key gives a new item a random id;
//! everyone else learns it when they read the record.
//!
//! **Saving** writes only what changed since the last load or save, and deletes (tombstones) what
//! is gone, in batches of up to 100 (one request and one transaction each on a server). Saves never overlap: a save asked for while one runs is done right after it.
//!
//! **Opening** with the congregation password unwraps the root key (`vault`), the keyring and then
//! every record. A **member** (Cloud, 5.6) opens with only the domain keys of their grants
//! ([`Store::open_with_keys`]): records of other domains are left out, and never written or
//! deleted by that member. A full-access member can also open with the root key from their grant
//! ([`Store::open_with_root`]), without the password.
//!
//! **Keys change** when a member leaves (R-246): [`Store::rotate_domains`] makes new versions of
//! the domains they held (new writes use them), and [`Store::rotate_root`] replaces the root key
//! and the congregation password when they had full access.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Map, Value};
use theo_crypto::envelope::{self, RecordContext};
use theo_crypto::{DomainId, DomainSecret, KdfParams, Keyring, RootKey, Vault};
use theo_db::{Db, DbError, KeyringEntry, NewRecord, RecordId, Write};

use crate::collections::{domain_of, DETAILS};
use crate::people::Directory;

/// The key of a collection kept as one record.
const WHOLE: &str = "";
/// The key of a split collection's order record.
const ORDER: &str = "#order";

/// Why the store could not open or save.
#[derive(Clone, Debug, PartialEq)]
pub enum StoreError {
    /// The storage failed (also a conflict with another writer).
    Db(DbError),
    /// Wrong password, damaged data or a key problem.
    Crypto(theo_crypto::Error),
    /// The data does not fit the model.
    Data(String),
    /// There is no congregation in this database yet (or there is one already, for create).
    NoCongregation,
    /// There is already a congregation in this database.
    Exists,
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StoreError::Db(e) => e.fmt(f),
            StoreError::Crypto(theo_crypto::Error::WrongPasswordOrDamaged) => f.write_str("This password does not unlock the congregation."),
            StoreError::Crypto(e) => e.fmt(f),
            StoreError::Data(e) => write!(f, "The congregation's data could not be read: {e}"),
            StoreError::NoCongregation => f.write_str("There is no congregation here."),
            StoreError::Exists => f.write_str("There is already a congregation here."),
        }
    }
}

impl std::error::Error for StoreError {}

impl From<DbError> for StoreError {
    fn from(e: DbError) -> Self {
        StoreError::Db(e)
    }
}

impl From<theo_crypto::Error> for StoreError {
    fn from(e: theo_crypto::Error) -> Self {
        StoreError::Crypto(e)
    }
}

/// How many changes travel together ([`theo_db::Db::write_many`]: one request on a server).
const BATCH: usize = 100;

/// One change a save sends: a record to write (`plain`), or to delete (`plain` None).
struct Change {
    collection: String,
    key: String,
    id: RecordId,
    expected: Option<u64>,
    version: u64,
    plain: Option<Vec<u8>>,
    envelope: Vec<u8>,
}

/// What is known about a stored record.
#[derive(Clone, Debug)]
struct Cached {
    id: RecordId,
    version: u64,
    /// The CBOR plaintext last written or read (None for a tombstone).
    plain: Option<Vec<u8>>,
}

type Item = (String, String, Value);
/// A collection while it is put back together: (whole value, order, items by key).
type Parts = (Option<Value>, Option<Vec<String>>, BTreeMap<String, Value>);

/// An unlocked congregation's store. Dropping it drops the keys (R-309).
pub struct Store {
    db: Db,
    congregation_id: String,
    /// None for a member who opened with grants only.
    root: RefCell<Option<RootKey>>,
    keyring: RefCell<Keyring>,
    /// The collections this account may write (a Cloud member's membership); None: all.
    writable: RefCell<Option<BTreeSet<String>>>,
    cache: RefCell<BTreeMap<(String, String), Cached>>,
    saving: Cell<bool>,
    pending: RefCell<Option<(Value, Directory)>>,
}

impl std::fmt::Debug for Store {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Store({}, {:?})", self.congregation_id, self.db)
    }
}

fn data(e: impl std::fmt::Display) -> StoreError {
    StoreError::Data(e.to_string())
}

fn item_key(v: &Value) -> Option<String> {
    match v.get("id")? {
        Value::Number(n) => Some(n.to_string()),
        Value::String(s) => Some(s.clone()),
        _ => None,
    }
}

/// Splits the details and the directory into (collection, key, value) items.
fn split(details: &Value, dir: &Directory) -> Result<Vec<Item>, StoreError> {
    let mut out = vec![(DETAILS.to_string(), WHOLE.to_string(), details.clone())];
    let Value::Object(fields) = serde_json::to_value(dir).map_err(data)? else { return Err(data("the directory is not an object")) };
    for (collection, value) in fields {
        let keys: Option<Vec<String>> = match &value {
            Value::Array(items) if !items.is_empty() => items.iter().map(item_key).collect(),
            _ => None,
        };
        match keys {
            Some(keys) if keys.iter().collect::<BTreeSet<_>>().len() == keys.len() => {
                let Value::Array(items) = value else { continue };
                out.push((collection.clone(), ORDER.into(), Value::from(keys.clone())));
                out.extend(keys.into_iter().zip(items).map(|(k, v)| (collection.clone(), k, v)));
            }
            _ => out.push((collection, WHOLE.into(), value)),
        }
    }
    Ok(out)
}

/// Puts items back together: (details, directory).
fn assemble(items: Vec<Item>) -> Result<(Value, Directory), StoreError> {
    let mut details = Value::Null;
    let mut by: BTreeMap<String, Parts> = BTreeMap::new();
    for (collection, key, value) in items {
        if collection == DETAILS {
            details = value;
            continue;
        }
        let e = by.entry(collection).or_default();
        match key.as_str() {
            WHOLE => e.0 = Some(value),
            ORDER => e.1 = serde_json::from_value(value).ok(),
            _ => {
                e.2.insert(key, value);
            }
        }
    }
    let mut fields = Map::new();
    for (collection, (whole, order, mut items)) in by {
        let value = match whole {
            Some(v) if items.is_empty() => v,
            _ => {
                // The order first; items it does not know (written elsewhere) after, by key.
                let mut list: Vec<Value> = order.unwrap_or_default().iter().filter_map(|k| items.remove(k)).collect();
                list.extend(items.into_values());
                Value::Array(list)
            }
        };
        fields.insert(collection, value);
    }
    let dir = serde_json::from_value(Value::Object(fields)).map_err(data)?;
    Ok((details, dir))
}

fn encode(key: &str, value: &Value) -> Result<Vec<u8>, StoreError> {
    let mut m = Map::new();
    m.insert("k".into(), Value::from(key));
    m.insert("v".into(), value.clone());
    let mut out = Vec::new();
    ciborium::into_writer(&Value::Object(m), &mut out).map_err(data)?;
    Ok(out)
}

fn decode(plain: &[u8]) -> Result<(String, Value), StoreError> {
    let v: Value = ciborium::from_reader(plain).map_err(data)?;
    let key = v.get("k").and_then(Value::as_str).ok_or_else(|| data("a record has no key"))?.to_string();
    Ok((key, v.get("v").cloned().unwrap_or(Value::Null)))
}

fn domain(collection: &str) -> Result<DomainId, StoreError> {
    domain_of(collection).ok_or_else(|| data(format!("the collection {collection} has no key domain")))
}

impl Store {
    fn ctx<'a>(&'a self, collection: &'a str, record_id: &'a str, version: u64) -> RecordContext<'a> {
        RecordContext { congregation_id: &self.congregation_id, collection, scope_id: "", record_id, version }
    }

    /// Creates a congregation in an empty database: the vault (root key under the password), the
    /// key domains (one version each), and the first save of `details` and `dir`.
    pub async fn create(db: Db, congregation_id: &str, password: &str, params: KdfParams, details: &Value, dir: &Directory) -> Result<Store, StoreError> {
        if db.read_header().await?.is_some() {
            return Err(StoreError::Exists);
        }
        let (vault, root) = Vault::create(congregation_id, password, params)?;
        db.write_header(&vault.to_bytes(), None).await?;
        let mut keyring = Keyring::new();
        for d in [DomainId::SCHEDULE, DomainId::PEOPLE, DomainId::REPORTS, DomainId::TERRITORIES] {
            let version = keyring.rotate(d)?;
            let public_key = keyring.get(d, version).map(|s| s.keys().public_key().to_bytes()).unwrap_or_default();
            let wrapped_secret = keyring.wrap_entry(&root, congregation_id, d, version)?;
            db.put_keyring(&KeyringEntry { domain: d.0, key_version: version, public_key, wrapped_secret }).await?;
        }
        let store = Store::new(db, congregation_id, Some(root), keyring);
        store.save(details.clone(), dir).await?;
        Ok(store)
    }

    fn new(db: Db, congregation_id: &str, root: Option<RootKey>, keyring: Keyring) -> Store {
        Store {
            db,
            congregation_id: congregation_id.into(),
            root: RefCell::new(root),
            keyring: RefCell::new(keyring),
            writable: RefCell::new(None),
            cache: RefCell::new(BTreeMap::new()),
            saving: Cell::new(false),
            pending: RefCell::new(None),
        }
    }

    /// Opens a congregation with its password: the store, the details and the directory.
    pub async fn open(db: Db, congregation_id: &str, password: &str) -> Result<(Store, Value, Directory), StoreError> {
        let header = db.read_header().await?.ok_or(StoreError::NoCongregation)?;
        let root = Vault::from_bytes(&header.bytes)?.unlock(congregation_id, password)?;
        Store::open_with_root(db, congregation_id, root).await
    }

    /// Opens a congregation with its root key (a full-access member's root grant): the keyring
    /// and every record, as with the password.
    pub async fn open_with_root(db: Db, congregation_id: &str, root: RootKey) -> Result<(Store, Value, Directory), StoreError> {
        let mut keyring = Keyring::new();
        for e in db.keyring().await? {
            keyring.unwrap_entry(&root, congregation_id, DomainId(e.domain), e.key_version, &e.wrapped_secret)?;
        }
        Store::new(db, congregation_id, Some(root), keyring).load().await
    }

    /// Opens a congregation with some of its domain keys only (a member's grants, 5.6): what is
    /// in other domains is left out of the directory, and this store never writes it.
    pub async fn open_with_keys(db: Db, congregation_id: &str, keys: Vec<(DomainId, u32, DomainSecret)>) -> Result<(Store, Value, Directory), StoreError> {
        let mut keyring = Keyring::new();
        for (domain, version, secret) in keys {
            keyring.insert(domain, version, secret);
        }
        Store::new(db, congregation_id, None, keyring).load().await
    }

    /// Reads and decrypts every record this store has keys for.
    async fn load(self) -> Result<(Store, Value, Directory), StoreError> {
        let store = self;
        let mut items = Vec::new();
        for r in store.db.changes(0).await? {
            let hex = r.record_id.to_hex();
            if r.deleted {
                // The key of a tombstone is unknown; it is kept by id so a new item with the same
                // id names its version. (Its cache key is the id, which no live item uses.)
                store.cache.borrow_mut().insert((r.collection.clone(), format!("~{hex}")), Cached { id: r.record_id, version: r.version, plain: None });
                continue;
            }
            let readable = envelope::header(&r.envelope).is_ok_and(|h| store.keyring.borrow().get(h.domain, h.key_version).is_some());
            if !readable && !store.has_root() {
                // A domain this member holds no key for.
                continue;
            }
            let plain = envelope::open(&store.keyring.borrow(), &store.ctx(&r.collection, &hex, r.version), &r.envelope)?;
            let (key, value) = decode(&plain)?;
            store.cache.borrow_mut().insert((r.collection.clone(), key.clone()), Cached { id: r.record_id, version: r.version, plain: Some(plain.to_vec()) });
            items.push((r.collection, key, value));
        }
        let (details, dir) = assemble(items)?;
        Ok((store, details, dir))
    }

    /// Is this the congregation password? (Unwraps the root key again; slow on purpose.)
    pub async fn password_ok(&self, password: &str) -> bool {
        match self.db.read_header().await {
            Ok(Some(h)) => Vault::from_bytes(&h.bytes).and_then(|v| v.unlock(&self.congregation_id, password)).is_ok(),
            _ => false,
        }
    }

    /// Copies this congregation into another, **empty** database (moving it between Local and
    /// Cloud, D-51): the same id, the same vault header (so the same congregation password) and
    /// the same keyring, and every record sealed again for the new database. Returns the store of
    /// the copy. Needs the root key. The source is not changed: delete it only once this succeeded.
    pub async fn copy_to(&self, target: Db, details: Value, dir: &Directory) -> Result<Store, StoreError> {
        let root = self.root.borrow().as_ref().map(|r| r.to_bytes()).ok_or(StoreError::Crypto(theo_crypto::Error::NoKey))?;
        if target.read_header().await?.is_some() {
            return Err(StoreError::Exists);
        }
        let header = self.db.read_header().await?.ok_or(StoreError::NoCongregation)?;
        // Bound to the congregation id, not to the database: copied as they are.
        for entry in self.db.keyring().await? {
            target.put_keyring(&entry).await?;
        }
        target.write_header(&header.bytes, None).await?;
        let copy = Store::new(target, &self.congregation_id, Some(RootKey::from_bytes(root.as_ref())?), self.keyring.borrow().clone());
        copy.save(details, dir).await?;
        Ok(copy)
    }

    /// Opened with the congregation password (not only with a member's grants)?
    pub fn has_root(&self) -> bool {
        self.root.borrow().is_some()
    }

    /// Runs `f` with the root key, when this store has it (for example to grant it to a
    /// full-access member).
    pub fn with_root<R>(&self, f: impl FnOnce(&RootKey) -> R) -> Option<R> {
        self.root.borrow().as_ref().map(f)
    }

    /// Limits the collections this store writes (a Cloud member's `writes`); None: all. Changes
    /// to other collections stay on this device and are not sent (the server would refuse them).
    pub fn set_writable(&self, collections: Option<BTreeSet<String>>) {
        *self.writable.borrow_mut() = collections;
    }

    /// Makes a new key version of each domain (R-246): new writes use it. Records stay readable
    /// with the old versions, which stay in the keyring. Returns the new secrets, to grant them
    /// to the members who keep the domain. Needs the root key.
    pub async fn rotate_domains(&self, domains: &[DomainId]) -> Result<Vec<(DomainId, u32, DomainSecret)>, StoreError> {
        let mut out = Vec::new();
        for d in domains {
            let (entry, secret) = {
                let root = self.root.borrow();
                let root = root.as_ref().ok_or(StoreError::Crypto(theo_crypto::Error::NoKey))?;
                let mut keyring = self.keyring.borrow_mut();
                let version = keyring.rotate(*d)?;
                let secret = keyring.get(*d, version).cloned().ok_or(StoreError::Crypto(theo_crypto::Error::NoKey))?;
                let public_key = secret.keys().public_key().to_bytes();
                let wrapped_secret = keyring.wrap_entry(root, &self.congregation_id, *d, version)?;
                (KeyringEntry { domain: d.0, key_version: version, public_key, wrapped_secret }, (*d, version, secret))
            };
            self.db.put_keyring(&entry).await?;
            out.push(secret);
        }
        Ok(out)
    }

    /// Replaces the root key and the congregation password (R-246, when a full-access member
    /// leaves): a new vault, and every keyring entry wrapped again with the new root key. The
    /// domain keys stay. Whoever knew the old password and kept the old vault header can no
    /// longer unwrap the new keyring entries.
    pub async fn rotate_root(&self, new_password: &str, params: KdfParams) -> Result<(), StoreError> {
        if !self.has_root() {
            return Err(StoreError::Crypto(theo_crypto::Error::NoKey));
        }
        let header = self.db.read_header().await?.ok_or(StoreError::NoCongregation)?;
        let (vault, new_root) = Vault::create(&self.congregation_id, new_password, params)?;
        let entries: Vec<KeyringEntry> = {
            let keyring = self.keyring.borrow();
            keyring
                .versions()
                .map(|(d, v)| {
                    let public_key = keyring.get(d, v).map(|s| s.keys().public_key().to_bytes()).unwrap_or_default();
                    Ok(KeyringEntry { domain: d.0, key_version: v, public_key, wrapped_secret: keyring.wrap_entry(&new_root, &self.congregation_id, d, v)? })
                })
                .collect::<Result<_, theo_crypto::Error>>()?
        };
        // The keyring first: until the header changes, the old password still opens the old
        // header, but its root key no longer opens these entries; a failure here can be repeated.
        for e in &entries {
            self.db.put_keyring(e).await?;
        }
        self.db.write_header(&vault.to_bytes(), Some(header.version)).await?;
        *self.root.borrow_mut() = Some(new_root);
        Ok(())
    }

    /// The domain secrets this store holds for `domains`, every version (to give them to a new
    /// member through an invite bundle or a grant).
    pub fn domain_secrets(&self, domains: &[DomainId]) -> Vec<(DomainId, u32, DomainSecret)> {
        let keyring = self.keyring.borrow();
        keyring.versions().filter(|(d, _)| domains.contains(d)).filter_map(|(d, v)| keyring.get(d, v).map(|s| (d, v, s.clone()))).collect()
    }

    /// Changes the congregation password: only the vault header changes (6.2).
    pub async fn change_password(&self, current: &str, new: &str, params: KdfParams) -> Result<(), StoreError> {
        let header = self.db.read_header().await?.ok_or(StoreError::NoCongregation)?;
        let vault = Vault::from_bytes(&header.bytes)?;
        vault.unlock(&self.congregation_id, current)?;
        let changed = {
            let root = self.root.borrow();
            let root = root.as_ref().ok_or(StoreError::Crypto(theo_crypto::Error::NoKey))?;
            vault.change_password(&self.congregation_id, root, new, params)?
        };
        self.db.write_header(&changed.to_bytes(), Some(header.version)).await?;
        Ok(())
    }

    /// Saves the details and the directory: writes what changed, deletes what is gone. Returns
    /// how many records were written or deleted. While a save runs, a new one is queued and done
    /// right after (only the newest one).
    pub async fn save(&self, details: Value, dir: &Directory) -> Result<usize, StoreError> {
        *self.pending.borrow_mut() = Some((details, dir.clone()));
        if self.saving.get() {
            return Ok(0);
        }
        self.saving.set(true);
        let mut written = 0;
        let result = loop {
            let next = self.pending.borrow_mut().take();
            let Some((details, dir)) = next else { break Ok(written) };
            match self.write(&details, &dir).await {
                Ok(n) => written += n,
                Err(e) => break Err(e),
            }
        };
        self.saving.set(false);
        result
    }

    async fn write(&self, details: &Value, dir: &Directory) -> Result<usize, StoreError> {
        let items = split(details, dir)?;
        let mut live: BTreeSet<(String, String)> = BTreeSet::new();
        // Every change first (sealed), then sent in batches: one request each on a server.
        let mut changes: Vec<Change> = Vec::new();
        for (collection, key, value) in items {
            let domain = domain(&collection)?;
            let allowed = self.writable.borrow().as_ref().is_none_or(|w| w.contains(&collection));
            if self.keyring.borrow().current(domain).is_none() || !allowed {
                // A member without this domain, or without write access: not theirs to write.
                continue;
            }
            let plain = encode(&key, &value)?;
            let cache_key = (collection.clone(), key.clone());
            live.insert(cache_key);
            let known = self.cache.borrow().get(&(collection.clone(), key.clone())).cloned();
            // The id: the one already stored, else the one the root key gives, else a new random one.
            let derived = self.root.borrow().as_ref().map(|r| RecordId(r.record_id(&collection, &key)));
            let id = match (&known, derived) {
                (Some(c), _) => c.id,
                (None, Some(d)) => d,
                (None, None) => RecordId::random()?,
            };
            let cached = known.or_else(|| self.cache.borrow().get(&(collection.clone(), format!("~{}", id.to_hex()))).cloned());
            if cached.as_ref().and_then(|c| c.plain.as_ref()) == Some(&plain) {
                continue;
            }
            let expected = cached.as_ref().map(|c| c.version);
            let version = expected.map_or(1, |v| v + 1);
            let envelope = envelope::seal(&self.keyring.borrow(), domain, &self.ctx(&collection, &id.to_hex(), version), &plain)?;
            changes.push(Change { collection, key, id, expected, version, plain: Some(plain), envelope });
        }
        let writable = self.writable.borrow().clone();
        let gone: Vec<Change> = self
            .cache
            .borrow()
            .iter()
            .filter(|(k, c)| c.plain.is_some() && !live.contains(*k) && writable.as_ref().is_none_or(|w| w.contains(&k.0)))
            .map(|((collection, key), c)| Change { collection: collection.clone(), key: key.clone(), id: c.id, expected: Some(c.version), version: c.version + 1, plain: None, envelope: Vec::new() })
            .collect();
        changes.extend(gone);
        for batch in changes.chunks(BATCH) {
            let writes: Vec<Write> = batch
                .iter()
                .map(|c| match c.plain {
                    Some(_) => Write::Put(NewRecord { collection: &c.collection, record_id: c.id, scope_id: None, envelope: &c.envelope }, c.expected),
                    None => Write::Delete { collection: &c.collection, record_id: c.id, expected: c.expected.unwrap_or(0) },
                })
                .collect();
            let versions = self.db.write_many(&writes).await?;
            let mut cache = self.cache.borrow_mut();
            for (c, got) in batch.iter().zip(versions) {
                if got != c.version {
                    return Err(StoreError::Db(DbError::Conflict { current: got }));
                }
                let tombstone = (c.collection.clone(), format!("~{}", c.id.to_hex()));
                match &c.plain {
                    Some(plain) => {
                        cache.remove(&tombstone);
                        cache.insert((c.collection.clone(), c.key.clone()), Cached { id: c.id, version: got, plain: Some(plain.clone()) });
                    }
                    None => {
                        cache.remove(&(c.collection.clone(), c.key.clone()));
                        cache.insert(tombstone, Cached { id: c.id, version: got, plain: None });
                    }
                }
            }
        }
        Ok(changes.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FAST: KdfParams = KdfParams::MINIMUM;
    const CONG: &str = "cong-test";

    fn details() -> Value {
        serde_json::json!({ "name": "Riverside", "midweek": { "day": "Thursday", "time": "19:00" } })
    }

    #[test]
    fn a_congregation_round_trips_through_encrypted_records() -> Result<(), StoreError> {
        pollster::block_on(async {
            let db = Db::open(theo_db::DbConfig::Memory).await?;
            let dir = crate::demo::demo_directory(30);
            let store = Store::create(db.clone(), CONG, "Blue-Lamp-7", FAST, &details(), &dir).await?;
            // Every record is ciphertext of its collection's domain; no plaintext names leak.
            let records = db.changes(0).await?;
            assert!(records.len() > dir.people.len());
            let first = dir.people.first().map(|p| p.last_name.clone()).unwrap_or_default();
            assert!(records.iter().all(|r| !String::from_utf8_lossy(&r.envelope).contains(&first)));
            drop(store);

            assert!(matches!(Store::open(db.clone(), CONG, "wrong").await, Err(StoreError::Crypto(theo_crypto::Error::WrongPasswordOrDamaged))));
            let (store, d, back) = Store::open(db.clone(), CONG, "Blue-Lamp-7").await?;
            assert_eq!(d, details());
            assert_eq!(back, dir);

            // Only changes are written: one person changed, one group removed.
            let mut changed = back.clone();
            if let Some(p) = changed.people.first_mut() {
                p.notes = Some("New note".into());
            }
            let removed = changed.groups.pop().map(|g| g.id);
            let n = store.save(d.clone(), &changed).await?;
            assert!((2..=4).contains(&n), "wrote {n}");
            assert_eq!(store.save(d.clone(), &changed).await?, 0);
            let (_, _, again) = Store::open(db.clone(), CONG, "Blue-Lamp-7").await?;
            assert_eq!(again, changed);
            assert!(removed.is_some_and(|g| !again.groups.iter().any(|x| x.id == g)));

            // A removed item can come back (its tombstone version is named).
            let (store, d, mut back) = Store::open(db.clone(), CONG, "Blue-Lamp-7").await?;
            back.groups = dir.groups.clone();
            store.save(d, &back).await?;
            let (_, _, again) = Store::open(db.clone(), CONG, "Blue-Lamp-7").await?;
            assert_eq!(again.groups, dir.groups);
            Ok(())
        })
    }

    #[test]
    fn the_password_changes_without_rewriting_records() -> Result<(), StoreError> {
        pollster::block_on(async {
            let db = Db::open(theo_db::DbConfig::Memory).await?;
            let dir = crate::demo::demo_directory(10);
            let store = Store::create(db.clone(), CONG, "old", FAST, &details(), &dir).await?;
            let seq = db.last_seq().await?;
            assert!(store.password_ok("old").await && !store.password_ok("new").await);
            assert!(store.change_password("wrong", "new", FAST).await.is_err());
            store.change_password("old", "new", FAST).await?;
            assert_eq!(db.last_seq().await?, seq);
            assert!(Store::open(db.clone(), CONG, "old").await.is_err());
            let (_, _, back) = Store::open(db.clone(), CONG, "new").await?;
            assert_eq!(back, dir);
            // Another congregation id cannot open it (the vault is bound to it).
            assert!(Store::open(db, "cong-other", "new").await.is_err());
            Ok(())
        })
    }

    #[test]
    fn a_member_with_some_domains_reads_and_writes_only_those() -> Result<(), StoreError> {
        pollster::block_on(async {
            let db = Db::open(theo_db::DbConfig::Memory).await?;
            let dir = crate::demo::demo_directory(12);
            let admin = Store::create(db.clone(), CONG, "pw", FAST, &details(), &dir).await?;
            let keys = admin.domain_secrets(&[DomainId::SCHEDULE]);
            assert!(!keys.is_empty() && keys.iter().all(|(d, _, _)| *d == DomainId::SCHEDULE));

            // The member sees the schedule, not the people.
            let (member, d, mine) = Store::open_with_keys(db.clone(), CONG, keys).await?;
            assert!(!member.has_root());
            assert_eq!(d, details());
            assert!(mine.people.is_empty() && !dir.people.is_empty());
            assert_eq!(mine.duty_types, dir.duty_types);

            // Their change to the schedule is saved; the people they cannot read stay untouched.
            let mut changed = mine.clone();
            changed.events.clear();
            changed.duty_types.retain(|t| t.id != dir.duty_types.first().map(|f| f.id).unwrap_or_default());
            member.save(d.clone(), &changed).await?;
            let (_, _, back) = Store::open(db.clone(), CONG, "pw").await?;
            assert_eq!(back.people, dir.people);
            assert_eq!(back.duty_types, changed.duty_types);
            assert!(member.change_password("pw", "new", FAST).await.is_err());
            drop(admin);
            Ok(())
        })
    }

    #[test]
    fn keys_rotate_and_the_root_key_opens_without_the_password() -> Result<(), StoreError> {
        pollster::block_on(async {
            let db = Db::open(theo_db::DbConfig::Memory).await?;
            let dir = crate::demo::demo_directory(10);
            let store = Store::create(db.clone(), CONG, "old", FAST, &details(), &dir).await?;
            let old_people = store.domain_secrets(&[DomainId::PEOPLE]);

            // A new version of people: new writes use it, old records stay readable.
            let new = store.rotate_domains(&[DomainId::PEOPLE]).await?;
            assert_eq!((new.len(), new[0].1), (1, 2));
            let mut changed = dir.clone();
            if let Some(p) = changed.people.first_mut() {
                p.notes = Some("after rotation".into());
            }
            store.save(details(), &changed).await?;
            // Someone with only the old people key cannot read the new write.
            let (_, _, old_view) = Store::open_with_keys(db.clone(), CONG, old_people).await?;
            assert!(old_view.people.len() < changed.people.len());

            // A new root key and password: the old password no longer opens it.
            store.rotate_root("new", FAST).await?;
            assert!(Store::open(db.clone(), CONG, "old").await.is_err());
            let (reopened, _, back) = Store::open(db.clone(), CONG, "new").await?;
            assert_eq!(back, changed);

            // The root key alone opens it too (a full-access member's grant).
            assert!(reopened.with_root(|r| r.record_id("people", "1")).is_some());
            let header = db.read_header().await?.ok_or(StoreError::NoCongregation)?;
            let root = Vault::from_bytes(&header.bytes)?.unlock(CONG, "new")?;
            let (_, _, again) = Store::open_with_root(db, CONG, root).await?;
            assert_eq!(again, changed);
            Ok(())
        })
    }

    #[test]
    fn a_member_writes_only_their_collections() -> Result<(), StoreError> {
        pollster::block_on(async {
            let db = Db::open(theo_db::DbConfig::Memory).await?;
            let dir = crate::demo::demo_directory(10);
            let store = Store::create(db.clone(), CONG, "pw", FAST, &details(), &dir).await?;
            store.set_writable(Some(["events".to_string()].into_iter().collect()));
            let mut changed = dir.clone();
            changed.events.clear();
            changed.groups.clear();
            store.save(details(), &changed).await?;
            let (_, _, back) = Store::open(db, CONG, "pw").await?;
            assert!(back.events.is_empty());
            assert_eq!(back.groups, dir.groups);
            Ok(())
        })
    }

    #[test]
    fn a_congregation_moves_to_another_database_with_the_same_password() -> Result<(), StoreError> {
        pollster::block_on(async {
            let from = Db::open(theo_db::DbConfig::Memory).await?;
            let to = Db::open(theo_db::DbConfig::Memory).await?;
            let dir = crate::demo::demo_directory(20);
            let store = Store::create(from.clone(), CONG, "pw", FAST, &details(), &dir).await?;
            let mut changed = dir.clone();
            changed.events.clear();
            let copy = store.copy_to(to.clone(), details(), &changed).await?;
            assert!(copy.has_root());
            // The copy opens with the same password and holds the data given to it.
            let (_, d, back) = Store::open(to.clone(), CONG, "pw").await?;
            assert_eq!((d, back), (details(), changed.clone()));
            assert!(Store::open(to.clone(), CONG, "other").await.is_err());
            // A database that already holds a congregation is refused; the source is unchanged.
            assert!(matches!(store.copy_to(to, details(), &dir).await, Err(StoreError::Exists)));
            let (_, _, source) = Store::open(from, CONG, "pw").await?;
            assert_eq!(source, dir);
            Ok(())
        })
    }

    #[test]
    fn the_order_of_lists_is_kept() -> Result<(), StoreError> {
        pollster::block_on(async {
            let db = Db::open(theo_db::DbConfig::Memory).await?;
            let mut dir = crate::demo::demo_directory(12);
            dir.people.reverse();
            let store = Store::create(db.clone(), CONG, "pw", FAST, &details(), &dir).await?;
            dir.people.swap(0, 1);
            store.save(details(), &dir).await?;
            let (_, _, back) = Store::open(db, CONG, "pw").await?;
            assert_eq!(back.people.iter().map(|p| p.id).collect::<Vec<_>>(), dir.people.iter().map(|p| p.id).collect::<Vec<_>>());
            Ok(())
        })
    }
}

