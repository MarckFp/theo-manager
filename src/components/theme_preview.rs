use dioxus::prelude::*;

// Accent palette: (50, 100, 500, 600, 700)
fn accent_colors(accent: &str) -> (&'static str, &'static str, &'static str, &'static str, &'static str) {
    match accent {
        "Green"  => ("#f0fdf4", "#dcfce7", "#22c55e", "#16a34a", "#15803d"),
        "Purple" => ("#faf5ff", "#f3e8ff", "#a855f7", "#9333ea", "#7e22ce"),
        "Rose"   => ("#fff1f2", "#ffe4e6", "#f43f5e", "#e11d48", "#be123c"),
        "Amber"  => ("#fffbeb", "#fef3c7", "#f59e0b", "#d97706", "#b45309"),
        _        => ("#eff6ff", "#dbeafe", "#3b82f6", "#2563eb", "#1d4ed8"), // Blue
    }
}

/// A small mock-up preview showing how a theme + accent combination looks.
///
/// Props are raw strings (`"dark"/"light"` and `"Blue"/"Green"` etc.)
/// so this works from any context without importing the enum types.
#[component]
pub fn ThemePreview(theme: String, accent: String) -> Element {
    let dark = theme == "dark";

    // Surface colors
    let bg_page    = if dark { "#111827" } else { "#f9fafb" };
    let bg_card    = if dark { "#1f2937" } else { "#ffffff" };
    let bg_sidebar = if dark { "#1f2937" } else { "#ffffff" };
    let border     = if dark { "#374151" } else { "#e5e7eb" };
    let text_main  = if dark { "#f3f4f6" } else { "#111827" };
    let text_muted = if dark { "#9ca3af" } else { "#6b7280" };
    let text_light = if dark { "#6b7280" } else { "#9ca3af" };

    let (c50, c100, c500, c600, _c700) = accent_colors(&accent);

    // Active sidebar item
    let nav_active_bg   = c100;
    let nav_active_text = c600;

    rsx! {
        div {
            style: format!(
                "background:{bg_page}; border:1px solid {border}; border-radius:12px; \
                         overflow:hidden; display:flex; height:140px; font-family:sans-serif; \
                         font-size:12px; user-select:none;",
            ),

            // ── Sidebar ────────────────────────────────────────────────────
            div {
                style: format!(
                    "background:{bg_sidebar}; border-right:1px solid {border}; \
                                 width:110px; display:flex; flex-direction:column; padding:8px 6px; gap:4px; flex-shrink:0;",
                ),
                // Congregation header
                div {
                    style: format!(
                        "background:{c500}; border-radius:6px; padding:5px 7px; \
                                         display:flex; align-items:center; gap:5px;",
                    ),
                    div { style: "background:rgba(255,255,255,0.25); width:18px; height:18px; \
                                border-radius:4px; flex-shrink:0;" }
                    div { style: "color:#fff; font-weight:600; font-size:10px; overflow:hidden; white-space:nowrap; text-overflow:ellipsis;",
                        "Congregation"
                    }
                }
                // Nav item — active
                div {
                    style: format!(
                        "background:{nav_active_bg}; color:{nav_active_text}; \
                                         border-radius:6px; padding:4px 6px; font-weight:600; font-size:10px; \
                                         display:flex; align-items:center; gap:4px;",
                    ),
                    span { "🏠" }
                    span { "Dashboard" }
                }
                // Nav item — inactive
                div {
                    style: format!(
                        "color:{text_muted}; border-radius:6px; padding:4px 6px; \
                                         font-size:10px; display:flex; align-items:center; gap:4px;",
                    ),
                    span { "👥" }
                    span { "Publishers" }
                }
                // Nav item — inactive
                div {
                    style: format!(
                        "color:{text_muted}; border-radius:6px; padding:4px 6px; \
                                         font-size:10px; display:flex; align-items:center; gap:4px;",
                    ),
                    span { "⚙️" }
                    span { "Settings" }
                }
            }

            // ── Main area ──────────────────────────────────────────────────
            div { style: "flex:1; display:flex; flex-direction:column; padding:10px; gap:8px; overflow:hidden;",

                // Top title bar
                div { style: format!("color:{text_main}; font-weight:700; font-size:13px;"),
                    "Dashboard"
                }

                // Card row
                div { style: "display:flex; gap:6px; flex:1;",

                    // Card 1
                    div {
                        style: format!(
                            "background:{bg_card}; border:1px solid {border}; border-radius:8px; \
                                                 padding:8px; flex:1; display:flex; flex-direction:column; gap:4px; overflow:hidden;",
                        ),
                        div { style: format!("color:{text_muted}; font-size:10px;"),
                            "Publishers"
                        }
                        div { style: format!("color:{text_main}; font-weight:700; font-size:16px;"),
                            "142"
                        }
                        div {
                            style: format!(
                                "background:{c50}; color:{nav_active_text}; border-radius:4px; \
                                                                    padding:2px 5px; font-size:9px; font-weight:600; width:fit-content;",
                            ),
                            "Active"
                        }
                    }

                    // Card 2
                    div {
                        style: format!(
                            "background:{bg_card}; border:1px solid {border}; border-radius:8px; \
                                                 padding:8px; flex:1; display:flex; flex-direction:column; gap:4px; overflow:hidden;",
                        ),
                        div { style: format!("color:{text_muted}; font-size:10px;"),
                            "Groups"
                        }
                        div { style: format!("color:{text_main}; font-weight:700; font-size:16px;"),
                            "8"
                        }
                        // Mini progress bar
                        div { style: format!("background:{border}; border-radius:3px; height:4px; margin-top:auto;"),
                            div { style: format!("background:{c500}; border-radius:3px; height:4px; width:60%;") }
                        }
                    }
                }

                // Button row
                div { style: "display:flex; gap:5px;",
                    div {
                        style: format!(
                            "background:{c600}; color:#fff; border-radius:6px; \
                                                 padding:4px 10px; font-size:10px; font-weight:600;",
                        ),
                        "Save"
                    }
                    div {
                        style: format!(
                            "background:transparent; color:{text_muted}; border:1px solid {border}; \
                                                 border-radius:6px; padding:4px 10px; font-size:10px;",
                        ),
                        "Cancel"
                    }
                    div { style: format!("color:{text_light}; font-size:9px; align-self:center; margin-left:auto;"),
                        if dark {
                            "Dark mode"
                        } else {
                            "Light mode"
                        }
                    }
                }
            }
        }
    }
}
