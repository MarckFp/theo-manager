use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use surrealdb::Surreal;
use surrealdb::engine::any::Any;

use crate::crypto::SessionCrypto;

/// Unified database handle. Works transparently with every backend:
/// embedded (offline) and remote WebSocket (online). `Surreal<Any>` is
/// internally `Arc`-wrapped, so cloning is cheap.
pub type Db = Surreal<Any>;

pub const NS: &str = "theo";
pub const DB_NAME: &str = "manager";

/// The SurrealDB Cloud endpoint used for all online connections.
/// Set the `SURREAL_CLOUD_ENDPOINT` environment variable at compile time.
/// Falls back to a local dev endpoint when not set.
pub const CLOUD_ENDPOINT: &str = match option_env!("SURREAL_CLOUD_ENDPOINT") {
    Some(e) => e,
    None => "ws://localhost:8000",
};

// ---------------------------------------------------------------------------
// Mode & configuration
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DatabaseMode {
    Offline,
    Online,
}

/// Connection settings for a remote SurrealDB instance.
///
/// # Security notes for a 100 % client-side (Cloudflare Pages) deployment
///
/// * `endpoint` is **not a secret** – treat it like a hostname. Security is
///   enforced entirely by SurrealDB's auth layer; the endpoint alone gives no
///   access to data.
/// * **Never persist passwords.** After `connect_online()` succeeds, call
///   `db.authenticate(token)` on subsequent launches using a JWT you stored
///   in `localStorage`. The JWT is short-lived and does not reveal the
///   original password.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OnlineConfig {
    /// Congregation UUID — used as the SurrealDB namespace.
    pub congregation_uid: String,
    /// Stored only to pre-fill the login form; never the password itself.
    pub username: String,
}

// ---------------------------------------------------------------------------
// Runtime state (provided as Dioxus context)
// ---------------------------------------------------------------------------

/// Active connection + metadata. Provided to the component tree as
/// `Signal<AppDatabase>` via [`DatabaseProvider`].
#[derive(Clone)]
pub struct AppDatabase {
    pub db: Option<Db>,
    pub mode: DatabaseMode,
    /// Present when `mode == Online`; used to restore the login form on
    /// re-authentication after a JWT expires.
    pub config: Option<OnlineConfig>,
    /// Congregation UUID — used as the SurrealDB namespace for both offline
    /// and online modes. `None` until onboarding is complete.
    pub congregation_uid: Option<String>,
}

impl Default for AppDatabase {
    fn default() -> Self {
        Self {
            db: None,
            mode: DatabaseMode::Offline,
            config: None,
            congregation_uid: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Connection helpers
// ---------------------------------------------------------------------------

/// Open an embedded (offline) database.
///
/// | Target  | Backend   | Data persistence          |
/// |---------|-----------|---------------------------|
/// | wasm32  | IndexedDB | Survives page reloads     |
/// | native  | in-memory | Lost on process exit      |
///
/// **Native note:** uses `kv-mem` (nightly Rust required via `rust-toolchain.toml`).
/// Data is in-memory only; it does not persist across restarts on native targets.
#[cfg(target_arch = "wasm32")]
pub async fn connect_offline(congregation_uid: &str) -> surrealdb::Result<Db> {
    use std::time::Duration;
    use surrealdb::opt::Config;
    // surrealdb 3.1.2 bug: `update_node_with_timeout` calls
    // `tokio::time::Instant::now()` without a `#[cfg(not(target_family="wasm"))]`
    // guard, which panics on `wasm32-unknown-unknown`. The only caller is the
    // node-membership-refresh background task. Setting the interval to just
    // under `i32::MAX` ms (~24 days) prevents that task from ever firing during
    // a real browser session. `wasmtimer` uses
    // `i32::try_from(millis).unwrap_or(0)`, so values ≤ i32::MAX are safe.
    let config =
        Config::default().node_membership_refresh_interval(Duration::from_millis(i32::MAX as u64));
    let store_name = format!("indxdb://theo_{congregation_uid}");
    let db = surrealdb::engine::any::connect((store_name.as_str(), config)).await?;
    db.use_ns(congregation_uid).use_db(DB_NAME).await?;
    Ok(db)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn connect_offline(congregation_uid: &str) -> surrealdb::Result<Db> {
    let db = surrealdb::engine::any::connect("mem://").await?;
    db.use_ns(congregation_uid).use_db(DB_NAME).await?;
    Ok(db)
}

/// Open an authenticated connection to the hardcoded SurrealDB Cloud endpoint.
/// Uses RECORD-level auth (DEFINE ACCESS TYPE RECORD).
/// `password` is used for this call only and is **never** stored.
pub async fn connect_online(config: &OnlineConfig, password: &str) -> surrealdb::Result<Db> {
    let db = surrealdb::engine::any::connect(CLOUD_ENDPOINT).await?;
    db.signin(surrealdb::opt::auth::Record {
        namespace: config.congregation_uid.clone(),
        database: DB_NAME.to_string(),
        access: "user".to_string(),
        params: serde_json::json!({
            "username": config.username,
            "password": password,
        }),
    })
    .await?;
    db.use_ns(&config.congregation_uid).use_db(DB_NAME).await?;
    Ok(db)
}

/// Register a new admin user via SurrealDB RECORD access signup.
/// Used once during onboarding to create the first user.
pub async fn signup_online(
    congregation_uid: &str,
    username: &str,
    email: &str,
    password: &str,
) -> surrealdb::Result<Db> {
    let db = surrealdb::engine::any::connect(CLOUD_ENDPOINT).await?;
    db.signup(surrealdb::opt::auth::Record {
        namespace: congregation_uid.to_string(),
        database: DB_NAME.to_string(),
        access: "user".to_string(),
        params: serde_json::json!({
            "username": username,
            "email": email,
            "password": password,
        }),
    })
    .await?;
    db.use_ns(congregation_uid).use_db(DB_NAME).await?;
    Ok(db)
}

// ---------------------------------------------------------------------------
// Dioxus integration
// ---------------------------------------------------------------------------

/// Mount near the root of the app to make `Signal<AppDatabase>` available
/// to every descendant via [`use_db`].
///
/// ```rust
/// fn App() -> Element {
///     rsx! {
///         DatabaseProvider {
///             Router::<Route> {}
///         }
///     }
/// }
/// ```
#[component]
pub fn DatabaseProvider(children: Element) -> Element {
    use_context_provider(|| Signal::new(AppDatabase::default()));
    use_context_provider(|| Signal::new(SessionCrypto::default()));
    rsx! {
        {children}
    }
}

/// Retrieve the database context signal from any descendant component.
/// Panics if [`DatabaseProvider`] is not an ancestor.
pub fn use_db() -> Signal<AppDatabase> {
    use_context::<Signal<AppDatabase>>()
}

/// Retrieve the encryption context signal from any descendant component.
/// Panics if [`DatabaseProvider`] is not an ancestor.
pub fn use_crypto() -> Signal<SessionCrypto> {
    use_context::<Signal<SessionCrypto>>()
}
