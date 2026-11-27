//! The workbook reader lives in `theo-core` (parsing only, no network). This module adds the
//! download, which needs the platform's network access.

pub use theo_core::workbook::*;

/// Downloads and reads the workbook of `issue` in `lang`. Desktop and phone: directly, in the
/// background. Web: browsers may not read the EPUB from jw.org (CORS, and there is no relay
/// server on purpose); then the error carries the address so the user can download it and pick
/// the file (R-035).
pub async fn download(issue: &str, lang: &str) -> Result<Workbook, DownloadError> {
    let fail = |message: String, epub_url: Option<String>| DownloadError { message, epub_url };
    let links = crate::platform::fetch_bytes(&links_url(issue, lang)).await.map_err(|e| fail(format!("jw.org could not be reached: {e}"), None))?;
    let url = epub_url_from_links(&String::from_utf8_lossy(&links), lang).ok_or_else(|| fail("This workbook is not available yet in this language.".into(), None))?;
    let bytes = crate::platform::fetch_bytes(&url).await.map_err(|e| {
        let why = if cfg!(target_arch = "wasm32") { "The browser cannot download the workbook by itself.".to_string() } else { format!("The workbook could not be downloaded: {e}") };
        fail(why, Some(url.clone()))
    })?;
    read_epub(&bytes, issue, lang).map_err(|e| fail(e, None))
}
