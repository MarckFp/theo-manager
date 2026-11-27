//! The memory engine: everything in a map, lost when dropped (tests and demos).

use std::cell::RefCell;
use std::collections::BTreeMap;

use crate::{next_version, now_minute, DbError, Header, KeyringEntry, NewRecord, Record, RecordId};

#[derive(Default)]
struct State {
    header: Option<Header>,
    keyring: BTreeMap<(u8, u32), KeyringEntry>,
    records: BTreeMap<(String, RecordId), Record>,
    seq: u64,
}

#[derive(Default)]
pub(crate) struct Memory {
    state: RefCell<State>,
}

impl Memory {
    pub(crate) async fn read_header(&self) -> Result<Option<Header>, DbError> {
        Ok(self.state.borrow().header.clone())
    }

    pub(crate) async fn write_header(&self, bytes: &[u8], expected: Option<u64>) -> Result<u64, DbError> {
        let mut s = self.state.borrow_mut();
        let version = next_version(s.header.as_ref().map(|h| h.version), expected)?;
        s.header = Some(Header { version, bytes: bytes.to_vec() });
        Ok(version)
    }

    pub(crate) async fn keyring(&self) -> Result<Vec<KeyringEntry>, DbError> {
        Ok(self.state.borrow().keyring.values().cloned().collect())
    }

    pub(crate) async fn put_keyring(&self, entry: &KeyringEntry) -> Result<(), DbError> {
        self.state.borrow_mut().keyring.insert((entry.domain, entry.key_version), entry.clone());
        Ok(())
    }

    pub(crate) async fn put(&self, rec: &NewRecord<'_>, expected: Option<u64>) -> Result<u64, DbError> {
        let mut s = self.state.borrow_mut();
        let key = (rec.collection.to_string(), rec.record_id);
        let version = next_version(s.records.get(&key).map(|r| r.version), expected)?;
        s.seq += 1;
        let seq = s.seq;
        s.records.insert(
            key,
            Record {
                collection: rec.collection.into(),
                record_id: rec.record_id,
                scope_id: rec.scope_id,
                version,
                seq,
                updated_at: now_minute(),
                deleted: false,
                envelope: rec.envelope.to_vec(),
            },
        );
        Ok(version)
    }

    pub(crate) async fn get(&self, collection: &str, id: RecordId) -> Result<Option<Record>, DbError> {
        Ok(self.state.borrow().records.get(&(collection.to_string(), id)).cloned())
    }

    pub(crate) async fn changes(&self, since_seq: u64) -> Result<Vec<Record>, DbError> {
        Ok(self.state.borrow().records.values().filter(|r| r.seq > since_seq).cloned().collect())
    }

    pub(crate) async fn delete(&self, collection: &str, id: RecordId, expected: u64) -> Result<u64, DbError> {
        let mut s = self.state.borrow_mut();
        s.seq += 1;
        let seq = s.seq;
        let r = s.records.get_mut(&(collection.to_string(), id)).ok_or(DbError::NotFound)?;
        let version = next_version(Some(r.version), Some(expected))?;
        r.version = version;
        r.seq = seq;
        r.updated_at = now_minute();
        r.deleted = true;
        r.envelope.clear();
        Ok(version)
    }

    pub(crate) async fn purge(&self, collection: &str, id: RecordId) -> Result<(), DbError> {
        self.state.borrow_mut().records.remove(&(collection.to_string(), id));
        Ok(())
    }

    pub(crate) async fn last_seq(&self) -> Result<u64, DbError> {
        Ok(self.state.borrow().seq)
    }
}
