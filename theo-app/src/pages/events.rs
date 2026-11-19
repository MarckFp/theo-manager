//! *Meetings › Events*: conventions, assemblies, the Memorial, the circuit overseer's visit and
//! custom events, as a list or a calendar. Rules: `docs/model/m12-events.md`.

use crate::absences::When;
use crate::events::{length_in_days, Event, EventDraft, EventKind};
use crate::people::{self, parse_date, Person};
use crate::prefs::{Preferences, WeekStart};
use crate::state::use_app_state;
use dioxus::prelude::*;
use std::collections::BTreeSet;
use theo_ui::*;
use theo_ui::Action; // explicit: dioxus::prelude also exports an `Action`

const F_WHEN: &str = "when";
const F_KIND: &str = "kind";
const F_WHO: &str = "who";

pub(super) fn kind_icon(k: EventKind) -> Icon {
    match k {
        EventKind::RegionalConvention => Icon::Building,
        EventKind::CircuitAssembly => Icon::Group,
        EventKind::Memorial => Icon::Sparkles,
        EventKind::CircuitOverseerVisit => Icon::Briefcase,
        EventKind::Custom => Icon::Calendar,
    }
}

fn kind_tone(k: EventKind) -> Tone {
    match k {
        EventKind::RegionalConvention | EventKind::CircuitAssembly => Tone::Info,
        EventKind::Memorial => Tone::Success,
        EventKind::CircuitOverseerVisit => Tone::Warning,
        EventKind::Custom => Tone::Neutral,
    }
}

/// "21 Nov 2026" or "21 Nov 2026 – 23 Nov 2026", in the user's date format.
pub(super) fn period(prefs: &Preferences, from: &str, to: &str) -> String {
    let show = |iso: &str| parse_date(iso).map(|d| prefs.format_date(d)).unwrap_or_else(|| iso.to_string());
    if from == to { show(from) } else { format!("{} – {}", show(from), show(to)) }
}

/// The list starts with events that are now or coming up.
fn default_filters() -> FilterSelection {
    let mut s = FilterSelection::new();
    s.insert(F_WHEN.into(), ["now".to_string(), "upcoming".to_string()].into_iter().collect::<BTreeSet<_>>());
    s
}

fn values(e: &Event, today: &str, facet: &str) -> Vec<String> {
    match facet {
        F_WHEN => vec![e.when(today).value().into()],
        F_KIND => vec![e.kind.value().into()],
        F_WHO => vec![if e.is_private() { "people" } else { "everyone" }.into()],
        _ => Vec::new(),
    }
}

/// `/app/meetings/events`
#[component]
pub fn EventsPage() -> Element {
    let state = use_app_state();
    let congregation = state.active_congregation().map(|c| c.name).unwrap_or_default();
    let view = (state.events_view)();
    let mut editing = use_signal(|| None::<Option<u32>>);
    rsx! {
        Page {
            title: "Events",
            subtitle: "{congregation} · Conventions, assemblies, the Memorial, the circuit overseer's visit and more",
            actions: rsx! {
                ActionButton { action: Action::Add, label: "Add event", onclick: move |_| editing.set(Some(None)) }
            },
            if view == "calendar" {
                StickyBar { ViewTabs {} }
                CalendarView { on_open: move |id| editing.set(Some(Some(id))) }
            } else {
                ListView { on_open: move |id| editing.set(Some(Some(id))) }
            }
        }
        if let Some(id) = editing() {
            EventDialog { key: "{id:?}", id, onclose: move |_| editing.set(None) }
        }
    }
}

#[component]
fn ViewTabs() -> Element {
    let mut state = use_app_state();
    rsx! {
        Segmented {
            value: (state.events_view)(),
            options: vec![
                Segment { value: "list".into(), label: "List".into(), icon: Some(Icon::Report) },
                Segment { value: "calendar".into(), label: "Calendar".into(), icon: Some(Icon::Calendar) },
            ],
            onchange: move |v: String| state.events_view.set(v),
        }
    }
}

#[component]
fn ListView(on_open: EventHandler<u32>) -> Element {
    let state = use_app_state();
    let prefs = state.preferences();
    let dir = state.directory();
    let (viewer, admin) = (state.me(), state.i_am_admin());
    let mut query = use_signal(String::new);
    let mut sel = use_signal(default_filters);
    let today = people::today_iso();
    let pool = dir.visible_events(viewer, admin);
    let tokens: Vec<String> = people::fold(&query()).split_whitespace().map(str::to_string).collect();
    let matches = |e: &Event, skip: Option<&str>| {
        let hay = people::fold(&format!("{} {}", e.title, e.description.clone().unwrap_or_default()));
        tokens.iter().all(|t| hay.contains(t.as_str()))
            && sel().iter().all(|(f, chosen)| Some(f.as_str()) == skip || chosen.is_empty() || values(e, &today, f).iter().any(|v| chosen.contains(v)))
    };
    let v = |a: &str, b: &str| (a.to_string(), b.to_string());
    let defs: Vec<people::FacetDef> = vec![
        (F_WHEN, "When", Icon::Calendar, vec![v("now", "Now"), v("upcoming", "Coming up"), v("past", "Past")]),
        (F_KIND, "Type", Icon::Tag, EventKind::ALL.iter().map(|k| v(k.value(), k.label())).collect()),
        (F_WHO, "For", Icon::Users, vec![v("everyone", "Whole congregation"), v("people", "Chosen people")]),
    ];
    let facets: Vec<Facet> = defs
        .into_iter()
        .map(|(id, label, icon, opts)| Facet {
            id: id.into(),
            label: label.into(),
            icon,
            options: opts
                .into_iter()
                .map(|(value, label)| {
                    let count = pool.iter().filter(|e| matches(e, Some(id)) && values(e, &today, id).contains(&value)).count();
                    FacetOption { value, label, count }
                })
                .collect(),
        })
        .collect();
    let shown: Vec<&&Event> = pool.iter().filter(|e| matches(e, None)).collect();
    let total = pool.len();
    rsx! {
        StickyBar {
            ViewTabs {}
            FilterBar {
                query: query(),
                on_query: move |q| query.set(q),
                search_placeholder: "Search events",
                facets,
                selected: sel(),
                on_select: move |s| sel.set(s),
                summary: format!("{} of {total} events", shown.len()),
                defaults: default_filters(),
            }
        }
        if shown.is_empty() {
            EmptyState { icon: Icon::Calendar, title: if total == 0 { "No events yet" } else { "No events match" }, description: "Add an event, or change the filters." }
        } else {
            Grid { cols: 3, gap: Gap::Sm,
                for e in shown.iter() {
                    {
                        let id = e.id;
                        let days = length_in_days(e);
                        let mut tags = vec![Tag::new(e.kind.label(), kind_tone(e.kind)).with_icon(kind_icon(e.kind))];
                        match e.when(&today) {
                            When::Now => tags.push(Tag::new("Now", Tone::Success).with_icon(Icon::Clock)),
                            When::Past => tags.push(Tag::new("Past", Tone::Neutral)),
                            When::Upcoming => {}
                        }
                        if days > 1 {
                            tags.push(Tag::new(format!("{days} days"), Tone::Neutral));
                        }
                        if let Some(p) = &e.people {
                            tags.push(Tag::new(if p.len() == 1 { "1 person".to_string() } else { format!("{} people", p.len()) }, Tone::Neutral).with_icon(Icon::Lock));
                        }
                        if e.absence {
                            tags.push(Tag::new("Away from meetings", Tone::Neutral).with_icon(Icon::Absence));
                        }
                        rsx! {
                            RecordCard {
                                key: "{id}",
                                title: e.title.clone(),
                                avatar: e.title.clone(),
                                subtitle: period(&prefs, &e.from, &e.to),
                                tags,
                                onclick: move |_| on_open.call(id),
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn CalendarView(on_open: EventHandler<u32>) -> Element {
    let mut state = use_app_state();
    let prefs = state.preferences();
    let dir = state.directory();
    let (viewer, admin) = (state.me(), state.i_am_admin());
    let (y, m, d) = crate::prefs::today();
    let month = (state.events_month)().unwrap_or((y, m));
    let events = dir.visible_events(viewer, admin);
    let entries: Vec<CalendarEntry> = events
        .iter()
        .filter_map(|e| {
            Some(CalendarEntry {
                id: e.id.to_string(),
                title: e.title.clone(),
                subtitle: period(&prefs, &e.from, &e.to),
                from: parse_date(&e.from)?,
                to: parse_date(&e.to)?,
                tone: kind_tone(e.kind),
                icon: kind_icon(e.kind),
                detail: None,
            })
        })
        .collect();
    // Two years each way, and as far as the events go.
    let first = entries.iter().map(|e| (e.from.0, e.from.1)).min().unwrap_or((y, m)).min((y - 2, m));
    let last = entries.iter().map(|e| (e.to.0, e.to.1)).max().unwrap_or((y, m)).max((y + 2, m));
    rsx! {
        Calendar {
            month,
            entries,
            today: (y, m, d),
            min: first,
            max: last,
            week_starts_sunday: prefs.week_start == WeekStart::Sunday,
            empty_text: "No events in this month.",
            on_month: move |v| state.events_month.set(Some(v)),
            on_select: move |id: String| {
                if let Ok(id) = id.parse::<u32>() {
                    on_open.call(id);
                }
            },
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Add or change an event
// ---------------------------------------------------------------------------------------------

#[component]
fn EventDialog(id: Option<u32>, onclose: EventHandler<()>) -> Element {
    let mut state = use_app_state();
    let toaster = use_toaster();
    let prefs = state.preferences();
    let dir = state.directory();
    let existing = id.and_then(|i| dir.event(i).cloned());
    let mut draft = use_signal(|| match &existing {
        Some(e) => EventDraft::from_event(e),
        None => EventDraft::new(EventKind::Custom, &people::today_iso()),
    });
    let mut errors = use_signal(Vec::<(&'static str, String)>::new);
    let mut query = use_signal(String::new);
    let mut confirm = use_signal(|| false);
    let err = move |f: &str| errors.read().iter().find(|(k, _)| *k == f).map(|(_, m)| m.clone());
    let d = draft();

    let kinds: Vec<SelectOption> = EventKind::ALL.iter().map(|k| SelectOption::new(k.value(), k.label())).collect();
    let tokens: Vec<String> = people::fold(&query()).split_whitespace().map(str::to_string).collect();
    let chosen: BTreeSet<String> = d.people.iter().map(|p| p.to_string()).collect();
    let mut current: Vec<&Person> = dir
        .people
        .iter()
        .filter(|p| p.moved_away.is_none())
        .filter(|p| {
            let hay = people::fold(&format!("{} {}", p.first_name, p.last_name));
            tokens.iter().all(|t| hay.contains(t.as_str()))
        })
        .collect();
    current.sort_by_cached_key(|p| (!chosen.contains(&p.id.to_string()), people::fold(&p.display_name(&prefs))));
    let rows: Vec<Row> = current.iter().map(|p| Row::new(p.id.to_string(), p.display_name(&prefs)).with_subtitle(p.publisher_type.label())).collect();
    let absence_help = if d.everyone {
        "The whole congregation is away from its own meetings on these days (for example at a convention or an assembly). It shows in Users › Absences."
    } else {
        "The chosen people are away from the meetings on these days. It shows in Users › Absences."
    };

    let save = move |_| {
        let d = draft();
        match state.update_directory(|dir| dir.save_event(id, &d)) {
            Ok(_) => {
                toaster.success(if id.is_some() { "Event saved." } else { "Event added." });
                onclose.call(());
            }
            Err(e) => errors.set(e),
        }
    };

    rsx! {
        Dialog {
            open: true,
            onclose: move |_| onclose.call(()),
            title: if existing.is_some() { "Event" } else { "Add event" },
            description: "Choose a type to fill in the usual title and days, then change what you need.",
            actions: rsx! {
                EditActions {
                    on_save: save,
                    on_cancel: move |_| onclose.call(()),
                    on_delete: existing.is_some().then_some(EventHandler::new(move |_| confirm.set(true))),
                }
            },
            Stack {
                Field { label: "Type",
                    Select { value: d.kind.value(), options: kinds, onchange: move |v: String| { let k = EventKind::parse(&v); draft.set(draft().with_kind(k)); } }
                }
                Field { label: "Title", required: true, error: err("title"),
                    TextInput { value: d.title.clone(), placeholder: "For example: Kingdom Hall cleaning day", oninput: move |v| draft.write().title = v }
                }
                Grid { cols: 2,
                    Field { label: "First day", required: true, error: err("from"),
                        TextInput { kind: InputKind::Date, value: d.from.clone(), oninput: move |v| draft.set(draft().with_from(v)) }
                    }
                    Field { label: "Last day", required: true, error: err("to"),
                        TextInput { kind: InputKind::Date, value: d.to.clone(), oninput: move |v| draft.write().to = v }
                    }
                }
                Field { label: "Description",
                    TextArea { value: d.description.clone(), rows: 3, oninput: move |v| draft.write().description = v }
                }
                Switch {
                    checked: d.absence,
                    label: if d.everyone { "Create an absence for the whole congregation" } else { "Create an absence for the chosen people" },
                    description: absence_help,
                    onchange: move |on| draft.write().absence = on,
                }
                Field { label: "For", error: err("people"),
                    Segmented {
                        value: if d.everyone { "everyone" } else { "people" },
                        options: vec![
                            Segment { value: "everyone".into(), label: "Whole congregation".into(), icon: Some(Icon::Congregation) },
                            Segment { value: "people".into(), label: "Chosen people".into(), icon: Some(Icon::Lock) },
                        ],
                        onchange: move |v: String| draft.write().everyone = v == "everyone",
                    }
                }
                if !d.everyone {
                    Text { tone: TextTone::Small, "Only these people and the administrators see this event. {d.people.len()} chosen." }
                    TextInput { kind: InputKind::Search, value: query(), icon: Icon::Search, placeholder: "Search by name", oninput: move |v| query.set(v) }
                    PickList {
                        rows,
                        selected: chosen,
                        on_toggle: move |pid: String| {
                            if let Ok(p) = pid.parse::<u32>() {
                                let mut w = draft.write();
                                match w.people.iter().position(|x| *x == p) {
                                    Some(i) => {
                                        w.people.remove(i);
                                    }
                                    None => w.people.push(p),
                                }
                            }
                        },
                    }
                }
            }
        }
        ConfirmDialog {
            open: confirm(),
            title: "Delete this event?",
            message: if existing.as_ref().is_some_and(|e| e.absence) { "The event and the absences it created are removed." } else { "The event is removed." },
            confirm: Action::Delete,
            oncancel: move |_| confirm.set(false),
            onconfirm: move |_| {
                if let Some(i) = id {
                    state.update_directory(|d| d.delete_event(i));
                }
                toaster.success("Event deleted.");
                onclose.call(());
            },
        }
    }
}
