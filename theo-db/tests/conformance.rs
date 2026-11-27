//! Every native engine passes the conformance suite.

use theo_db::{conformance, Db, DbConfig, NewRecord, RecordId};

#[test]
fn memory_engine_conforms() -> Result<(), String> {
    pollster::block_on(async {
        let db = Db::open(DbConfig::Memory).await.map_err(|e| e.to_string())?;
        conformance::run(&db).await
    })
}

fn temp_path(name: &str) -> std::path::PathBuf {
    let n = RecordId::random().map(|r| r.to_hex()).unwrap_or_default();
    std::env::temp_dir().join(format!("theo-db-test-{name}-{n}")).join("congregation.sqlite")
}

#[test]
fn sqlite_engine_conforms_and_keeps_data_after_reopening() -> Result<(), String> {
    let path = temp_path("conform");
    let result = pollster::block_on(async {
        let config = DbConfig::Sqlite { path: path.clone() };
        let db = Db::open(config.clone()).await.map_err(|e| e.to_string())?;
        conformance::run(&db).await?;
        let id = RecordId::random().map_err(|e| e.to_string())?;
        db.put(&NewRecord { collection: "people", record_id: id, scope_id: None, envelope: b"kept" }, None).await.map_err(|e| e.to_string())?;
        let seq = db.last_seq().await.map_err(|e| e.to_string())?;
        drop(db);
        // Reopened: same data, migrations not applied twice, change numbers go on.
        let db = Db::open(config.clone()).await.map_err(|e| e.to_string())?;
        let got = db.get("people", id).await.map_err(|e| e.to_string())?;
        if got.map(|r| r.envelope) != Some(b"kept".to_vec()) {
            return Err("the record is not kept".into());
        }
        if db.last_seq().await.map_err(|e| e.to_string())? != seq || db.read_header().await.map_err(|e| e.to_string())?.is_none() {
            return Err("the change number or header is not kept".into());
        }
        drop(db);
        Db::destroy(config.clone()).await.map_err(|e| e.to_string())?;
        if path.exists() {
            return Err("destroy left the file".into());
        }
        Ok(())
    });
    let _ = std::fs::remove_dir_all(path.parent().unwrap_or(&path));
    result
}
