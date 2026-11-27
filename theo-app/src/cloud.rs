//! The Cloud service (D-49): one SurrealDB server, chosen when the app is **built**, never typed
//! by users and never in the source code. The app only holds its address: no credentials.
//!
//! | Build variable | Meaning | Default |
//! |---|---|---|
//! | `THEO_CLOUD_URL` | The server: `https://host`, or the `wss://host/rpc` form SurrealDB shows | none: no Cloud option |
//! | `THEO_CLOUD_NS` | Namespace | `theo` |
//! | `THEO_CLOUD_DB` | Database | `theo` |
//!
//! Example: `THEO_CLOUD_URL=https://db.example.org dx serve`. Release builds take it from the
//! repository secret of the same name (`.github/workflows/release.yml`).

use theo_db::surreal::Server;

/// `wss://host/rpc` → `https://host`; `ws://` → `http://`; no trailing `/rpc` or `/`.
fn normalize(url: &str) -> String {
    let url = url.trim();
    let url = if let Some(rest) = url.strip_prefix("wss://") {
        format!("https://{rest}")
    } else if let Some(rest) = url.strip_prefix("ws://") {
        format!("http://{rest}")
    } else {
        url.to_string()
    };
    let url = url.trim_end_matches('/');
    url.strip_suffix("/rpc").unwrap_or(url).trim_end_matches('/').to_string()
}

fn configured(url: Option<&str>, ns: Option<&str>, db: Option<&str>) -> Option<Server> {
    let url = normalize(url?);
    if url.is_empty() {
        return None;
    }
    let mut server = Server::new(&url);
    if let Some(ns) = ns.map(str::trim).filter(|v| !v.is_empty()) {
        server.namespace = ns.to_string();
    }
    if let Some(db) = db.map(str::trim).filter(|v| !v.is_empty()) {
        server.database = db.to_string();
    }
    Some(server)
}

/// The Cloud server of this build, if it has one.
pub fn server() -> Option<Server> {
    configured(option_env!("THEO_CLOUD_URL"), option_env!("THEO_CLOUD_NS"), option_env!("THEO_CLOUD_DB"))
}

/// Does this build offer Cloud accounts and congregations?
pub fn available() -> bool {
    server().is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addresses_are_normalized() {
        assert_eq!(normalize("wss://abc.example.earth/rpc"), "https://abc.example.earth");
        assert_eq!(normalize(" https://db.example.org/ "), "https://db.example.org");
        assert_eq!(normalize("ws://127.0.0.1:8000/rpc/"), "http://127.0.0.1:8000");
        let s = configured(Some("wss://abc.example.earth/rpc"), Some("prod"), None).unwrap_or_else(|| Server::new("x"));
        assert_eq!((s.url.as_str(), s.namespace.as_str(), s.database.as_str()), ("https://abc.example.earth", "prod", "theo"));
        assert!(configured(Some("  "), None, None).is_none());
        assert!(configured(None, None, None).is_none());
    }
}
