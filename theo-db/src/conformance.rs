//! The conformance suite: what every engine must do the same way. Each engine's tests call
//! [`run`] on a fresh, empty database (natively in `tests/conformance.rs`; the IndexedDB engine in
//! the browser).

use crate::{Db, DbError, KeyringEntry, NewRecord, RecordId};

fn check(ok: bool, what: &str) -> Result<(), String> {
    if ok { Ok(()) } else { Err(format!("conformance: {what}")) }
}

fn err(e: DbError) -> String {
    format!("conformance: unexpected error: {e:?}")
}

/// Runs every check on `db`, which must be new and empty. Returns the first failure.
pub async fn run(db: &Db) -> Result<(), String> {
    // Header: created once, then only with the right version.
    check(db.read_header().await.map_err(err)?.is_none(), "a new database has no header")?;
    check(db.write_header(b"h1", None).await.map_err(err)? == 1, "first header is version 1")?;
    check(db.write_header(b"again", None).await == Err(DbError::Conflict { current: 1 }), "a second first header conflicts")?;
    check(db.write_header(b"h2", Some(1)).await.map_err(err)? == 2, "header version grows")?;
    let h = db.read_header().await.map_err(err)?;
    check(h.as_ref().map(|h| (h.version, h.bytes.as_slice())) == Some((2, b"h2".as_slice())), "header reads back")?;

    // Keyring: one entry per (domain, version); replace keeps one.
    let k = |domain: u8, v: u32, s: &[u8]| KeyringEntry { domain, key_version: v, public_key: vec![domain; 4], wrapped_secret: s.to_vec() };
    db.put_keyring(&k(2, 1, b"a")).await.map_err(err)?;
    db.put_keyring(&k(2, 2, b"b")).await.map_err(err)?;
    db.put_keyring(&k(1, 1, b"c")).await.map_err(err)?;
    db.put_keyring(&k(2, 2, b"B")).await.map_err(err)?;
    let mut ring = db.keyring().await.map_err(err)?;
    ring.sort_by_key(|e| (e.domain, e.key_version));
    check(ring == vec![k(1, 1, b"c"), k(2, 1, b"a"), k(2, 2, b"B")], "keyring entries read back, replaced in place")?;

    // Records: create, conflict, update, read.
    let start = db.last_seq().await.map_err(err)?;
    let a = RecordId::random().map_err(err)?;
    let b = RecordId::random().map_err(err)?;
    let scope = RecordId::random().map_err(err)?;
    let rec = |id: RecordId, env: &'static [u8]| NewRecord { collection: "people", record_id: id, scope_id: None, envelope: env };
    check(db.put(&rec(a, b"a1"), None).await.map_err(err)? == 1, "a new record is version 1")?;
    check(db.put(&rec(a, b"x"), None).await == Err(DbError::Conflict { current: 1 }), "creating an existing record conflicts")?;
    check(db.put(&rec(a, b"x"), Some(5)).await == Err(DbError::Conflict { current: 1 }), "a wrong version conflicts")?;
    check(db.put(&rec(b, b"x"), Some(1)).await == Err(DbError::Conflict { current: 0 }), "updating a missing record conflicts")?;
    check(db.put(&rec(a, b"a2"), Some(1)).await.map_err(err)? == 2, "an update is +1")?;
    let other = NewRecord { collection: "reports", record_id: b, scope_id: Some(scope), envelope: b"r1" };
    db.put(&other, None).await.map_err(err)?;
    let got = db.get("people", a).await.map_err(err)?.ok_or("conformance: record a is missing")?;
    check(got.version == 2 && got.envelope == b"a2" && !got.deleted && got.scope_id.is_none(), "a record reads back")?;
    check(got.updated_at % 60 == 0 && got.updated_at > 1_700_000_000, "the time is rounded to the minute")?;
    let got_b = db.get("reports", b).await.map_err(err)?;
    check(got_b.as_ref().and_then(|r| r.scope_id) == Some(scope), "the scope id reads back")?;
    check(db.get("people", b).await.map_err(err)?.is_none(), "collections are separate")?;

    // Changes and lists, by change number.
    let all = db.changes(start).await.map_err(err)?;
    check(all.len() == 2 && all[0].record_id == a && all[1].record_id == b && all[0].seq < all[1].seq, "changes come in order, one per record")?;
    let after = all[0].seq;
    let later = db.changes(after).await.map_err(err)?;
    check(later.len() == 1 && later[0].record_id == b, "changes after a number skip older ones")?;
    let people = db.list("people", 0).await.map_err(err)?;
    check(people.len() == 1 && people[0].record_id == a, "list gives one collection")?;
    check(db.last_seq().await.map_err(err)? == all[1].seq, "the last change number is the newest")?;

    // Delete: a tombstone without the envelope, still versioned; then purge.
    check(db.delete("people", a, 1).await == Err(DbError::Conflict { current: 2 }), "a delete checks the version")?;
    check(db.delete("people", RecordId::default(), 1).await == Err(DbError::NotFound), "deleting a missing record says so")?;
    check(db.delete("people", a, 2).await.map_err(err)? == 3, "a delete is +1")?;
    let t = db.get("people", a).await.map_err(err)?.ok_or("conformance: the tombstone is missing")?;
    check(t.deleted && t.envelope.is_empty() && t.version == 3, "a tombstone keeps no envelope")?;
    check(db.changes(after).await.map_err(err)?.iter().any(|r| r.record_id == a && r.deleted), "the delete is a change")?;
    check(db.put(&rec(a, b"again"), None).await == Err(DbError::Conflict { current: 3 }), "re-creating names the tombstone's version")?;
    check(db.put(&rec(a, b"again"), Some(3)).await.map_err(err)? == 4, "a tombstone can be written again")?;
    db.purge("people", a).await.map_err(err)?;
    check(db.get("people", a).await.map_err(err)?.is_none(), "purge removes it for good")?;
    db.purge("people", a).await.map_err(err)?;

    // Binary safety: every byte value survives.
    let bytes: Vec<u8> = (0..=255u8).collect();
    let c = RecordId::random().map_err(err)?;
    db.put(&NewRecord { collection: "bin", record_id: c, scope_id: None, envelope: &bytes }, None).await.map_err(err)?;
    check(db.get("bin", c).await.map_err(err)?.map(|r| r.envelope) == Some(bytes), "all byte values survive")?;
    Ok(())
}
