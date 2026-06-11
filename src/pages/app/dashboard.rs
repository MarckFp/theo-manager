use dioxus::prelude::*;
use dioxus_i18n::t;

use crate::database::{use_crypto, use_db};
use crate::models::event::{CongregationEvent, EventType};
use crate::models::field_service_report::FieldServiceReport;
use crate::models::user::{Appointment, User, UserType};
use crate::pages::app::events::{event_display_title, event_type_label};

// ── Platform helpers ──────────────────────────────────────────────────────────

#[cfg(target_arch = "wasm32")]
fn current_year_month() -> (i32, u8) {
    let d = js_sys::Date::new_0();
    (d.get_full_year() as i32, (d.get_month() + 1) as u8)
}

#[cfg(not(target_arch = "wasm32"))]
fn current_year_month() -> (i32, u8) {
    (2026, 6)
}

/// Hex color for each user type — used in inline styles so Tailwind scanning
/// doesn't need to see the class names.
fn type_color(ut: &UserType) -> &'static str {
    match ut {
        UserType::Publisher => "#60a5fa",               // blue-400
        UserType::BaptizedPublisher => "#818cf8",       // indigo-400
        UserType::ContinuousAuxiliaryPioneer => "#a78bfa", // violet-400
        UserType::RegularPioneer => "#a855f7",          // purple-500
        UserType::SpecialPioneer => "#d946ef",          // fuchsia-500
        UserType::Missionary => "#f43f5e",              // rose-500
        UserType::Student => "#9ca3af",
    }
}

#[component]
pub fn AppDashboard() -> Element {
    let db_signal = use_db();
    let crypto_signal = use_crypto();

    let users_res = use_resource(move || async move {
        let Some(db) = db_signal.read().db.clone() else { return vec![] };
        let crypto = crypto_signal.read().clone();
        User::all(&db, &crypto).await.unwrap_or_default()
    });

    let (cy, cm) = current_year_month();
    let reports_res = use_resource(move || async move {
        let Some(db) = db_signal.read().db.clone() else { return vec![] };
        let crypto = crypto_signal.read().clone();
        FieldServiceReport::by_month(&db, &crypto, cy, cm)
            .await
            .unwrap_or_default()
    });

    let events_res = use_resource(move || async move {
        let Some(db) = db_signal.read().db.clone() else { return vec![] };
        CongregationEvent::upcoming(&db, 60).await.unwrap_or_default()
    });

    let is_loading = users_res.read().is_none() || reports_res.read().is_none() || events_res.read().is_none();

    let users: Vec<User> = users_res().unwrap_or_default();
    let reports: Vec<FieldServiceReport> = reports_res().unwrap_or_default();
    let upcoming: Vec<CongregationEvent> = events_res().unwrap_or_default();

    // ── Derived stats ─────────────────────────────────────────────────────────
    let publishers: Vec<&User> = users
        .iter()
        .filter(|u| !matches!(u.user_type, UserType::Student))
        .collect();
    let publisher_count = publishers.len();
    let elder_count = publishers
        .iter()
        .filter(|u| matches!(u.appointment, Some(Appointment::Elder)))
        .count();
    let ms_count = publishers
        .iter()
        .filter(|u| matches!(u.appointment, Some(Appointment::MinisterialServant)))
        .count();

    // Bar chart: count per non-student type (skip types with 0)
    let type_entries: Vec<(UserType, usize)> = [
        UserType::Publisher,
        UserType::BaptizedPublisher,
        UserType::ContinuousAuxiliaryPioneer,
        UserType::RegularPioneer,
        UserType::SpecialPioneer,
        UserType::Missionary,
    ]
    .into_iter()
    .map(|ut| {
        let count = publishers.iter().filter(|u| u.user_type == ut).count();
        (ut, count)
    })
    .filter(|(_, c)| *c > 0)
    .collect();
    let max_count = type_entries.iter().map(|(_, c)| *c).max().unwrap_or(1);

    // Donut: submitted reports vs total eligible publishers
    let submitted_count = reports.len();
    let eligible = publisher_count.max(1);
    let submitted_pct = (submitted_count * 100 / eligible).min(100);
    // SVG donut math (r = 38)
    let circ: f64 = 2.0 * std::f64::consts::PI * 38.0; // ≈ 238.76
    let filled: f64 = circ * submitted_count as f64 / eligible as f64;
    let filled_str = format!("{:.2} {:.2}", filled.min(circ), (circ - filled).max(0.0));

    rsx! {
        div { class: "space-y-6 w-full pb-10",
            h1 { class: "text-2xl font-bold text-gray-900", {t!("page-dashboard")} }

            if is_loading {
                div { class: "flex items-center justify-center py-20 text-gray-400",
                    span { class: "text-sm animate-pulse", {t!("dash-loading")} }
                }
            } else {
                // ── Row 1: Big publisher stat + elder/MS counts ───────────────
                div { class: "grid grid-cols-2 sm:grid-cols-4 gap-4",
                    // Total publishers (spans 2 columns)
                    div { class: "col-span-2 bg-white rounded-xl border border-gray-200 p-5 flex flex-col gap-1",
                        p { class: "text-sm font-medium text-gray-500",
                            {t!("dash-stat-publishers")}
                        }
                        p { class: "text-6xl font-extrabold text-primary-600 leading-none",
                            "{publisher_count}"
                        }
                        p { class: "text-xs text-gray-400 mt-1", {t!("dash-stat-publishers-desc")} }
                    }
                    // Elders
                    div { class: "bg-white rounded-xl border border-gray-200 p-5 flex flex-col gap-1",
                        p { class: "text-sm font-medium text-gray-500", {t!("dash-stat-elders")} }
                        p { class: "text-4xl font-extrabold text-amber-600 leading-none",
                            "{elder_count}"
                        }
                    }
                    // Ministerial servants
                    div { class: "bg-white rounded-xl border border-gray-200 p-5 flex flex-col gap-1",
                        p { class: "text-sm font-medium text-gray-500", {t!("dash-stat-ms")} }
                        p { class: "text-4xl font-extrabold text-emerald-600 leading-none",
                            "{ms_count}"
                        }
                    }
                }

                // ── Row 2: Bar chart + Donut ──────────────────────────────────
                div { class: "grid grid-cols-1 sm:grid-cols-2 gap-4",

                    // ── Bar chart: members by type ────────────────────────────
                    div { class: "bg-white rounded-xl border border-gray-200 p-5",
                        p { class: "text-sm font-semibold text-gray-700 mb-4",
                            {t!("dash-stat-user-types-title")}
                        }
                        if type_entries.is_empty() {
                            p { class: "text-sm text-gray-400 text-center py-6",
                                {t!("dash-loading")}
                            }
                        } else {
                            div { class: "space-y-3",
                                for (ut , count) in type_entries.iter() {
                                    {
                                        let label = match ut {
                                            UserType::Publisher => t!("user-type-publisher"),
                                            UserType::BaptizedPublisher => t!("user-type-baptized"),
                                            UserType::ContinuousAuxiliaryPioneer => t!("user-type-cont-aux-pioneer"),
                                            UserType::RegularPioneer => t!("user-type-regular-pioneer"),
                                            UserType::SpecialPioneer => t!("user-type-special-pioneer"),
                                            UserType::Missionary => t!("user-type-missionary"),
                                            UserType::Student => t!("user-type-student"),
                                        };
                                        let color = type_color(ut);
                                        let pct = (*count * 100 / max_count).min(100);
                                        let bar_style =
                                            format!("width: {}%; background-color: {};", pct, color);
                                        let count_val = *count;
                                        rsx! {
                                            div { class: "flex items-center gap-2",
                                                span { class: "text-xs text-gray-600 w-28 shrink-0 truncate", "{label}" }
                                                div { class: "flex-1 bg-gray-100 rounded-full h-2.5 overflow-hidden",
                                                    div {
                                                        class: "h-2.5 rounded-full transition-all duration-500",
                                                        style: "{bar_style}",
                                                    }
                                                }
                                                span { class: "text-xs font-semibold text-gray-700 w-5 text-right shrink-0", "{count_val}" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // ── Donut: field service reports submitted ────────────────
                    div { class: "bg-white rounded-xl border border-gray-200 p-5 flex flex-col items-center",
                        p { class: "text-sm font-semibold text-gray-700 mb-4 self-start",
                            {t!("dash-stat-report-title")}
                        }
                        // SVG donut
                        div { class: "relative w-44 h-44",
                            svg {
                                view_box: "0 0 100 100",
                                class: "w-full h-full -rotate-90",
                                // Background ring
                                circle {
                                    cx: "50",
                                    cy: "50",
                                    r: "38",
                                    fill: "none",
                                    stroke: "#e5e7eb",
                                    stroke_width: "12",
                                }
                                // Filled arc
                                circle {
                                    cx: "50",
                                    cy: "50",
                                    r: "38",
                                    fill: "none",
                                    stroke: "var(--color-primary-500)",
                                    stroke_width: "12",
                                    stroke_linecap: "round",
                                    stroke_dasharray: "{filled_str}",
                                }
                            }
                            // Centre text
                            div { class: "absolute inset-0 flex flex-col items-center justify-center",
                                span { class: "text-3xl font-extrabold text-gray-900 leading-none",
                                    "{submitted_pct}%"
                                }
                                span { class: "text-xs text-gray-400 mt-1",
                                    "{submitted_count}/{publisher_count}"
                                }
                            }
                        }
                        // Legend
                        div { class: "flex gap-5 mt-4 text-xs",
                            div { class: "flex items-center gap-1.5",
                                span { class: "inline-block w-2.5 h-2.5 rounded-full bg-primary-500" }
                                span { class: "text-gray-600", {t!("dash-stat-report-submitted")} }
                            }
                            div { class: "flex items-center gap-1.5",
                                span { class: "inline-block w-2.5 h-2.5 rounded-full bg-gray-200" }
                                span { class: "text-gray-600", {t!("dash-stat-report-pending")} }
                            }
                        }
                    }
                }
                // ── Row 3: Upcoming events ─────────────────────────────────────────
                div { class: "bg-white rounded-xl border border-gray-200 p-5",
                    p { class: "text-sm font-semibold text-gray-700 mb-3",
                        {t!("dash-upcoming-events")}
                    }
                    if upcoming.is_empty() {
                        p { class: "text-sm text-gray-400 text-center py-4",
                            {t!("dash-no-upcoming-events")}
                        }
                    } else {
                        div { class: "divide-y divide-gray-100",
                            for event in upcoming.iter().take(5) {
                                {
                                    let badge_cls = match &event.event_type {
                                        EventType::CircuitAssembly => "bg-blue-100 text-blue-700",
                                        EventType::Memorial => "bg-purple-100 text-purple-700",
                                        EventType::CircuitOverseerVisit => "bg-green-100 text-green-700",
                                        EventType::RegionalConvention => "bg-orange-100 text-orange-700",
                                        EventType::Other => "bg-gray-100 text-gray-600",
                                    };
                                    let type_label = event_type_label(&event.event_type);
                                    let title = event_display_title(event);
                                    let date_range = if event.start_date == event.end_date {
                                        event.start_date.clone()
                                    } else {
                                        format!("{} – {}", event.start_date, event.end_date)
                                    };
                                    rsx! {
                                        div { class: "flex items-center gap-3 py-2.5",
                                            span { class: "inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium shrink-0 {badge_cls}",
                                                "{type_label}"
                                            }
                                            div { class: "flex-1 min-w-0",
                                                p { class: "text-sm font-medium text-gray-800 truncate", "{title}" }
                                                p { class: "text-xs text-gray-400 tabular-nums", "{date_range}" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
