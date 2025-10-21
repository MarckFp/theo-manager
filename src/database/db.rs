use surrealdb::engine::any::{self, Any};
use surrealdb::Surreal;
use surrealdb::Result;
use once_cell::sync::OnceCell;

// === Configuration ===

const APP_DATABASE: &str = "theo_manager_db";

const ROCKSDB_FILE_PATH: &str = "rocksdb://data/theo_manager_db";
const INDEXEDDB_NAME: &str = "indxdb://theo_manager_db";
const REMOTE_DB_ENDPOINT: &str = "wss://database.theo-manager.com";

static DB: OnceCell<Surreal<Any>> = OnceCell::new();

pub async fn init(namespace: &str, remote_db: bool) -> Result<Surreal<Any>> {

    let connection_string: &str = if remote_db {
        REMOTE_DB_ENDPOINT
    } else {
        #[cfg(target_arch = "wasm32")]
        { INDEXEDDB_NAME }
        #[cfg(not(target_arch = "wasm32"))]
        { ROCKSDB_FILE_PATH }
    };

    let db = any::connect(connection_string).await?;
    db.use_ns(namespace).use_db(APP_DATABASE).await?;
    Ok(db)
}

pub async fn get_db() -> Result<&'static Surreal<Any>> {
    if DB.get().is_none() {
        let db = init("parla_norte", false).await?;
        DB.set(db).unwrap();
    }
    Ok(DB.get().unwrap())
}