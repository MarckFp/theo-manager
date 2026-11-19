use crate::mock;
use crate::state::use_app_state;
use dioxus::prelude::*;
use theo_ui::*;
use theo_ui::Action; // explicit: dioxus::prelude also exports an `Action`

/// `/app` home: widgets that the user reorders and resizes.
#[component]
pub fn Dashboard() -> Element {
    let mut state = use_app_state();
    let mut editing = use_signal(|| false);
    let name = state.account().map(|a| a.display_name).unwrap_or_default();
    let first = name.split_whitespace().next().unwrap_or("").to_string();
    let congregation = state.active_congregation().map(|c| c.name).unwrap_or_default();
    // Widgets that were removed: they can be added back while customizing.
    let shown: Vec<String> = state.layout().into_iter().map(|w| w.id).collect();
    let hidden: Vec<WidgetLayout> = mock::default_layout().into_iter().filter(|w| !shown.contains(&w.id)).collect();

    rsx! {
        Page {
            title: "Welcome back, {first}",
            subtitle: "{congregation} · Some widgets still show sample data",
            actions: rsx! {
                if editing() && !hidden.is_empty() {
                    MenuButton {
                        label: "Add widget",
                        icon: Icon::Plus,
                        items: hidden.iter().map(|w| MenuItem::new(w.id.clone(), w.title.clone(), w.icon)).collect::<Vec<_>>(),
                        onselect: move |id: String| {
                            let mut layout = state.layout();
                            if let Some(w) = mock::default_layout().into_iter().find(|w| w.id == id) {
                                layout.push(w);
                                state.set_layout(layout);
                            }
                        },
                    }
                }
                if editing() {
                    ActionButton { action: Action::ResetLayout, onclick: move |_| state.reset_layout() }
                }
                ActionButton {
                    action: if editing() { Action::Done } else { Action::Customize },
                    variant: if editing() { ButtonVariant::Primary } else { ButtonVariant::Secondary },
                    onclick: move |_| editing.set(!editing()),
                }
            },
            if editing() {
                Alert { tone: Tone::Info, "Drag a widget by its handle to move it, drag the corner to resize it, and remove it with ×. Add widget brings it back." }
            }
            theo_ui::Dashboard {
                widgets: state.layout(),
                editing: editing(),
                on_change: move |layout| state.set_layout(layout),
                render: move |id: String| widget(&id, &state),
            }
        }
    }
}

/// "My assignments": what changed for me (until I mark it seen), then what is coming up.
#[component]
fn MyAssignments() -> Element {
    let mut state = use_app_state();
    let navigator = navigator();
    let prefs = state.preferences();
    let Some(me) = state.me() else {
        return rsx! { Text { tone: TextTone::Muted, "Your assignments appear here when your account is linked to your person in Users." } };
    };
    let dir = state.directory();
    let today = crate::people::today_iso();
    let when = |date: &str, time: &Option<String>| {
        let day = crate::people::parse_date(date).map(|d| prefs.format_date(d)).unwrap_or_else(|| date.to_string());
        match time {
            Some(t) => format!("{day} · {}", prefs.format_hhmm(t)),
            None => day,
        }
    };
    // Notices of what is still to come: new and changed ones mark their row below; cancelled
    // ones (no longer in the list) are shown on top.
    let notices: Vec<&crate::assignments::Notice> = dir.my_notices(me).into_iter().filter(|n| n.date.as_str() >= today.as_str()).collect();
    let badge = |k: crate::assignments::NoticeKind| match k {
        crate::assignments::NoticeKind::New => Tag::new("New", Tone::Info).with_icon(Icon::Sparkles),
        crate::assignments::NoticeKind::Changed => Tag::new("Changed", Tone::Warning).with_icon(Icon::Edit),
        crate::assignments::NoticeKind::Cancelled => Tag::new("Cancelled", Tone::Error).with_icon(Icon::Close),
    };
    let cancelled: Vec<AccentItem> = notices
        .iter()
        .filter(|n| n.kind == crate::assignments::NoticeKind::Cancelled)
        .map(|n| AccentItem { icon: n.what.icon(), accent: n.what.accent(), kind: n.what.label().into(), text: format!("{} · {}", when(&n.date, &n.time), n.detail), badge: Some(badge(n.kind)) })
        .collect();
    let upcoming: Vec<AccentItem> = dir
        .my_assignments(me, &today)
        .into_iter()
        .take(6)
        .map(|a| AccentItem {
            icon: a.kind.icon(),
            accent: a.kind.accent(),
            kind: a.kind.label().into(),
            text: format!("{} · {}", when(&a.date, &a.time), a.detail),
            badge: notices.iter().find(|n| n.what == a.kind && n.record == a.record && n.kind != crate::assignments::NoticeKind::Cancelled).map(|n| badge(n.kind)),
        })
        .collect();
    let changes = match notices.len() {
        0 => String::new(),
        1 => "1 change for you".to_string(),
        n => format!("{n} changes for you"),
    };
    let has_notices = !notices.is_empty();
    rsx! {
        Stack { gap: Gap::Sm,
            if has_notices {
                Cluster { justify: Justify::Between,
                    Badge { tone: Tone::Info, icon: Icon::Sparkles, "{changes}" }
                    ActionButton { action: Action::Done, variant: ButtonVariant::Ghost, label: "Got it", onclick: move |_| state.update_directory(|d| d.mark_notices_seen(me)) }
                }
            }
            if !cancelled.is_empty() {
                AccentList { items: cancelled }
            }
            if upcoming.is_empty() {
                Text { tone: TextTone::Muted, "Nothing coming up for you." }
            } else {
                AccentList { items: upcoming }
            }
            TextLink { icon: Icon::ArrowRight, onclick: move |_| { navigator.push(crate::Route::MyAssignmentsPage {}); }, "All my assignments" }
        }
    }
}

fn widget(id: &str, state: &crate::state::AppState) -> Element {
    let p = state.preferences();
    let meeting = state
        .active_congregation()
        .map(|c| format!("Midweek meeting · {} {}", c.midweek.day, p.format_hhmm(&c.midweek.time)))
        .unwrap_or_default();
    let names = format!(
        "Chairman: {} · Bible reading: {} · Congregation Bible Study: {}",
        p.format_name("John", "Smith"),
        p.format_name("Mark", "Brown"),
        p.format_name("Anna", "Lee")
    );
    match id {
        "next-meeting" => rsx! {
            Stack { gap: Gap::Sm,
                Cluster { justify: Justify::Between,
                    Heading { level: HeadingLevel::H3, "{meeting}" }
                    Badge { tone: Tone::Success, icon: Icon::Check, "All parts assigned" }
                }
                Text { tone: TextTone::Muted, "{names}" }
            }
        },
        "publishers" => {
            let (value, delta) = super::reports::publishers_widget_values(&state.directory());
            rsx! { StatTile { label: "Publishers", value, icon: Icon::Users, delta, trend: Trend::Flat } }
        }
        "reports" => {
            let (label, value, delta, done) = super::reports::reports_widget_values(&state.directory());
            rsx! { StatTile { label, value, icon: Icon::Footprints, delta, trend: if done { Trend::Up } else { Trend::Down } } }
        }
        "assignments" => rsx! { MyAssignments {} },
        "attendance" => rsx! { BarChart { data: mock::attendance(), highlight_last: true } },
        "territories" => rsx! {
            StatTile { label: "Checked out", value: "18 / 42", icon: Icon::Map, delta: "5 overdue", trend: Trend::Down }
        },
        "duties" => rsx! { ItemList { items: mock::duties() } },
        "events" => rsx! { ItemList { items: mock::events() } },
        _ => rsx! { Skeleton {} },
    }
}
