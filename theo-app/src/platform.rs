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
