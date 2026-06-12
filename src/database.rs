use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use surrealdb::Surreal;
use surrealdb::engine::any::Any;
use surrealdb::types::RecordId;

use crate::crypto::SessionCrypto;

use std::sync::Arc;

// ---------------------------------------------------------------------------
// LocalStorage helpers (JS interop via document::eval)
// ---------------------------------------------------------------------------

pub async fn ls_get(key: &str) -> Option<String> {
    let js = format!(
        "try {{ dioxus.send(localStorage.getItem({key:?})); }} catch(e) {{ dioxus.send(null); }}"
    );
    let mut eval = document::eval(&js);
    eval.recv::<serde_json::Value>().await.ok().and_then(|v| match v {
        serde_json::Value::String(s) => Some(s),
        _ => None,
    })
}

pub fn ls_set(key: &str, value: &str) {
    let js = format!("try {{ localStorage.setItem({key:?}, {value:?}); }} catch(e) {{}}");
    let _ = document::eval(&js);
}

pub fn ls_remove(key: &str) {
    let js = format!("try {{ localStorage.removeItem({key:?}); }} catch(e) {{}}");
    let _ = document::eval(&js);
}

/// Unified database handle. Works transparently with every backend:
/// embedded (offline) and remote WebSocket (online).
/// Wrapped in Arc to prevent SurrealDB from generating a new session ID
/// on every clone (which causes intermittent "Session not found" race conditions).
pub type Db = Arc<Surreal<Any>>;

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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Workspace {
    pub uid: String,
    pub name: String,
    pub mode: DatabaseMode,
    pub username: Option<String>,
    #[serde(default)]
    pub theme: String,
    #[serde(default)]
    pub accent_color: String,
}

pub async fn get_workspaces() -> Vec<Workspace> {
    if let Some(json) = ls_get("theo_workspaces").await {
        serde_json::from_str(&json).unwrap_or_default()
    } else {
        vec![]
    }
}

pub async fn add_workspace(workspace: Workspace) {
    let mut wks = get_workspaces().await;
    if let Some(pos) = wks.iter().position(|w| w.uid == workspace.uid) {
        wks[pos] = workspace;
    } else {
        wks.push(workspace);
    }
    if let Ok(json) = serde_json::to_string(&wks) {
        ls_set("theo_workspaces", &json);
    }
}

pub async fn remove_workspace(uid: &str) {
    let mut wks = get_workspaces().await;
    wks.retain(|w| w.uid != uid);
    if let Ok(json) = serde_json::to_string(&wks) {
        ls_set("theo_workspaces", &json);
    }
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
    /// The currently active congregation in the UI.
    pub active_congregation_id: Option<RecordId>,
    /// Kept alive to prevent WASM panic on drop
    pub leaked_dbs: Vec<Db>,
}

impl Default for AppDatabase {
    fn default() -> Self {
        Self {
            db: None,
            mode: DatabaseMode::Offline,
            config: None,
            congregation_uid: None,
            active_congregation_id: None,
            leaked_dbs: vec![],
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
    // surrealdb-core 3.1.2 bug: several background tasks call
    // `tokio::time::Instant::now()` without a `#[cfg(not(target_family="wasm"))]`
    // guard, which panics on `wasm32-unknown-unknown`.
    //
    // Affected code paths:
    //   • `update_node_with_timeout`  — fired by node_membership_refresh task
    //   • `has_lease()` retry sleep   — fired by changefeed_gc / node_check /
    //                                   node_cleanup tasks on DB errors
    //
    // Fix: set all four configurable intervals to i32::MAX ms (~24.8 days) so
    // none of these background tasks ever fire during a normal browser session.
    // `wasmtimer` converts Duration to ms via `i32::try_from(millis).unwrap_or(0)`,
    // so values ≤ i32::MAX are safe.
    //
    // Note: `Datastore::shutdown()` → `delete_node_with_timeout()` still calls
    // `Instant::now()` — this fires when the Db is dropped (hot reload, page
    // refresh). That is an upstream bug with no config workaround; it is benign
    // in dev (panic = "unwind") since the wasm instance is being torn down anyway.
    const MAX_INTERVAL: Duration = Duration::from_millis(i32::MAX as u64);
    let config = Config::default()
        .query_timeout(None)
        .transaction_timeout(None)
        .node_membership_refresh_interval(MAX_INTERVAL)
        .node_membership_check_interval(MAX_INTERVAL)
        .node_membership_cleanup_interval(MAX_INTERVAL)
        .changefeed_gc_interval(MAX_INTERVAL);
    let store_name = format!("indxdb://theo_{congregation_uid}");
    let db = surrealdb::engine::any::connect((store_name.as_str(), config)).await?;
    
    // WORKAROUND: Give SurrealDB's async channels time to process `SessionId::Initial`
    // before we send the `USE NS` query. Under wasm, both channels are polled randomly,
    // which can lead to a "Session not found" race-condition panic.
    #[cfg(target_arch = "wasm32")]
    gloo_timers::future::sleep(std::time::Duration::from_millis(350)).await;

    db.use_ns(congregation_uid).use_db(DB_NAME).await?;
    Ok(Arc::new(db))
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn connect_offline(congregation_uid: &str) -> surrealdb::Result<Db> {
    let db = surrealdb::engine::any::connect("mem://").await?;
    db.use_ns(congregation_uid).use_db(DB_NAME).await?;
    Ok(Arc::new(db))
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

    #[cfg(target_arch = "wasm32")]
    gloo_timers::future::sleep(std::time::Duration::from_millis(350)).await;

    db.use_ns(&config.congregation_uid).use_db(DB_NAME).await?;
    Ok(Arc::new(db))
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

    #[cfg(target_arch = "wasm32")]
    gloo_timers::future::sleep(std::time::Duration::from_millis(350)).await;

    db.use_ns(congregation_uid).use_db(DB_NAME).await?;
    Ok(Arc::new(db))
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
