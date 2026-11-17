//! Enforces the UI consistency rules (REQUIREMENTS.md 8.4, R-412, R-431, R-434 step 1):
//! `theo-app` composes `theo-ui` components only. It never writes classes, inline styles or
//! raw HTML elements, so every screen looks the same.
//!
//! This is a simple line scanner. A real `rsx!` parser (an xtask) can replace it later.

use std::fs;
use std::path::{Path, PathBuf};

/// Raw HTML elements that screens must not use (they must use a theo-ui component instead).
const FORBIDDEN_ELEMENTS: &[&str] = &[
    "div", "span", "p", "a", "button", "input", "select", "option", "textarea", "form", "label", "table",
    "thead", "tbody", "tr", "td", "th", "ul", "ol", "li", "dialog", "img", "svg", "section", "header",
    "footer", "nav", "main", "aside", "h1", "h2", "h3", "h4", "h5", "h6",
];

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            rust_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

fn violations() -> Vec<String> {
    let mut files = Vec::new();
    rust_files(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src"), &mut files);
    let mut found = Vec::new();
    for f in files {
        let Ok(text) = fs::read_to_string(&f) else { continue };
        for (n, raw) in text.lines().enumerate() {
            let line = raw.trim_start();
            if line.starts_with("//") {
                continue;
            }
            let at = format!("{}:{}", f.display(), n + 1);
            if line.contains("class:") {
                found.push(format!("{at}: `class:` is not allowed in theo-app (R-412)"));
            }
            if line.contains("style:") {
                found.push(format!("{at}: `style:` is not allowed in theo-app (R-412)"));
            }
            for el in FORBIDDEN_ELEMENTS {
                let opens_element = line.starts_with(&format!("{el} {{")) || line.contains(&format!("rsx! {{ {el} {{"));
                if opens_element {
                    found.push(format!("{at}: raw `{el}` element; use a theo-ui component (R-431)"));
                }
            }
        }
    }
    found
}

#[test]
fn theo_app_uses_only_theo_ui_components() {
    let v = violations();
    assert!(v.is_empty(), "UI rule violations:\n{}", v.join("\n"));
}
