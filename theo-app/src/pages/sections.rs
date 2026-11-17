use crate::platform::biometric_label;
use crate::prefs::{self, Choice, DateFormat, NameOrder, Overrides, Preferences, TimeFormat, WeekStart};
use crate::state::{use_app_state, StorageMode};
use crate::{nav, route_for, Route};
use dioxus::prelude::*;
use theo_ui::*;
use theo_ui::Action; // explicit: dioxus::prelude also exports an `Action`

/// `/app/:section`: the section's pages as large tiles (the phone entry point for sub-menus).
#[component]
pub fn SectionHub(section: String) -> Element {
    let navigator = navigator();
    let Some(s) = nav::section(&section) else {
        return rsx! { NotFound { segments: vec!["app".to_string(), section] } };
    };
    rsx! {
        Page { title: s.label.clone(), subtitle: s.description.clone(),
            NavHub { section: s, on_navigate: move |id: String| { navigator.push(route_for(&id)); } }
        }
    }
}

/// `/app/:section/:page`. Pages are built in phase 3; until then they show a placeholder.
#[component]
pub fn SectionPage(section: String, page: String) -> Element {
    if section == "congregation" && page == "settings" {
        return rsx! { CongregationSettings {} };
    }
    let Some(p) = nav::page_of(&section, &page) else {
        return rsx! { NotFound { segments: vec!["app".to_string(), section, page] } };
    };
    rsx! {
        Page { title: p.label.clone(), subtitle: p.description.clone(),
            Grid { cols: 3,
                for i in 0..3 {
                    Card { key: "{i}", Skeleton { lines: 3 } }
                }
            }
            EmptyState {
                icon: p.icon,
                title: "{p.label} is coming",
                description: "This page gets its own data-modeling session before it is built (REQUIREMENTS.md phase 3).",
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Preferences (R-062): one select per setting. In the profile, the first option inherits the
// congregation default; in congregation settings, the defaults themselves are edited.
// ---------------------------------------------------------------------------------------------

fn choice_options<T: Choice>(inherit: Option<T>) -> Vec<SelectOption> {
    let mut out = Vec::new();
    if let Some(d) = inherit {
        out.push(SelectOption::new("", format!("Congregation default ({})", d.label())));
    }
    out.extend(T::all().iter().map(|c| SelectOption::new(c.value(), c.label())));
    out
}

fn language_options(inherit: Option<&str>) -> Vec<SelectOption> {
    let mut out = Vec::new();
    if let Some(code) = inherit {
        out.push(SelectOption::new("", format!("Congregation default ({})", prefs::language_label(code))));
    }
    out.extend(prefs::LANGUAGES.iter().map(|(c, l)| SelectOption::new(*c, *l)));
    out
}

/// A one-line example of the effective preferences.
fn preview(p: &Preferences) -> String {
    format!(
        "{} · {} · {} · Weeks start on {}",
        p.format_date(prefs::today()),
        p.format_time(19, 30),
        p.format_name("John", "Smith"),
        p.week_start.label()
    )
}

/// The five preference fields. `on_change(key, value)`; an empty value means "inherit".
#[component]
fn PreferenceFields(
    values: Overrides,
    /// When set, every field offers "Congregation default (…)" as its first option.
    #[props(default)]
    inherit_from: Option<Preferences>,
    on_change: EventHandler<(String, String)>,
) -> Element {
    let opt = |v: Option<&'static str>| v.unwrap_or("").to_string();
    rsx! {
        Grid { cols: 2,
            Field { label: "Language", help: "Translations are coming; texts stay in English for now.",
                Select {
                    value: values.language.clone().unwrap_or_default(),
                    options: language_options(inherit_from.as_ref().map(|p| p.language.as_str())),
                    onchange: move |v| on_change.call(("language".into(), v)),
                }
            }
            Field { label: "Date format",
                Select {
                    value: opt(values.date.map(Choice::value)),
                    options: choice_options::<DateFormat>(inherit_from.as_ref().map(|p| p.date)),
                    onchange: move |v| on_change.call(("date".into(), v)),
                }
            }
            Field { label: "Time format",
                Select {
                    value: opt(values.time.map(Choice::value)),
                    options: choice_options::<TimeFormat>(inherit_from.as_ref().map(|p| p.time)),
                    onchange: move |v| on_change.call(("time".into(), v)),
                }
            }
            Field { label: "First day of the week",
                Select {
                    value: opt(values.week_start.map(Choice::value)),
                    options: choice_options::<WeekStart>(inherit_from.as_ref().map(|p| p.week_start)),
                    onchange: move |v| on_change.call(("week_start".into(), v)),
                }
            }
            Field { label: "Name format",
                Select {
                    value: opt(values.name_order.map(Choice::value)),
                    options: choice_options::<NameOrder>(inherit_from.as_ref().map(|p| p.name_order)),
                    onchange: move |v| on_change.call(("name_order".into(), v)),
                }
            }
        }
    }
}

/// Applies a (key, value) change to overrides. Empty value = inherit.
fn apply_override(o: &mut Overrides, key: &str, v: &str) {
    match key {
        "language" => o.language = (!v.is_empty()).then(|| v.to_string()),
        "date" => o.date = DateFormat::parse(v),
        "time" => o.time = TimeFormat::parse(v),
        "week_start" => o.week_start = WeekStart::parse(v),
        "name_order" => o.name_order = NameOrder::parse(v),
        _ => {}
    }
}

/// Applies a (key, value) change to congregation defaults.
fn apply_default(p: &mut Preferences, key: &str, v: &str) {
    match key {
        "language" if !v.is_empty() => p.language = v.to_string(),
        "date" => p.date = DateFormat::parse(v).unwrap_or(p.date),
        "time" => p.time = TimeFormat::parse(v).unwrap_or(p.time),
        "week_start" => p.week_start = WeekStart::parse(v).unwrap_or(p.week_start),
        "name_order" => p.name_order = NameOrder::parse(v).unwrap_or(p.name_order),
        _ => {}
    }
}

/// Defaults as "all set" overrides, so the same fields can edit them.
fn as_values(p: &Preferences) -> Overrides {
    Overrides {
        date: Some(p.date),
        time: Some(p.time),
        week_start: Some(p.week_start),
        language: Some(p.language.clone()),
        name_order: Some(p.name_order),
    }
}

/// `/app/congregation/settings`: details, meeting times and display defaults.
#[component]
fn CongregationSettings() -> Element {
    let mut state = use_app_state();
    let toaster = use_toaster();
    let Some(c) = state.active_congregation() else { return rsx! {} };
    let p = state.preferences();
    let (mode_title, mode_sub) = match &c.mode {
        StorageMode::Local => ("Local".to_string(), "Stored only on this device".to_string()),
        StorageMode::Cloud { server } => ("Cloud".to_string(), server.clone()),
    };
    rsx! {
        Page { title: "Settings", subtitle: "{c.name} · Congregation settings",
            Grid { cols: 2,
                Card { title: "Details", icon: Icon::Congregation,
                    ItemList {
                        items: vec![
                            ListEntry { icon: Icon::Congregation, title: c.name.clone(), subtitle: if c.number.is_empty() { "No number".into() } else { format!("Number {}", c.number) }, meta: c.language.to_uppercase() },
                            ListEntry { icon: Icon::Clock, title: c.time_zone.clone(), subtitle: "Time zone".into(), meta: String::new() },
                            ListEntry { icon: c.mode.icon(), title: mode_title, subtitle: mode_sub, meta: String::new() },
                        ],
                    }
                }
                Card { title: "Meetings", icon: Icon::Meetings,
                    ItemList {
                        items: vec![
                            ListEntry { icon: Icon::BookOpen, title: "Midweek meeting".into(), subtitle: c.midweek.day.clone(), meta: p.format_hhmm(&c.midweek.time) },
                            ListEntry { icon: Icon::Talk, title: "Weekend meeting".into(), subtitle: c.weekend.day.clone(), meta: p.format_hhmm(&c.weekend.time) },
                        ],
                    }
                }
            }
            Card { title: "Display defaults", icon: Icon::Language,
                Stack {
                    Text { tone: TextTone::Muted, "Everyone in the congregation uses these, unless they choose their own in their profile." }
                    PreferenceFields {
                        values: as_values(&c.defaults),
                        on_change: move |(k, v): (String, String)| {
                            state.update_congregation_defaults(|d| apply_default(d, &k, &v));
                            toaster.success("Congregation defaults saved.");
                        },
                    }
                    Text { tone: TextTone::Small, "Example: {preview(&c.defaults)}" }
                }
            }
        }
    }
}

/// `/app/me`: the user's profile and settings.
#[component]
pub fn Me() -> Element {
    let mut state = use_app_state();
    let navigator = navigator();
    let toaster = use_toaster();
    let account = state.account();
    let congregation = state.active_congregation();
    let name = account.as_ref().map(|a| a.display_name.clone()).unwrap_or_default();
    let username = account.as_ref().map(|a| a.username.clone()).unwrap_or_default();
    let overrides = account.as_ref().map(|a| a.overrides.clone()).unwrap_or_default();
    let defaults = congregation.as_ref().map(|c| c.defaults.clone()).unwrap_or_default();
    let effective = state.preferences();

    rsx! {
        Page { title: name.clone(), subtitle: "@{username}",
            actions: rsx! {
                ActionButton { action: Action::Lock, onclick: move |_| { state.lock(); navigator.replace(Route::Unlock {}); } }
                ActionButton { action: Action::SignOut, onclick: move |_| { state.sign_out(); navigator.replace(Route::Landing {}); } }
            },
            Card { title: "Preferences", icon: Icon::Language,
                Stack {
                    Text { tone: TextTone::Muted, "By default you use your congregation's settings. Choose a value to use your own." }
                    PreferenceFields {
                        values: overrides,
                        inherit_from: defaults,
                        on_change: move |(k, v): (String, String)| state.update_overrides(|o| apply_override(o, &k, &v)),
                    }
                    Text { tone: TextTone::Small, "Example: {preview(&effective)}" }
                }
            }
            Grid { cols: 2,
                Card { title: "Appearance", icon: Icon::Light,
                    Field { label: "Theme",
                        Segmented {
                            value: (state.theme)().as_str(),
                            options: vec![
                                Segment { value: "light".into(), label: "Light".into(), icon: Some(Icon::Light) },
                                Segment { value: "dark".into(), label: "Dark".into(), icon: Some(Icon::Dark) },
                                Segment { value: "system".into(), label: "System".into(), icon: Some(Icon::System) },
                            ],
                            onchange: move |v: String| state.theme.set(Theme::parse(&v)),
                        }
                    }
                }
                Card { title: "Security", icon: Icon::Shield,
                    Stack {
                        if let (Some(label), Some(c)) = (biometric_label(), congregation.clone()) {
                            Switch {
                                checked: c.biometric,
                                onchange: move |v| {
                                    state.set_biometric(&c.id, v);
                                    toaster.success(if v { "Biometric unlock is on for this device." } else { "Biometric unlock is off." });
                                },
                                label: "Unlock {c.name} with {label}",
                                description: "The key stays in this device's secure hardware.",
                            }
                        } else {
                            Text { tone: TextTone::Muted, "Biometric unlock is not available on this device." }
                        }
                        Text { tone: TextTone::Small, "The app locks itself after 5 minutes without activity (R-310)." }
                    }
                }
            }
        }
    }
}

#[component]
pub fn NotFound(segments: Vec<String>) -> Element {
    let navigator = navigator();
    let path = segments.join("/");
    rsx! {
        AuthLayout { title: "Page not found", subtitle: "There is nothing at /{path}.",
            EmptyState {
                icon: Icon::Map,
                title: "This page does not exist",
                action: rsx! { ActionButton { action: Action::Back, label: "Go home", onclick: move |_| { navigator.replace(Route::Landing {}); } } },
            }
        }
    }
}
