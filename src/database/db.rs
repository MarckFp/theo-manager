use surrealdb::engine::any::{self, Any};
use surrealdb::Surreal;
use surrealdb::Result;
use once_cell::sync::OnceCell;

// === Configuration ===

const APP_DATABASE: &str = "theo_manager_db";

// Desktop uses a local data directory
#[cfg(all(not(target_arch = "wasm32"), not(target_os = "android"), not(target_os = "ios")))]
const DB_PATH: &str = "rocksdb://data/theo_manager_db_desktop";

// Android/iOS use app-specific data directory (will be created in app's private storage)
#[cfg(any(target_os = "android", target_os = "ios"))]
const DB_PATH: &str = "rocksdb://theo_manager_db";

// Web uses IndexedDB
#[cfg(target_arch = "wasm32")]
const DB_PATH: &str = "indxdb://theo_manager_db";

const REMOTE_DB_ENDPOINT: &str = "wss://database.theo-manager.com";

static DB: OnceCell<Surreal<Any>> = OnceCell::new();

pub async fn init(namespace: &str, remote_db: bool) -> Result<Surreal<Any>> {
    let connection_string: &str = if remote_db {
        REMOTE_DB_ENDPOINT
    } else {
        DB_PATH
    };

    println!("Initializing database with connection: {}", connection_string);
    
    match any::connect(connection_string).await {
        Ok(db) => {
            println!("Database connected successfully");
            match db.use_ns(namespace).use_db(APP_DATABASE).await {
                Ok(_) => {
                    println!("Database namespace and database set successfully");
                    Ok(db)
                }
                Err(e) => {
                    eprintln!("Failed to set namespace/database: {:?}", e);
                    Err(e)
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to connect to database: {:?}", e);
            Err(e)
        }
    }
}

pub async fn get_db() -> Result<&'static Surreal<Any>> {
    if DB.get().is_none() {
        println!("Initializing database for the first time...");
        match init("parla_norte", false).await {
            Ok(db) => {
                println!("Database initialized successfully");
                let _ = DB.set(db); // Ignore error if already set
            }
            Err(e) => {
                eprintln!("Failed to initialize database: {:?}", e);
                return Err(e);
            }
        }
    }
    
    DB.get().ok_or_else(|| {
        eprintln!("Database not available in OnceCell");
        surrealdb::Error::Db(surrealdb::error::Db::DbEmpty)
    })
}