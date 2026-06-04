use dioxus_i18n::prelude::I18nConfig;
use dioxus_i18n::unic_langid::{LanguageIdentifier, langid};

const EN_US_FTL: &str = include_str!("../assets/i18n/en-US.ftl");
const ES_ES_FTL: &str = include_str!("../assets/i18n/es-ES.ftl");

/// Build the i18n configuration, picking the initial locale from the
/// system / browser settings and falling back to English (en-US).
///
/// Language-only aliases (`en`, `es`) are registered alongside the full
/// regional locales so that e.g. `es-MX` or `en-GB` correctly resolve to
/// Spanish or English instead of falling back to English.
pub fn init_config() -> I18nConfig {
    let detected = detect_locale();
    I18nConfig::new(detected)
        .with_fallback(langid!("en-US"))
        // English — regional (en-US)
        .with_locale((langid!("en-US"), EN_US_FTL))
        // English — language-only alias so en-GB, en-AU, etc. all resolve
        .with_locale((langid!("en"), EN_US_FTL))
        // Spanish — regional (es-ES)
        .with_locale((langid!("es-ES"), ES_ES_FTL))
        // Spanish — language-only alias so es-MX, es-419, etc. all resolve
        .with_locale((langid!("es"), ES_ES_FTL))
}

/// Detect the user's preferred language from the runtime environment.
pub fn detect_locale() -> LanguageIdentifier {
    let raw = raw_locale();
    // Strip encoding suffixes like ".UTF-8" (POSIX locales)
    let trimmed = raw.split('.').next().unwrap_or("en-US");
    // Normalise POSIX underscore separator to BCP 47 hyphen (es_ES → es-ES)
    let normalised = trimmed.replace('_', "-");
    LanguageIdentifier::from_bytes(normalised.as_bytes()).unwrap_or_else(|_| langid!("en-US"))
}

/// Read the raw locale string from the environment.
#[cfg(target_arch = "wasm32")]
fn raw_locale() -> String {
    web_sys::window()
        .and_then(|w| w.navigator().language())
        .unwrap_or_else(|| "en-US".to_string())
}

/// Read the raw locale string from the OS.
#[cfg(not(target_arch = "wasm32"))]
fn raw_locale() -> String {
    sys_locale::get_locale().unwrap_or_else(|| "en-US".to_string())
}
