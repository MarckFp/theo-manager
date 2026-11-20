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

/// A small value remembered by this device (the browser's storage on the web and the installed
/// web app). Native apps remember nothing yet. Never congregation data: only settings of the
/// device, which congregation is open and until when, and which notices were seen.
pub fn device_get(key: &str) -> Option<String> {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window().and_then(|w| w.local_storage().ok().flatten()).and_then(|s| s.get_item(key).ok().flatten())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = key;
        None
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
    let _ = (key, value);
}

/// Seconds since 1970.
pub fn now_secs() -> u64 {
    web_time::SystemTime::now().duration_since(web_time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}
