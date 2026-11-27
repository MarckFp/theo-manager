//! A whole congregation through the real storage engines: every collection of the demo
//! congregation (people, meetings, duties, talks, territories, reports, …) is encrypted, written,
//! read back and compared, then changed and read back again.
//!
//! - SQLite: always (a file in the temp folder).
//! - SurrealDB: when `THEO_SURREAL_URL` points to a server with the schema applied (see
//!   `theo-db/tests/surreal.rs`); otherwise skipped. It signs up a throwaway account.
//!
//! IndexedDB runs only in a browser; the app's browser tests cover it.

use theo_core::demo::demo_directory;
use theo_core::people::Directory;
use theo_core::store::{Store, StoreError};
use theo_crypto::{DomainId, KdfParams};
use theo_db::{Db, DbConfig, RecordId};

const FAST: KdfParams = KdfParams::MINIMUM;

fn details() -> serde_json::Value {
    serde_json::json!({ "name": "Riverside", "number": "12345", "midweek": { "day": "Thursday", "time": "19:00" }, "defaults": { "language": "en" } })
}

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

/// Changes something in many collections at once.
fn change_everything(d: &mut Directory) {
    if let Some(p) = d.people.first_mut() {
        p.notes = Some("changed".into());
    }
    d.people.pop();
    d.groups.reverse();
    d.events.clear();
    d.duty_types.truncate(1);
    d.territories.truncate(d.territories.len() / 2);
    d.reports.truncate(d.reports.len() / 2);
    d.midweek_weeks.truncate(1);
    d.labels.clear();
}

async fn round_trip(db: Db, id: &str) -> Result<(), String> {
    let dir = demo_directory(36);
    assert!(!dir.people.is_empty() && !dir.reports.is_empty() && !dir.territories.is_empty() && !dir.midweek_weeks.is_empty(), "the demo has data in every module");
    let store = Store::create(db.clone(), id, "Blue-Lamp-7", FAST, &details(), &dir).await.map_err(err)?;
    drop(store);

    let (store, d, back) = Store::open(db.clone(), id, "Blue-Lamp-7").await.map_err(err)?;
    if d != details() || back != dir {
        return Err("the congregation did not come back the same".into());
    }
    let mut changed = back.clone();
    change_everything(&mut changed);
    store.save(d.clone(), &changed).await.map_err(err)?;
    drop(store);
    let (_, _, again) = Store::open(db.clone(), id, "Blue-Lamp-7").await.map_err(err)?;
    if again != changed {
        return Err("the changes did not come back the same".into());
    }
    if !matches!(Store::open(db, id, "wrong").await, Err(StoreError::Crypto(_))) {
        return Err("a wrong password opened it".into());
    }
    Ok(())
}

#[test]
fn a_whole_congregation_round_trips_through_sqlite() -> Result<(), String> {
    let path = std::env::temp_dir().join(format!("theo-engines-{}.sqlite", RecordId::random().map_err(err)?.to_hex()));
    let result = pollster::block_on(async {
        let db = Db::open(DbConfig::Sqlite { path: path.clone() }).await.map_err(err)?;
        round_trip(db, "cong-sqlite").await
    });
    for p in [path.clone(), path.with_extension("sqlite-wal"), path.with_extension("sqlite-shm")] {
        let _ = std::fs::remove_file(p);
    }
    result
}

#[test]
fn a_congregation_moves_from_sqlite_to_surrealdb_and_back() -> Result<(), String> {
    let Some(url) = std::env::var("THEO_SURREAL_URL").ok().filter(|u| !u.is_empty()) else {
        eprintln!("THEO_SURREAL_URL is not set: skipped");
        return Ok(());
    };
    let tag = RecordId::random().map_err(err)?.to_hex();
    let (first, second) = (std::env::temp_dir().join(format!("theo-move-{tag}-a.sqlite")), std::env::temp_dir().join(format!("theo-move-{tag}-b.sqlite")));
    let result = pollster::block_on(async {
        use theo_db::surreal::{sign_up, Server};
        let cong = RecordId::random().map_err(err)?;
        let id = cong.to_hex();
        let dir = demo_directory(36);
        // On a device.
        let local = Db::open(DbConfig::Sqlite { path: first.clone() }).await.map_err(err)?;
        let store = Store::create(local, &id, "Blue-Lamp-7", FAST, &details(), &dir).await.map_err(err)?;
        // To the Cloud: the same id, keys and password.
        let owner = sign_up(&Server::new(&url), &format!("mov-{}", &tag[..10]), "auth-key-move").await.map_err(err)?;
        owner.create_congregation(cong, "label").await.map_err(err)?;
        let config = DbConfig::Surreal { session: owner.clone(), congregation: id.clone() };
        let cloud = Db::open(config.clone()).await.map_err(err)?;
        let in_cloud = store.copy_to(cloud.clone(), details(), &dir).await.map_err(err)?;
        let (_, _, back) = Store::open(cloud.clone(), &id, "Blue-Lamp-7").await.map_err(err)?;
        if back != dir {
            return Err("the congregation changed on its way to the Cloud".into());
        }
        // And back to another device, then the Cloud copy goes.
        let mut changed = back.clone();
        changed.events.clear();
        let local2 = Db::open(DbConfig::Sqlite { path: second.clone() }).await.map_err(err)?;
        in_cloud.copy_to(local2.clone(), details(), &changed).await.map_err(err)?;
        Db::destroy(config).await.map_err(err)?;
        let (_, d, again) = Store::open(local2, &id, "Blue-Lamp-7").await.map_err(err)?;
        if d != details() || again != changed {
            return Err("the congregation changed on its way back".into());
        }
        if !owner.memberships().await.map_err(err)?.is_empty() {
            return Err("the Cloud copy is still there".into());
        }
        Ok(())
    });
    for p in [&first, &second] {
        for ext in ["sqlite", "sqlite-wal", "sqlite-shm"] {
            let _ = std::fs::remove_file(p.with_extension(ext));
        }
    }
    result
}

#[test]
fn a_whole_congregation_round_trips_through_surrealdb() -> Result<(), String> {
    let Some(url) = std::env::var("THEO_SURREAL_URL").ok().filter(|u| !u.is_empty()) else {
        eprintln!("THEO_SURREAL_URL is not set: skipped");
        return Ok(());
    };
    pollster::block_on(async {
        use theo_db::surreal::{sign_up, Server};
        let server = Server::new(&url);
        let tag = RecordId::random().map_err(err)?.to_hex();
        let owner = sign_up(&server, &format!("eng-{}", &tag[..10]), "auth-key-engines").await.map_err(err)?;
        let cong = RecordId::random().map_err(err)?;
        owner.create_congregation(cong, "label").await.map_err(err)?;
        let config = DbConfig::Surreal { session: owner.clone(), congregation: cong.to_hex() };
        let db = Db::open(config.clone()).await.map_err(err)?;
        round_trip(db.clone(), &cong.to_hex()).await?;

        // A member with only the schedule key, who may write events only, sees and changes just that.
        let (store, d, full) = Store::open(db.clone(), &cong.to_hex(), "Blue-Lamp-7").await.map_err(err)?;
        let keys = store.domain_secrets(&[DomainId::SCHEDULE]);
        let (member, _, mine) = Store::open_with_keys(db.clone(), &cong.to_hex(), keys).await.map_err(err)?;
        if !mine.people.is_empty() || mine.duty_types != full.duty_types {
            return Err("the member view is wrong".into());
        }
        member.set_writable(Some(["events".to_string()].into_iter().collect()));
        let mut edit = mine.clone();
        edit.events.clear();
        edit.duty_types.clear();
        member.save(d, &edit).await.map_err(err)?;
        let (_, _, after) = Store::open(db.clone(), &cong.to_hex(), "Blue-Lamp-7").await.map_err(err)?;
        if !after.events.is_empty() || after.duty_types != full.duty_types || after.people != full.people {
            return Err("the member changed more than events".into());
        }
        Db::destroy(config).await.map_err(err)?;
        Ok(())
    })
}
