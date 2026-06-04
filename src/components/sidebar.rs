use dioxus::prelude::*;
use dioxus_primitives::{
    dropdown_menu::{DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger},
    separator::Separator,
};

use crate::{Route, database::use_db};
use dioxus_i18n::t;

// ── Context ───────────────────────────────────────────────────────────────────

/// Wraps the sidebar-open signal so it can be distinguished from other
/// `Signal<bool>` values that might also live in the context tree.
#[derive(Clone, Copy)]
pub struct SidebarCtx(pub Signal<bool>);

/// Read the sidebar open/close signal from any descendant component.
pub fn use_sidebar() -> Signal<bool> {
    use_context::<SidebarCtx>().0
}

// ── AppSidebar ────────────────────────────────────────────────────────────────

/// Full-height sidebar with congregation switcher, sectioned navigation and a
/// user footer.
///
/// Responsive behaviour:
/// - **Desktop (md+):** always visible as a static flex column.
/// - **Mobile:** `position: fixed`, slides in from the left when the
///   `SidebarCtx` signal is `true`. The [`AppLayout`] renders a backdrop that
///   sets the signal to `false` when tapped.
#[component]
pub fn AppSidebar() -> Element {
    let open = use_sidebar();

    // On mobile the sidebar is a fixed overlay that translates in/out.
    // On md+ it becomes a normal static flex child via md:relative / md:translate-x-0.
    let aside_class = if *open.read() {
        "fixed inset-y-0 left-0 z-30 flex flex-col w-64 h-full bg-white border-r border-gray-200 transition-transform duration-300 translate-x-0 md:relative md:z-auto"
    } else {
        "fixed inset-y-0 left-0 z-30 flex flex-col w-64 h-full bg-white border-r border-gray-200 transition-transform duration-300 -translate-x-full md:relative md:z-auto md:translate-x-0"
    };

    rsx! {
        aside { class: aside_class,
            CongregationSwitcher {}

            // ── Primary navigation ─────────────────────────────────────────
            nav { class: "flex-1 overflow-y-auto px-2 py-3",
                NavItem {
                    to: Route::AppDashboard {},
                    icon: "🏠",
                    label: t!("nav-dashboard"),
                }

                NavDivider {}

                // User section
                NavSectionLabel { label: t!("section-users") }
                NavItem {
                    to: Route::AppUsers {},
                    icon: "👥",
                    label: t!("nav-user-list"),
                }
                NavItem {
                    to: Route::AppFieldServiceReports {},
                    icon: "📊",
                    label: t!("nav-field-service-reports"),
                }
                NavItem {
                    to: Route::AppAbsences {},
                    icon: "📅",
                    label: t!("nav-absences"),
                }

                NavDivider {}

                // Ministry section
                NavSectionLabel { label: t!("section-ministry") }
                NavItem {
                    to: Route::AppPublicPreaching {},
                    icon: "📢",
                    label: t!("nav-public-preaching"),
                }
                NavItem {
                    to: Route::AppFieldServiceGroups {},
                    icon: "💼",
                    label: t!("nav-field-service-groups"),
                }
                NavItem {
                    to: Route::AppTerritory {},
                    icon: "🗺️",
                    label: t!("nav-territory"),
                }
                NavItem {
                    to: Route::AppFieldServiceMeetings {},
                    icon: "🤝",
                    label: t!("nav-field-service-meetings"),
                }

                NavDivider {}

                // Meetings section
                NavSectionLabel { label: t!("section-meetings") }
                NavItem {
                    to: Route::AppAttendants {},
                    icon: "🚪",
                    label: t!("nav-attendants"),
                }
                NavItem {
                    to: Route::AppAvPlatform {},
                    icon: "🎧",
                    label: t!("nav-av-platform"),
                }
                NavItem {
                    to: Route::AppCleaning {},
                    icon: "🧹",
                    label: t!("nav-cleaning"),
                }
                NavItem {
                    to: Route::AppWeekdayMeeting {},
                    icon: "📖",
                    label: t!("nav-weekday-meeting"),
                }
                NavItem {
                    to: Route::AppWeekendMeeting {},
                    icon: "🗣️",
                    label: t!("nav-weekend-meeting"),
                }

                NavDivider {}

                // Congregation section
                NavSectionLabel { label: t!("section-congregation") }
                NavItem {
                    to: Route::AppCongregationSettings {},
                    icon: "⚙️",
                    label: t!("nav-settings"),
                }
                NavItem {
                    to: Route::AppCongregationPermissions {
                    },
                    icon: "🔒",
                    label: t!("nav-permissions"),
                }
            }

            // ── User / disconnect footer ───────────────────────────────────
            UserMenu {}
        }
    }
}

// ── Congregation switcher ─────────────────────────────────────────────────────

/// Top-of-sidebar dropdown that shows the active congregation and allows
/// switching or adding a new one.
///
/// Congregation data will be loaded from the DB in a future iteration;
/// for now it uses a placeholder signal.
#[component]
fn CongregationSwitcher() -> Element {
    let congregation = use_signal(|| "My Congregation".to_string());

    let initial = congregation
        .read()
        .chars()
        .next()
        .map(|c| c.to_ascii_uppercase().to_string())
        .unwrap_or_else(|| "?".to_string());

    rsx! {
        div { class: "p-2 border-b border-gray-200",
            DropdownMenu { class: "relative",
                DropdownMenuTrigger { class: "w-full flex items-center gap-3 px-3 py-2 rounded-lg hover:bg-gray-100 active:bg-gray-200 transition-colors text-left",
                    // Avatar / initial
                    div { class: "flex items-center justify-center w-9 h-9 rounded-lg bg-blue-600 text-white text-sm font-bold shrink-0",
                        "{initial}"
                    }
                    // Name + role
                    div { class: "flex-1 min-w-0",
                        p { class: "text-sm font-semibold text-gray-900 truncate",
                            "{congregation}"
                        }
                        p { class: "text-xs text-gray-500", {t!("congregation-label")} }
                    }
                    span { class: "text-gray-400 text-xs shrink-0", "⌄" }
                }
                DropdownMenuContent { class: "absolute left-0 top-full mt-1 w-full bg-white border border-gray-200 rounded-xl shadow-lg py-1 z-50",
                    div { class: "px-3 py-2 text-xs font-medium text-gray-400 uppercase tracking-wider border-b border-gray-100",
                        {t!("switch-congregation")}
                    }
                    DropdownMenuItem::<String> {
                        index: 0usize,
                        value: "new".to_string(),
                        // TODO: navigate to congregation creation
                        on_select: move |_: String| {},
                        div { class: "flex items-center gap-3 px-3 py-2 text-sm text-blue-600 font-medium",
                            span { "＋" }
                            span { {t!("new-congregation")} }
                        }
                    }
                }
            }
        }
    }
}

// ── User / account menu ────────────────────────────────────────────────────────

/// Footer dropdown with account settings and disconnect.
#[component]
fn UserMenu() -> Element {
    let mut db = use_db();
    let nav = use_navigator();

    rsx! {
        div { class: "p-2 border-t border-gray-200",
            DropdownMenu { class: "relative",
                DropdownMenuTrigger { class: "w-full flex items-center gap-3 px-3 py-2 rounded-lg hover:bg-gray-100 transition-colors text-left",
                    div { class: "flex items-center justify-center w-9 h-9 rounded-full bg-gray-200 text-gray-600 text-sm font-medium shrink-0",
                        "U"
                    }
                    div { class: "flex-1 min-w-0",
                        p { class: "text-sm font-medium text-gray-900 truncate", {t!("user-label")} }
                        p { class: "text-xs text-gray-500 truncate", {t!("account-settings")} }
                    }
                    span { class: "text-gray-400 text-xs shrink-0", "⌄" }
                }
                // Opens upward so it doesn't overflow the viewport bottom
                DropdownMenuContent { class: "absolute left-0 bottom-full mb-1 w-full bg-white border border-gray-200 rounded-xl shadow-lg py-1 z-50",
                    DropdownMenuItem::<String> {
                        index: 0usize,
                        value: "settings".to_string(),
                        // TODO: navigate to settings page
                        on_select: move |_: String| {},
                        div { class: "flex items-center gap-3 px-3 py-2 text-sm text-gray-700",
                            span { "⚙️" }
                            span { {t!("menu-settings")} }
                        }
                    }
                    DropdownMenuItem::<String> {
                        index: 1usize,
                        value: "disconnect".to_string(),
                        on_select: move |_: String| {
                            db.write().db = None;
                            nav.push(Route::Landing {});
                        },
                        div { class: "flex items-center gap-3 px-3 py-2 text-sm text-red-600",
                            span { "⎋" }
                            span { {t!("menu-disconnect")} }
                        }
                    }
                }
            }
        }
    }
}

// ── Nav primitives ────────────────────────────────────────────────────────────

/// Section heading inside the nav — uppercase, muted, small.
#[component]
fn NavSectionLabel(label: String) -> Element {
    rsx! {
        p { class: "px-3 pt-4 pb-1.5 text-xs font-semibold text-gray-400 uppercase tracking-wider select-none",
            "{label}"
        }
    }
}

/// Thin horizontal rule between nav groups.
#[component]
fn NavDivider() -> Element {
    rsx! {
        Separator {
            class: "h-px bg-gray-100 my-2 mx-1",
            horizontal: true,
            decorative: true,
        }
    }
}

/// Single navigation link. Highlights when the current route matches `to`.
/// On mobile, clicking any link also closes the sidebar via [`use_sidebar`].
#[component]
fn NavItem(to: Route, icon: String, label: String) -> Element {
    let current = use_route::<Route>();
    let mut sidebar_open = use_sidebar();

    let is_active = current == to;

    let class = if is_active {
        "flex items-center gap-3 px-3 py-2 rounded-lg bg-blue-50 text-blue-700 font-semibold text-sm w-full"
    } else {
        "flex items-center gap-3 px-3 py-2 rounded-lg text-gray-600 hover:bg-gray-100 hover:text-gray-900 transition-colors text-sm w-full"
    };

    rsx! {
        Link {
            to,
            class,
            // Close the mobile sidebar when navigating
            onclick: move |_| sidebar_open.set(false),
            span { class: "w-5 text-center text-base shrink-0", "{icon}" }
            span { class: "flex-1 truncate", "{label}" }
        }
    }
}
