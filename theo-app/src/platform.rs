//! Platform capabilities.
//!
//! Biometric unlock (REQUIREMENTS.md R-312, extended): the real implementation keeps a key that
//! unwraps the congregation key in the platform's secure hardware, and releases it only after a
//! biometric check:
//!
//! | Platform | Mechanism (later, in a platform adapter) |
//! |---|---|
//! | Web | WebAuthn passkey with the PRF extension (the key never leaves the authenticator) |
//! | Windows | Windows Hello (`KeyCredentialManager`) |
//! | macOS / iOS | Keychain item with biometry access control (Touch ID / Face ID) |
//! | Android | Android Keystore key bound to `BiometricPrompt` |
//! | Linux | Not available (no standard API); password only |
//!
//! In this scaffold the check is simulated (see `state::AppState::unlock_with_biometrics`).

/// True in the browser build. The landing page and "Home" links exist only there (R-063);
/// installed apps (Windows, macOS, Linux, iOS, Android) open on the log-in screen.
pub const IS_WEB: bool = cfg!(target_arch = "wasm32");

use dioxus::prelude::document;

/// The web app runs installed (a PWA opened from the home screen or the app list), not in a tab.
pub fn is_installed_web_app() -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window()
            .and_then(|w| w.match_media("(display-mode: standalone)").ok().flatten())
            .is_some_and(|m| m.matches())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        false
    }
}

// ---------------------------------------------------------------------------------------------
// Biometric unlock (UI level only)
// ---------------------------------------------------------------------------------------------
//
// ⚠️ MOCK security: the check only answers "is it the user of this device?". It does not unlock
// any key yet (later the root key is sealed by the platform's secure hardware).
//
// Web and installed web app: a real passkey (WebAuthn, platform authenticator with user
// verification), so the browser shows its fingerprint, face or PIN prompt. The credential id is
// kept per congregation in this browser's storage, so the choice survives reopening the app.
// Desktop and mobile apps: a short simulated check until the native plugins exist.

/// What remembers that a congregation unlocks with biometrics: the passkey id, or "simulated".
pub const SIMULATED: &str = "simulated";

fn storage_key(congregation: &str) -> String {
    format!("theo-manager.biometric.{congregation}")
}

/// The remembered biometric unlock of a congregation on this device, if any.
pub fn stored_biometric(congregation: &str) -> Option<String> {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window()
            .and_then(|w| w.local_storage().ok().flatten())
            .and_then(|s| s.get_item(&storage_key(congregation)).ok().flatten())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = storage_key(congregation);
        None
    }
}

/// Remembers (or, with `None`, forgets) the biometric unlock of a congregation on this device.
pub fn store_biometric(congregation: &str, credential: Option<&str>) {
    #[cfg(target_arch = "wasm32")]
    if let Some(s) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
        let key = storage_key(congregation);
        let _ = match credential {
            Some(c) => s.set_item(&key, c),
            None => s.remove_item(&key),
        };
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (storage_key(congregation), credential);
}

async fn simulated_prompt() {
    theo_ui::sleep(std::time::Duration::from_millis(700)).await;
}

/// Sets up biometric unlock: on the web the browser asks for the fingerprint, face or PIN and
/// creates a passkey. Returns what to remember, or a message when it did not work.
pub async fn biometric_register(username: &str, display_name: &str) -> Result<String, String> {
    if !cfg!(target_arch = "wasm32") {
        simulated_prompt().await;
        return Ok(SIMULATED.into());
    }
    let name = serde_json::to_string(username).unwrap_or_else(|_| "\"user\"".into());
    let display = serde_json::to_string(display_name).unwrap_or_else(|_| "\"User\"".into());
    let script = format!(
        r#"
        if (!window.PublicKeyCredential || !(await PublicKeyCredential.isUserVerifyingPlatformAuthenticatorAvailable())) {{
            return {{ error: "This device has no fingerprint, face or PIN unlock for the browser." }};
        }}
        // Passkeys need a real site (not an IP address or an app's own scheme): simulate there.
        const simulate = ["SecurityError", "NotSupportedError"];
        try {{
            const random = (n) => crypto.getRandomValues(new Uint8Array(n));
            const credential = await navigator.credentials.create({{ publicKey: {{
                challenge: random(32),
                rp: {{ name: "Theo Manager" }},
                user: {{ id: random(16), name: {name}, displayName: {display} }},
                pubKeyCredParams: [{{ type: "public-key", alg: -7 }}, {{ type: "public-key", alg: -257 }}],
                authenticatorSelection: {{ authenticatorAttachment: "platform", userVerification: "required", residentKey: "discouraged" }},
                attestation: "none",
                timeout: 60000,
            }} }});
            return {{ id: btoa(String.fromCharCode(...new Uint8Array(credential.rawId))) }};
        }} catch (e) {{
            if (simulate.includes(e.name)) return {{ simulated: true }};
            return {{ error: e.name === "NotAllowedError" ? "Cancelled." : e.message }};
        }}
        "#
    );
    let value = document::eval(&script).await.map_err(|e| e.to_string())?;
    if value.get("simulated").is_some() {
        simulated_prompt().await;
        return Ok(SIMULATED.into());
    }
    match (value.get("id").and_then(|v| v.as_str()), value.get("error").and_then(|v| v.as_str())) {
        (Some(id), _) => Ok(id.to_string()),
        (_, Some(e)) => Err(e.to_string()),
        _ => Err("The browser did not answer.".into()),
    }
}

/// Asks for the fingerprint, face or PIN of a remembered biometric unlock. True when it is the user.
pub async fn biometric_verify(credential: &str) -> bool {
    if credential == SIMULATED || !cfg!(target_arch = "wasm32") {
        simulated_prompt().await;
        return true;
    }
    let id = serde_json::to_string(credential).unwrap_or_default();
    let script = format!(
        r#"
        try {{
            const id = Uint8Array.from(atob({id}), (c) => c.charCodeAt(0));
            await navigator.credentials.get({{ publicKey: {{
                challenge: crypto.getRandomValues(new Uint8Array(32)),
                allowCredentials: [{{ type: "public-key", id }}],
                userVerification: "required",
                timeout: 60000,
            }} }});
            return true;
        }} catch (e) {{
            return false;
        }}
        "#
    );
    document::eval(&script).await.ok().and_then(|v| v.as_bool()).unwrap_or(false)
}

/// Name of the biometric method on this platform, or `None` if it has none.
pub fn biometric_label() -> Option<&'static str> {
    if cfg!(target_arch = "wasm32") {
        Some("passkey (fingerprint or face)")
    } else if cfg!(target_os = "windows") {
        Some("Windows Hello")
    } else if cfg!(target_os = "macos") {
        Some("Touch ID")
    } else if cfg!(target_os = "ios") {
        Some("Face ID or Touch ID")
    } else if cfg!(target_os = "android") {
        Some("fingerprint or face unlock")
    } else {
        None
    }
}

/// Saves an exported file. Web: a download. Desktop: the Downloads folder. Phones: not yet
/// (needs the platform share sheet). Returns a message for the user.
pub fn save_file(name: &str, mime: &str, bytes: &[u8]) -> Result<String, String> {
    save_file_impl(name, mime, bytes)
}

#[cfg(target_arch = "wasm32")]
fn save_file_impl(name: &str, mime: &str, bytes: &[u8]) -> Result<String, String> {
    use wasm_bindgen::JsCast;
    let fail = |_| "The browser did not allow the download.".to_string();
    let parts = js_sys::Array::of1(&js_sys::Uint8Array::from(bytes));
    let opts = web_sys::BlobPropertyBag::new();
    opts.set_type(mime);
    let blob = web_sys::Blob::new_with_u8_array_sequence_and_options(&parts, &opts).map_err(fail)?;
    let url = web_sys::Url::create_object_url_with_blob(&blob).map_err(fail)?;
    let document = web_sys::window().and_then(|w| w.document()).ok_or("No document.")?;
    let a: web_sys::HtmlAnchorElement = document.create_element("a").map_err(fail)?.dyn_into().map_err(|_| "No link element.")?;
    a.set_href(&url);
    a.set_download(name);
    a.click();
    // Give the browser time to start the download before freeing the data.
    dioxus::prelude::spawn(async move {
        theo_ui::sleep(std::time::Duration::from_secs(5)).await;
        let _ = web_sys::Url::revoke_object_url(&url);
    });
    Ok(format!("Downloaded {name}."))
}

#[cfg(all(not(target_arch = "wasm32"), not(any(target_os = "android", target_os = "ios"))))]
fn save_file_impl(name: &str, _mime: &str, bytes: &[u8]) -> Result<String, String> {
    let home = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")).ok_or("No home folder.")?;
    let downloads = std::path::Path::new(&home).join("Downloads");
    let dir = if downloads.is_dir() { downloads } else { std::path::PathBuf::from(home) };
    let (stem, ext) = name.rsplit_once('.').unwrap_or((name, ""));
    let mut path = dir.join(name);
    let mut i = 2;
    while path.exists() {
        path = dir.join(format!("{stem} ({i}).{ext}"));
        i += 1;
    }
    std::fs::write(&path, bytes).map_err(|e| format!("Could not save the file: {e}"))?;
    Ok(format!("Saved to {}.", path.display()))
}

#[cfg(all(not(target_arch = "wasm32"), any(target_os = "android", target_os = "ios")))]
fn save_file_impl(_name: &str, _mime: &str, _bytes: &[u8]) -> Result<String, String> {
    Err("Exporting is not available on phones yet.".into())
}

/// The address to put in front of invite links: on the web, this page's origin. The installed
/// apps have no web address: their links start with `theomanager:` and are pasted on the join
/// screen (the link's fragment is all that counts).
pub fn app_address() -> String {
    #[cfg(target_arch = "wasm32")]
    if let Some(origin) = web_sys::window().and_then(|w| w.location().origin().ok()) {
        return origin;
    }
    "theomanager:".into()
}

/// The fragment (`#...`) of the address the app was opened with. The router drops it when it
/// starts, so `main` keeps it here first, for the join page.
static FRAGMENT: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

/// Keeps the invite fragment the app was opened with (call before the app starts). On the web:
/// the page's fragment, which is also removed from the address bar and the history, so an invite
/// token does not stay on screen. Installed apps: a launch argument with an invite link
/// (`theomanager:/join#s=…&t=…`), for when the system opens the app with one.
pub fn capture_location_fragment() {
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(arg) = std::env::args().skip(1).find(|a| a.contains("#s=") && a.contains("&t="))
        && let Ok(mut f) = FRAGMENT.lock()
    {
        *f = arg.split_once('#').map(|(_, frag)| format!("#{frag}"));
    }
    #[cfg(target_arch = "wasm32")]
    {
        let Some(window) = web_sys::window() else { return };
        let Some(hash) = window.location().hash().ok().filter(|h| h.len() > 1) else { return };
        let path = window.location().pathname().unwrap_or_default();
        if let Ok(history) = window.history() {
            let _ = history.replace_state_with_url(&wasm_bindgen::JsValue::NULL, "", Some(&path));
        }
        if let Ok(mut f) = FRAGMENT.lock() {
            *f = Some(hash);
        }
    }
}

/// Is there an invite fragment waiting for the join page?
pub fn has_location_fragment() -> bool {
    FRAGMENT.lock().is_ok_and(|f| f.is_some())
}

/// Takes the fragment kept by [`capture_location_fragment`] (once).
pub fn take_location_fragment() -> Option<String> {
    FRAGMENT.lock().ok().and_then(|mut f| f.take())
}

/// Sets `<html lang>` (screen readers and search engines). Web only; native windows do not need it.
pub fn set_document_language(code: &str) {
    #[cfg(target_arch = "wasm32")]
    if let Some(root) = web_sys::window().and_then(|w| w.document()).and_then(|d| d.document_element()) {
        let _ = root.set_attribute("lang", code);
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = code;
}

/// Draws map tiles into one JPEG image, for the territory card PDF. `tiles` are (address,
/// left, top) in pixels. None when no tile could be read (offline, or the provider does not
/// allow it). Exception to P1: only a canvas can read tile pixels inside the webview; this short
/// script draws them and gives back the image. It runs the same on web, desktop and mobile.
pub async fn render_tiles(tiles: &[(String, f64, f64)], width: u32, height: u32) -> Option<Vec<u8>> {
    if tiles.is_empty() {
        return None;
    }
    let list = serde_json::to_string(tiles).ok()?;
    let script = format!(
        r##"
        const tiles = {list};
        const canvas = document.createElement("canvas");
        canvas.width = {width};
        canvas.height = {height};
        const ctx = canvas.getContext("2d");
        ctx.fillStyle = "#f2f3f5";
        ctx.fillRect(0, 0, {width}, {height});
        const load = (t) => new Promise((ok) => {{
            const img = new Image();
            img.crossOrigin = "anonymous";
            img.onload = () => {{ ctx.drawImage(img, t[1], t[2], 256, 256); ok(true); }};
            img.onerror = () => ok(false);
            img.src = t[0];
        }});
        const done = await Promise.all(tiles.map(load));
        if (!done.some(Boolean)) return null;
        try {{ return canvas.toDataURL("image/jpeg", 0.88).split(",")[1]; }} catch (e) {{ return null; }}
        "##
    );
    let value = document::eval(&script).await.ok()?;
    crate::territory_card::base64_decode(value.as_str()?)
}

/// Copies text to the clipboard (the webview's clipboard API, the same on every platform).
pub async fn copy_text(text: &str) -> bool {
    let Ok(quoted) = serde_json::to_string(text) else { return false };
    document::eval(&format!("await navigator.clipboard.writeText({quoted}); return true;")).await.is_ok()
}

/// A link with directions to a place. Web browsers (also on phones, where it opens the Google Maps
/// app when it is installed) and the desktop app: Google Maps. The Android app: the default map
/// app (`geo:`). The iPhone and iPad app: Apple Maps. Only opened when the user taps it.
pub fn directions_url(lat: f64, lon: f64, label: &str) -> String {
    let google = format!("https://www.google.com/maps/dir/?api=1&destination={lat:.6},{lon:.6}");
    let name: String = label.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '+' }).collect();
    if cfg!(target_arch = "wasm32") {
        google
    } else if cfg!(target_os = "android") {
        format!("geo:{lat:.6},{lon:.6}?q={lat:.6},{lon:.6}({name})")
    } else if cfg!(target_os = "ios") {
        format!("https://maps.apple.com/?daddr={lat:.6},{lon:.6}&q={name}")
    } else {
        google
    }
}

/// The app's own folder on desktop and phone: the congregations' SQLite files and the device
/// settings. Desktop: the user's data folder (`%APPDATA%\TheoManager`, `~/Library/Application
/// Support/TheoManager`, `~/.local/share/TheoManager`). Android: the app's private files folder.
#[cfg(not(target_arch = "wasm32"))]
pub fn data_dir() -> std::path::PathBuf {
    if cfg!(target_os = "android") {
        return std::path::PathBuf::from("/data/data/app.theomanager/files");
    }
    dirs::data_dir().unwrap_or_else(std::env::temp_dir).join("TheoManager")
}

/// Device settings on desktop and phone: one small JSON file, read once and written on change.
#[cfg(not(target_arch = "wasm32"))]
mod device_file {
    use std::collections::BTreeMap;
    use std::sync::Mutex;

    static VALUES: Mutex<Option<BTreeMap<String, String>>> = Mutex::new(None);

    fn path() -> std::path::PathBuf {
        super::data_dir().join("device.json")
    }

    fn with<T>(f: impl FnOnce(&mut BTreeMap<String, String>) -> T) -> T {
        let mut guard = VALUES.lock().unwrap_or_else(|e| e.into_inner());
        let values = guard.get_or_insert_with(|| std::fs::read_to_string(path()).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default());
        f(values)
    }

    pub fn get(key: &str) -> Option<String> {
        with(|v| v.get(key).cloned())
    }

    pub fn set(key: &str, value: Option<&str>) {
        with(|v| {
            match value {
                Some(x) => v.insert(key.into(), x.into()),
                None => v.remove(key),
            };
            let _ = std::fs::create_dir_all(super::data_dir());
            if let Ok(text) = serde_json::to_string_pretty(v) {
                let _ = std::fs::write(path(), text);
            }
        });
    }
}

/// A small value remembered by this device: the browser's storage on the web and the installed
/// web app, a settings file on desktop and phone. Never congregation data (that is encrypted in
/// `theo-db`): only settings of the device, the list of accounts and congregations on it, which
/// congregation is open and until when, and which notices were seen.
pub fn device_get(key: &str) -> Option<String> {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window().and_then(|w| w.local_storage().ok().flatten()).and_then(|s| s.get_item(key).ok().flatten())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        device_file::get(key)
    }
}

/// Remembers (or, with `None`, forgets) a value on this device. See [`device_get`].
pub fn device_set(key: &str, value: Option<&str>) {
    #[cfg(target_arch = "wasm32")]
    if let Some(s) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
        let _ = match value {
            Some(v) => s.set_item(key, v),
            None => s.remove_item(key),
        };
    }
    #[cfg(not(target_arch = "wasm32"))]
    device_file::set(key, value);
}

/// Seconds since 1970.
pub fn now_secs() -> u64 {
    web_time::SystemTime::now().duration_since(web_time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// Downloads a file (the meeting workbook, R-034). Desktop and phone: in a background thread, so
/// the app keeps working. Web: the browser's own download (some servers do not allow it).
pub async fn fetch_bytes(url: &str) -> Result<Vec<u8>, String> {
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast;
        let window = web_sys::window().ok_or("No browser window.")?;
        let resp = wasm_bindgen_futures::JsFuture::from(window.fetch_with_str(url)).await.map_err(|_| "The browser could not download it.".to_string())?;
        let resp: web_sys::Response = resp.dyn_into().map_err(|_| "No answer.".to_string())?;
        if !resp.ok() {
            return Err(format!("The server answered {}.", resp.status()));
        }
        let buffer = wasm_bindgen_futures::JsFuture::from(resp.array_buffer().map_err(|_| "No data.".to_string())?).await.map_err(|_| "No data.".to_string())?;
        Ok(js_sys::Uint8Array::new(&buffer).to_vec())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let url = url.to_string();
        let (tx, rx) = futures_channel::oneshot::channel();
        std::thread::spawn(move || {
            let result = (|| -> Result<Vec<u8>, String> {
                let mut resp = ureq::get(&url)
                    .header("User-Agent", concat!("TheoManager/", env!("CARGO_PKG_VERSION"), " (+https://github.com/MarckFp/theo-manager)"))
                    .call()
                    .map_err(|e| e.to_string())?;
                resp.body_mut().with_config().limit(60 * 1024 * 1024).read_to_vec().map_err(|e| e.to_string())
            })();
            let _ = tx.send(result);
        });
        rx.await.map_err(|_| "The download stopped.".to_string())?
    }
}

// ---------------------------------------------------------------------------------------------
// Keys kept on this device (REQUIREMENTS.md 6.5): *Stay unlocked* and passkey unlock
// ---------------------------------------------------------------------------------------------

/// Seals `plain` with this device's key, for `purpose`. Web: a non-extractable AES-GCM key kept
/// in IndexedDB (scripts can use it, not read it out). Apps: a random key file in the app's data
/// folder, made on first use. Hex, or None when the device cannot keep a key.
pub async fn device_seal(purpose: &str, plain: &[u8]) -> Option<String> {
    #[cfg(target_arch = "wasm32")]
    {
        // The scripts are constants: their `{purpose}` and `{plain}` are filled in here.
        let script = format!("{DEVICE_KEY_JS}\nconst key = await deviceKey(true);\n{SEAL_JS}")
            .replace("{purpose}", &js(purpose))
            .replace("{plain}", &js(&crate::local::to_hex(plain)));
        document::eval(&script).await.ok()?.as_str().map(String::from)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let key = native_device_key(true)?;
        theo_crypto::device::seal(&key, purpose, plain).ok().map(|b| crate::local::to_hex(&b))
    }
}

/// Opens what [`device_seal`] made on this device. None for another device, purpose or key.
pub async fn device_open(purpose: &str, sealed: &str) -> Option<zeroize::Zeroizing<Vec<u8>>> {
    #[cfg(target_arch = "wasm32")]
    {
        let script = format!("{DEVICE_KEY_JS}\nconst key = await deviceKey(false);\nif (!key) return null;\n{OPEN_JS}")
            .replace("{purpose}", &js(purpose))
            .replace("{sealed}", &js(sealed));
        let hex = document::eval(&script).await.ok()?;
        crate::local::from_hex(hex.as_str()?).map(zeroize::Zeroizing::new)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let key = native_device_key(false)?;
        theo_crypto::device::open(&key, purpose, &crate::local::from_hex(sealed)?).ok()
    }
}

/// A JavaScript string literal.
#[cfg(target_arch = "wasm32")]
fn js(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_else(|_| "\"\"".into())
}

/// The device key in IndexedDB (`theo-manager-device`), made when `create` and missing.
#[cfg(target_arch = "wasm32")]
const DEVICE_KEY_JS: &str = r#"
const hex = (b) => [...new Uint8Array(b)].map((x) => x.toString(16).padStart(2, "0")).join("");
const unhex = (h) => new Uint8Array((h.match(/../g) || []).map((x) => parseInt(x, 16)));
async function deviceKey(create) {
    const db = await new Promise((ok, err) => {
        const r = indexedDB.open("theo-manager-device", 1);
        r.onupgradeneeded = () => r.result.createObjectStore("keys");
        r.onsuccess = () => ok(r.result);
        r.onerror = () => err(r.error);
    });
    let key = await new Promise((ok) => {
        const g = db.transaction("keys").objectStore("keys").get("device");
        g.onsuccess = () => ok(g.result);
        g.onerror = () => ok(undefined);
    });
    if (!key && create) {
        key = await crypto.subtle.generateKey({ name: "AES-GCM", length: 256 }, false, ["encrypt", "decrypt"]);
        await new Promise((ok) => {
            const t = db.transaction("keys", "readwrite");
            t.objectStore("keys").put(key, "device");
            t.oncomplete = ok;
            t.onerror = ok;
        });
    }
    return key;
}
"#;

#[cfg(target_arch = "wasm32")]
const SEAL_JS: &str = r#"
const iv = crypto.getRandomValues(new Uint8Array(12));
const ct = await crypto.subtle.encrypt({ name: "AES-GCM", iv, additionalData: new TextEncoder().encode("theo/v1/device:" + {purpose}) }, key, unhex({plain}));
return hex(iv) + hex(ct);
"#;

#[cfg(target_arch = "wasm32")]
const OPEN_JS: &str = r#"
try {
    const all = unhex({sealed});
    const pt = await crypto.subtle.decrypt({ name: "AES-GCM", iv: all.slice(0, 12), additionalData: new TextEncoder().encode("theo/v1/device:" + {purpose}) }, key, all.slice(12));
    return hex(pt);
} catch (e) {
    return null;
}
"#;

/// The apps' device key: `device.key` in the app's data folder (32 random bytes, hex), made on
/// first use when `create`. ⚠️ Any program running as this user can read the file; an OS key
/// store (Windows Hello, Keychain, Android Keystore) would be stronger (to do).
#[cfg(not(target_arch = "wasm32"))]
fn native_device_key(create: bool) -> Option<zeroize::Zeroizing<[u8; 32]>> {
    let path = data_dir().join("device.key");
    if let Some(bytes) = std::fs::read_to_string(&path).ok().and_then(|t| crate::local::from_hex(t.trim())).filter(|b| b.len() == 32) {
        let mut key = zeroize::Zeroizing::new([0u8; 32]);
        key.copy_from_slice(&bytes);
        return Some(key);
    }
    if !create {
        return None;
    }
    let key = theo_crypto::device::new_key().ok()?;
    std::fs::create_dir_all(data_dir()).ok()?;
    std::fs::write(&path, crate::local::to_hex(key.as_ref())).ok()?;
    Some(key)
}

/// A passkey made to unlock a congregation: its id and PRF salt (to keep), and the PRF output
/// (the secret its sealing key comes from; never kept).
pub struct Passkey {
    pub credential: String,
    pub salt: String,
    pub prf: zeroize::Zeroizing<Vec<u8>>,
}

/// Can this platform unlock with a passkey? Only the web for now: the browser's passkeys with
/// the PRF extension. The apps need their OS key stores first (Windows Hello, Keychain, Android).
pub const PASSKEY_UNLOCK: bool = cfg!(target_arch = "wasm32");

/// The PRF part of the WebAuthn scripts: evaluates the salt and gives the output as hex.
#[cfg(target_arch = "wasm32")]
const PRF_JS: &str = r#"
const hex = (b) => [...new Uint8Array(b)].map((x) => x.toString(16).padStart(2, "0")).join("");
const unhex = (h) => new Uint8Array((h.match(/../g) || []).map((x) => parseInt(x, 16)));
const why = (e) => e.name === "NotAllowedError" ? "Cancelled, or the passkey was not accepted."
    : e.name === "SecurityError" ? "Passkeys need the app's own web address (not an IP address)."
    : (e.message || e.name);
async function prfOf(id, salt) {
    const got = await navigator.credentials.get({ publicKey: {
        challenge: crypto.getRandomValues(new Uint8Array(32)),
        allowCredentials: [{ type: "public-key", id: unhex(id) }],
        userVerification: "required",
        timeout: 60000,
        extensions: { prf: { eval: { first: unhex(salt) } } },
    } });
    const r = got.getClientExtensionResults().prf;
    if (!r || !r.results || !r.results.first) return null;
    return hex(r.results.first);
}
"#;

/// Makes a passkey on this device (the browser asks for the fingerprint, face or PIN) and gets
/// its PRF output. Fails when the browser or the authenticator has no PRF: such a passkey could
/// prove who you are, but could not protect the congregation's key.
pub async fn passkey_register(username: &str, display_name: &str) -> Result<Passkey, String> {
    #[cfg(target_arch = "wasm32")]
    {
        let script = format!(
            r#"{PRF_JS}
            if (!window.PublicKeyCredential || !(await PublicKeyCredential.isUserVerifyingPlatformAuthenticatorAvailable())) {{
                return {{ error: "This device has no fingerprint, face or PIN unlock for the browser." }};
            }}
            try {{
                const salt = crypto.getRandomValues(new Uint8Array(32));
                const c = await navigator.credentials.create({{ publicKey: {{
                    challenge: crypto.getRandomValues(new Uint8Array(32)),
                    rp: {{ name: "Theo Manager" }},
                    user: {{ id: crypto.getRandomValues(new Uint8Array(16)), name: {name}, displayName: {display} }},
                    pubKeyCredParams: [{{ type: "public-key", alg: -7 }}, {{ type: "public-key", alg: -257 }}],
                    authenticatorSelection: {{ authenticatorAttachment: "platform", userVerification: "required", residentKey: "preferred" }},
                    attestation: "none",
                    timeout: 60000,
                    extensions: {{ prf: {{ eval: {{ first: salt }} }} }},
                }} }});
                const id = hex(c.rawId);
                const r = c.getClientExtensionResults().prf;
                if (!r || r.enabled === false) return {{ error: "This browser's passkeys cannot protect keys (no PRF support). Keep using the password." }};
                // Some authenticators give the PRF output only when the passkey is used.
                const prf = (r.results && r.results.first) ? hex(r.results.first) : await prfOf(id, hex(salt));
                if (!prf) return {{ error: "This browser's passkeys cannot protect keys (no PRF support). Keep using the password." }};
                return {{ id, salt: hex(salt), prf }};
            }} catch (e) {{
                return {{ error: why(e) }};
            }}
            "#,
            name = js(username),
            display = js(display_name),
        );
        let value = document::eval(&script).await.map_err(|e| e.to_string())?;
        if let Some(e) = value.get("error").and_then(|v| v.as_str()) {
            return Err(e.to_string());
        }
        let field = |k: &str| value.get(k).and_then(|v| v.as_str()).map(String::from).ok_or_else(|| "The browser did not answer.".to_string());
        let prf = crate::local::from_hex(&field("prf")?).ok_or("The browser did not answer.")?;
        Ok(Passkey { credential: field("id")?, salt: field("salt")?, prf: zeroize::Zeroizing::new(prf) })
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (username, display_name);
        Err("Passkey unlock is not in the installed apps yet.".into())
    }
}

/// Uses a passkey made by [`passkey_register`]: the browser asks for the fingerprint, face or PIN,
/// and the authenticator gives the same PRF output again.
pub async fn passkey_prf(credential: &str, salt: &str) -> Result<zeroize::Zeroizing<Vec<u8>>, String> {
    #[cfg(target_arch = "wasm32")]
    {
        let script = format!(
            r#"{PRF_JS}
            try {{
                const prf = await prfOf({id}, {salt});
                return prf ? {{ prf }} : {{ error: "The passkey gave no key (PRF). Use the password." }};
            }} catch (e) {{
                return {{ error: why(e) }};
            }}
            "#,
            id = js(credential),
            salt = js(salt),
        );
        let value = document::eval(&script).await.map_err(|e| e.to_string())?;
        if let Some(e) = value.get("error").and_then(|v| v.as_str()) {
            return Err(e.to_string());
        }
        let prf = value.get("prf").and_then(|v| v.as_str()).and_then(crate::local::from_hex).ok_or("The browser did not answer.")?;
        Ok(zeroize::Zeroizing::new(prf))
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (credential, salt);
        Err("Passkey unlock is not in the installed apps yet.".into())
    }
}
