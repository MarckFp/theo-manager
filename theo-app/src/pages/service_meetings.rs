//! *Ministry › Meetings for field service*: the meetings of a month (list or calendar), the
//! weekly presets with who can conduct them, filling and assigning a month, and sending it.
//! Planners (write permission) see everything; everyone else sees only what was sent.
//! Rules: `docs/model/m15-field-service-meetings.md`.

use crate::people::{self, parse_date, Directory, Person};
use crate::prefs::{Preferences, WeekStart};
use crate::reports::{month_label, Ym};
use crate::service_meetings::{weekday_of, Availability, MeetingVersion, SendStatus, ServiceMeetingDraft, WEEKDAYS};
use crate::state::use_app_state;
use dioxus::prelude::*;
use std::collections::BTreeSet;
use theo_ui::*;
use theo_ui::Action; // explicit: dioxus::prelude also exports an `Action`

const PAGE: &str = "ministry/service-meetings";

fn this_month() -> Ym {
    let (y, m, _) = crate::prefs::today();
    (y, m)
}

/// "Monday 2 November".
fn day_title(iso: &str) -> String {
    match (parse_date(iso), weekday_of(iso)) {
        (Some((_, m, d)), Some(w)) => format!("{} {d} {}", WEEKDAYS[w as usize], theo_ui::month_name(m)),
        _ => iso.to_string(),
    }
}

fn name_of(dir: &Directory, prefs: &Preferences, id: Option<u32>) -> Option<String> {
    id.and_then(|p| dir.person(p)).map(|p| p.display_name(prefs))
}

/// The tag that says a planner's change is not sent yet (nothing for sent meetings).
fn status_tag(s: SendStatus) -> Option<Tag> {
    match s {
        SendStatus::Sent => None,
        SendStatus::NotSent => Some(Tag::new("Not sent", Tone::Warning).with_icon(Icon::Clock)),
        SendStatus::Changed => Some(Tag::new("Changed · not sent", Tone::Warning).with_icon(Icon::Edit)),
        SendStatus::Removed => Some(Tag::new("Removed · not sent", Tone::Error).with_icon(Icon::Trash)),
    }
}

fn availability_tag(a: Availability) -> Option<Tag> {
    match a {
        Availability::Available => Some(Tag::new(a.label(), Tone::Success).with_icon(Icon::Check)),
        Availability::NotForThisSlot => Some(Tag::new(a.label(), Tone::Warning).with_icon(Icon::Clock)),
        Availability::BusyThatDay => Some(Tag::new(a.label(), Tone::Warning).with_icon(Icon::Calendar)),
        Availability::Away => Some(Tag::new(a.label(), Tone::Error).with_icon(Icon::Absence)),
        Availability::NoPrivilege => None,
    }
}

/// Weekday numbers in the user's week order.
fn week_order(prefs: &Preferences) -> Vec<u8> {
    let first = if prefs.week_start == WeekStart::Sunday { 0 } else { 1 };
    (0..7).map(|i| (first + i) % 7).collect()
}

/// `/app/ministry/service-meetings`
#[component]
pub fn ServiceMeetingsPage() -> Element {
    let mut state = use_app_state();
    let toaster = use_toaster();
    let congregation = state.active_congregation().map(|c| c.name).unwrap_or_default();
    let planner = state.can_write(PAGE);
    let dir = state.directory();
    let mut view = (state.service_view)();
    // Presets have their own page now; an old "presets" choice falls back to the list.
    if view == "presets" || (!planner && view == "statistics") {
        view = "list".into();
    }
    let navigator = navigator();
    let month = (state.service_month)().unwrap_or_else(this_month);
    let mut editing = use_signal(|| None::<Option<u32>>);
    let mut confirm_send = use_signal(|| false);
    let mut confirm_clear = use_signal(|| false);
    let unsent = dir.unsent_service_changes(month);
    let count = |s: SendStatus| unsent.iter().filter(|(_, x)| *x == s).count();
    let (new, changed, removed) = (count(SendStatus::NotSent), count(SendStatus::Changed), count(SendStatus::Removed));
    let has_presets = !dir.service_presets.is_empty();

    let on_fill = move |id: String| {
        if (id == "fill" || id == "fill-assign") && state.directory().service_presets.is_empty() {
            toaster.error("Add the weekly meetings first (Presets).");
            navigator.push(crate::Route::ServicePresetsPage {});
            return;
        }
        match id.as_str() {
            "fill" | "fill-assign" => {
                let r = state.update_directory(|d| d.fill_service_month(month, id == "fill-assign"));
                let mut text = match r.added {
                    0 => "The month already has every meeting of the presets.".to_string(),
                    1 => "1 meeting added.".to_string(),
                    n => format!("{n} meetings added."),
                };
                if id == "fill-assign" {
                    text.push_str(&format!(" {} with a conductor.", r.assigned));
                }
                if !r.skipped_days.is_empty() {
                    text.push_str(&format!(" {} days skipped: the congregation is away.", r.skipped_days.len()));
                }
                if r.unassigned > 0 {
                    toaster.show(Tone::Warning, format!("{text} {} without a conductor: nobody is available.", r.unassigned));
                } else {
                    toaster.success(text);
                }
            }
            "assign" => {
                let (a, left) = state.update_directory(|d| d.assign_service_meetings(month));
                if left > 0 {
                    toaster.show(Tone::Warning, format!("{a} conductors assigned. {left} meetings without a conductor: nobody is available."));
                } else if a == 0 {
                    toaster.success("Every meeting already has a conductor.");
                } else {
                    toaster.success(format!("{a} conductors assigned."));
                }
            }
            "clear" => confirm_clear.set(true),
            _ => {}
        }
    };

    let on_export = {
        let dir = dir.clone();
        let prefs = state.preferences();
        let congregation = congregation.clone();
        move |id: String| {
            let Some(format) = crate::export::Format::parse(&id) else { return };
            let versions: Vec<MeetingVersion> = if planner {
                dir.service_meetings_in(month).iter().filter(|m| !m.removed).map(|m| m.current()).collect()
            } else {
                dir.sent_service_meetings_in(month).into_iter().map(|(_, v)| v).collect()
            };
            if versions.is_empty() {
                toaster.error("There are no meetings to export in this month.");
                return;
            }
            let rows = versions
                .iter()
                .map(|v| vec![day_title(&v.date), v.time.clone(), v.place.clone(), name_of(&dir, &prefs, v.conductor).unwrap_or_default()])
                .collect();
            let columns = ["Date", "Time", "Place", "Conductor"];
            let table = crate::export::Table {
                title: format!("{congregation} · Meetings for field service"),
                subtitle: month_label(month),
                columns: columns.iter().map(|c| c.to_string()).collect(),
                widths: vec![2.2, 1.0, 3.0, 2.2],
                rows,
            };
            super::users::save_export(&table, format, &congregation, &format!("field-service-meetings-{:04}-{:02}", month.0, month.1), toaster);
        }
    };
    let export = rsx! { super::users::ExportMenu { csv_description: "The meetings of the month, for spreadsheets", onselect: on_export } };
    let presets_button = rsx! {
        ActionButton { action: Action::Settings, variant: ButtonVariant::Secondary, label: "Presets", onclick: move |_| { navigator.push(crate::Route::ServicePresetsPage {}); } }
    };
    let actions: Element = if planner {
        if view == "statistics" {
            presets_button
        } else {
            rsx! {
                {presets_button}
                {export}
                MenuButton {
                    label: "Fill month",
                    icon: Icon::Magic,
                    items: vec![
                        MenuItem::new("fill", "Fill", Icon::Calendar).with_description("Every preset on its days, without conductors"),
                        MenuItem::new("fill-assign", "Fill and assign", Icon::Magic).with_description("Every preset, and a conductor for each meeting"),
                        MenuItem::new("assign", "Assign the empty ones", Icon::User).with_description("A conductor for meetings that have none"),
                        MenuItem::new("clear", "Clear the month", Icon::Trash).with_description("Delete every meeting of the month"),
                    ],
                    onselect: on_fill,
                }
                ActionButton {
                    action: Action::Send,
                    variant: ButtonVariant::Secondary,
                    label: if unsent.is_empty() { "Send".to_string() } else { format!("Send ({})", unsent.len()) },
                    disabled: unsent.is_empty(),
                    onclick: move |_| confirm_send.set(true),
                }
                ActionButton { action: Action::Add, label: "Add meeting", onclick: move |_| editing.set(Some(None)) }
            }
        }
    } else {
        export
    };

    rsx! {
        Page {
            title: "Meetings for field service",
            subtitle: "{congregation} · Where and when to meet for the ministry, and who conducts",
            actions: actions,
            StickyBar {
                Segmented {
                    value: view.clone(),
                    options: {
                        let mut o = vec![
                            Segment { value: "list".into(), label: "List".into(), icon: Some(Icon::Report) },
                            Segment { value: "calendar".into(), label: "Calendar".into(), icon: Some(Icon::Calendar) },
                        ];
                        if planner {
                            o.push(Segment { value: "statistics".into(), label: "Statistics".into(), icon: Some(Icon::Chart) });
                        }
                        o
                    },
                    onchange: move |v: String| state.service_view.set(v),
                }
                if view == "list" {
                    MonthPicker {
                        value: month,
                        today: this_month(),
                        min: crate::reports::shift(this_month(), -24),
                        max: crate::reports::shift(this_month(), 12),
                        full_width: true,
                        onchange: move |m| state.service_month.set(Some(m)),
                    }
                }
            }
            if view == "statistics" {
                StatisticsTab {}
            } else {
                if planner {
                    PlannerSummary { month, has_presets }
                } else {
                    Text { tone: TextTone::Muted, "The meetings for field service of {month_label(month)}. Changes appear here when they are sent." }
                }
                if view == "calendar" {
                    MeetingsCalendar { month, planner, on_open: move |id| editing.set(Some(Some(id))) }
                } else {
                    MeetingsList { month, planner, on_open: move |id| editing.set(Some(Some(id))) }
                }
            }
        }
        if let Some(id) = editing() {
            MeetingDialog { key: "{id:?}", id, month, onclose: move |_| editing.set(None) }
        }
        ConfirmDialog {
            open: confirm_send(),
            title: format!("Send {}?", month_label(month)),
            message: {
                let mut parts = Vec::new();
                if new > 0 { parts.push(format!("{new} new")); }
                if changed > 0 { parts.push(format!("{changed} changed")); }
                if removed > 0 { parts.push(format!("{removed} removed")); }
                format!("{}. Everyone who can see the meetings for field service will see them as they are now.", parts.join(", "))
            },
            confirm: Action::Send,
            oncancel: move |_| confirm_send.set(false),
            onconfirm: move |_| {
                let n = state.update_directory(|d| d.send_service_month(month));
                confirm_send.set(false);
                toaster.success(if n == 1 { "1 change sent.".to_string() } else { format!("{n} changes sent.") });
            },
        }
        ConfirmDialog {
            open: confirm_clear(),
            title: format!("Clear {}?", month_label(month)),
            message: "Every meeting of the month is deleted. Meetings that were sent stay visible to everyone until you send again.",
            confirm: Action::Delete,
            oncancel: move |_| confirm_clear.set(false),
            onconfirm: move |_| {
                state.update_directory(|d| d.clear_service_month(month));
                confirm_clear.set(false);
                toaster.success("The month is cleared.");
            },
        }
    }
}

/// For planners: what is not sent yet, and meetings without a conductor. One short alert.
#[component]
fn PlannerSummary(month: Ym, has_presets: bool) -> Element {
    let state = use_app_state();
    let dir = state.directory();
    let unsent = dir.unsent_service_changes(month).len();
    let empty = dir.service_meetings_in(month).iter().filter(|m| !m.removed && m.conductor.is_none()).count();
    let total = dir.service_meetings_in(month).iter().filter(|m| !m.removed).count();
    if total == 0 && unsent == 0 {
        return rsx! {
            Alert { tone: Tone::Info, title: "Nothing planned for {month_label(month)}",
                if has_presets { "Use Fill month to add the weekly meetings, or Add meeting for a single one." } else { "Add the weekly meetings in the Presets tab, then use Fill month." }
            }
        };
    }
    let mut lines = Vec::new();
    if unsent > 0 {
        lines.push(format!(
            "{unsent} {} not sent yet: only planners see {}. Send when the month is ready.",
            if unsent == 1 { "change is" } else { "changes are" },
            if unsent == 1 { "it" } else { "them" }
        ));
    }
    if empty > 0 {
        lines.push(format!("{empty} {} no conductor.", if empty == 1 { "meeting has" } else { "meetings have" }));
    }
    if lines.is_empty() {
        return rsx! {
            Text { tone: TextTone::Small, "Everything is sent: everyone sees these {total} meetings." }
        };
    }
    rsx! {
        Alert { tone: Tone::Warning, title: "{month_label(month)}",
            Stack { gap: Gap::Xs,
                for (i, l) in lines.iter().enumerate() {
                    Text { key: "{i}", "{l}" }
                }
            }
        }
    }
}

#[component]
fn MeetingsList(month: Ym, planner: bool, on_open: EventHandler<u32>) -> Element {
    let state = use_app_state();
    let prefs = state.preferences();
    let dir = state.directory();
    // (id, version shown, status for planners)
    let shown: Vec<(u32, MeetingVersion, Option<SendStatus>)> = if planner {
        dir.service_meetings_in(month).iter().map(|m| (m.id, m.current(), Some(m.status()))).collect()
    } else {
        dir.sent_service_meetings_in(month).into_iter().map(|(id, v)| (id, v, None)).collect()
    };
    if shown.is_empty() {
        return rsx! {
            EmptyState { icon: Icon::Footprints, title: "No meetings for field service this month", description: if planner { "Fill the month from the presets, or add a meeting." } else { "They appear here when they are sent." } }
        };
    }
    rsx! {
        Grid { cols: 3, gap: Gap::Sm,
            for (id, v, status) in shown {
                {
                    let conductor = name_of(&dir, &prefs, v.conductor);
                    let mut tags = vec![match &conductor {
                        Some(n) => Tag::new(n.clone(), Tone::Neutral).with_icon(Icon::User),
                        None => Tag::new("No conductor", Tone::Error).with_icon(Icon::Warning),
                    }];
                    if let Some(t) = status.and_then(status_tag) {
                        tags.push(t);
                    }
                    rsx! {
                        RecordCard {
                            key: "{id}",
                            title: format!("{} · {}", day_title(&v.date), v.time),
                            avatar: conductor.clone().unwrap_or_else(|| "?".into()),
                            subtitle: v.place.clone(),
                            tags,
                            onclick: move |_| if planner { on_open.call(id) },
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn MeetingsCalendar(month: Ym, planner: bool, on_open: EventHandler<u32>) -> Element {
    let mut state = use_app_state();
    let prefs = state.preferences();
    let dir = state.directory();
    let (y, m, d) = crate::prefs::today();
    let versions: Vec<(u32, MeetingVersion, Option<SendStatus>)> = if planner {
        dir.service_meetings.iter().filter(|x| !x.removed).map(|x| (x.id, x.current(), Some(x.status()))).collect()
    } else {
        dir.service_meetings.iter().filter_map(|x| x.sent.clone().map(|s| (x.id, s, None))).collect()
    };
    let entries: Vec<CalendarEntry> = versions
        .into_iter()
        .filter_map(|(id, v, status)| {
            let date = parse_date(&v.date)?;
            let conductor = name_of(&dir, &prefs, v.conductor);
            let tone = match (&conductor, status) {
                (None, _) => Tone::Error,
                (_, Some(SendStatus::NotSent | SendStatus::Changed)) => Tone::Warning,
                _ => Tone::Info,
            };
            Some(CalendarEntry {
                id: id.to_string(),
                title: format!("{} {}", v.time, conductor.clone().unwrap_or_else(|| "No conductor".into())),
                subtitle: format!("{} · {}", day_title(&v.date), v.place),
                from: date,
                to: date,
                tone,
                icon: Icon::Footprints,
                detail: Some(v.place.clone()),
            })
        })
        .collect();
    rsx! {
        if planner {
            Text { tone: TextTone::Small, "Blue: sent. Yellow: not sent yet. Red: no conductor." }
        }
        Calendar {
            month,
            entries,
            today: (y, m, d),
            min: crate::reports::shift(this_month(), -24),
            max: crate::reports::shift(this_month(), 12),
            week_starts_sunday: prefs.week_start == WeekStart::Sunday,
            empty_text: "No meetings for field service in this month.",
            on_month: move |v| state.service_month.set(Some(v)),
            on_select: move |id: String| {
                if planner && let Ok(id) = id.parse::<u32>() {
                    on_open.call(id);
                }
            },
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Presets
// ---------------------------------------------------------------------------------------------

/// `/app/ministry/service-meetings/presets`: the weekly meetings and who can conduct them.
#[component]
pub fn ServicePresetsPage() -> Element {
    let state = use_app_state();
    let navigator = navigator();
    let congregation = state.active_congregation().map(|c| c.name).unwrap_or_default();
    let mut editing = use_signal(|| None::<Option<u32>>);
    let back = ("Meetings for field service".to_string(), EventHandler::new(move |_| { navigator.push(crate::Route::ServiceMeetingsPage {}); }));
    if !state.can_write(PAGE) {
        return rsx! {
            Page { title: "Presets", back,
                EmptyState { icon: Icon::Lock, title: "Only planners can change the presets", description: "Ask an administrator for write access to Meetings for field service." }
            }
        };
    }
    rsx! {
        Page {
            title: "Presets",
            subtitle: "{congregation} · Meetings for field service: the weekly meetings and who can conduct them",
            back,
            actions: rsx! { ActionButton { action: Action::Add, label: "Add preset", onclick: move |_| editing.set(Some(None)) } },
            PresetsView { on_open: move |id| editing.set(Some(Some(id))) }
        }
        if let Some(id) = editing() {
            PresetDialog { key: "{id:?}", id, onclose: move |_| editing.set(None) }
        }
    }
}

/// Statistics of the last 6 or 12 months (sent meetings).
#[component]
fn StatisticsTab() -> Element {
    let state = use_app_state();
    let prefs = state.preferences();
    let dir = state.directory();
    let mut period = use_signal(|| "6".to_string());
    let months: i64 = if period() == "12" { 12 } else { 6 };
    let today = people::today_iso();
    let from = crate::events::add_days(&today, -months * 30).unwrap_or_else(|| today.clone());
    let (meetings, with_conductor) = dir.service_meeting_totals(&from, &today);
    let per = dir.meetings_per_conductor(&from, &today);
    let conductors = per.len();
    let conducted = per.iter().filter(|(_, n)| *n > 0).count();
    let none: Vec<String> = per.iter().filter(|(_, n)| *n == 0).filter_map(|(p, _)| dir.person(*p)).map(|p| p.display_name(&prefs)).collect();
    let top = per.first().map(|(_, n)| *n).unwrap_or(0).max(1);
    let pct = (with_conductor * 100).checked_div(meetings).unwrap_or(0);
    let first = week_order(&prefs)[0];
    let mut presets: Vec<(String, String, usize)> = dir
        .service_presets_sorted(first)
        .iter()
        .map(|p| (p.label(), p.place.clone(), dir.conductors().iter().filter(|c| p.available.contains(&c.id)).count()))
        .collect();
    presets.sort_by_key(|p| p.2);
    let busiest = per.first().filter(|(_, n)| *n > 0).and_then(|(p, n)| dir.person(*p).map(|x| (x.display_name(&prefs), *n)));
    rsx! {
        Segmented {
            value: period(),
            options: vec![
                Segment { value: "6".into(), label: "Last 6 months".into(), icon: Some(Icon::Calendar) },
                Segment { value: "12".into(), label: "Last 12 months".into(), icon: Some(Icon::History) },
            ],
            onchange: move |v: String| period.set(v),
        }
        Grid { cols: 4, gap: Gap::Sm, compact: true,
            Card { StatTile { label: "Meetings", value: meetings.to_string(), icon: Icon::Footprints, delta: "sent meetings".to_string() } }
            Card { StatTile { label: "With a conductor", value: format!("{pct}%"), icon: Icon::Check, delta: format!("{with_conductor} of {meetings}") } }
            Card { StatTile { label: "Conducted", value: format!("{conducted} of {conductors}"), icon: Icon::Users, delta: "with the privilege".to_string() } }
            Card { StatTile { label: "Meetings per conductor", value: if conductors > 0 { format!("{:.1}", with_conductor as f64 / conductors as f64) } else { "–".into() }, icon: Icon::User, delta: "on average".to_string() } }
        }
        Grid { cols: 2,
            Card { title: "Meetings per conductor", icon: Icon::Users,
                if per.is_empty() {
                    Text { tone: TextTone::Muted, "Nobody has the privilege yet." }
                } else {
                    Stack { gap: Gap::Sm,
                        for (p, n) in per.iter() {
                            ProgressRow {
                                key: "{p}",
                                label: dir.person(*p).map(|x| x.display_name(&prefs)).unwrap_or_default(),
                                detail: if *n == 1 { "1 meeting".to_string() } else { format!("{n} meetings") },
                                value: *n as f64,
                                max: top as f64,
                                tone: if *n == 0 { Tone::Warning } else { Tone::Neutral },
                            }
                        }
                    }
                }
            }
            Stack {
                Card { title: "Availability per preset", icon: Icon::Settings,
                    if presets.is_empty() {
                        Text { tone: TextTone::Muted, "No presets yet." }
                    } else {
                        Stack { gap: Gap::Sm,
                            Text { tone: TextTone::Small, "Fewest available first: these are the hardest to fill." }
                            for (i, (label, place, can)) in presets.iter().enumerate() {
                                ProgressRow {
                                    key: "{i}",
                                    label: label.clone(),
                                    detail: format!("{can} of {conductors} available"),
                                    value: *can as f64,
                                    max: conductors.max(1) as f64,
                                    note: place.clone(),
                                    tone: if *can == 0 { Tone::Error } else if *can < 3 { Tone::Warning } else { Tone::Success },
                                }
                            }
                        }
                    }
                }
                Card { title: "Highlights", icon: Icon::Sparkles,
                    ItemList {
                        items: {
                            let mut items = Vec::new();
                            if let (Some(low), Some(high)) = (presets.first(), presets.last()) {
                                items.push(ListEntry { icon: Icon::Warning, title: format!("Fewest available: {}", low.0), subtitle: format!("{} · {} of {conductors}", low.1, low.2), meta: String::new() });
                                if presets.len() > 1 {
                                    items.push(ListEntry { icon: Icon::Check, title: format!("Most available: {}", high.0), subtitle: format!("{} · {} of {conductors}", high.1, high.2), meta: String::new() });
                                }
                            }
                            if let Some((name, n)) = &busiest {
                                items.push(ListEntry { icon: Icon::User, title: format!("Most meetings: {name}"), subtitle: format!("{n} meetings in the last {months} months"), meta: String::new() });
                            }
                            if meetings > with_conductor {
                                items.push(ListEntry { icon: Icon::Warning, title: format!("{} without a conductor", meetings - with_conductor), subtitle: "Sent meetings that nobody conducted".into(), meta: String::new() });
                            }
                            if !none.is_empty() {
                                items.push(ListEntry {
                                    icon: Icon::Users,
                                    title: format!("{} without a meeting", none.len()),
                                    subtitle: none.iter().take(5).cloned().collect::<Vec<_>>().join(", ") + if none.len() > 5 { "…" } else { "" },
                                    meta: String::new(),
                                });
                            }
                            items
                        },
                    }
                }
            }
        }
    }
}

#[component]
fn PresetsView(on_open: EventHandler<u32>) -> Element {
    let state = use_app_state();
    let prefs = state.preferences();
    let dir = state.directory();
    let conductors = dir.conductors().len();
    let first = week_order(&prefs)[0];
    let presets = dir.service_presets_sorted(first);
    rsx! {
        Text { tone: TextTone::Muted,
            "The usual meetings of a week. Fill month adds them to every matching day, and assigns only people who can conduct that meeting."
        }
        if conductors == 0 {
            Alert { tone: Tone::Warning, title: "Nobody can conduct yet",
                "Give the privilege \"Meeting for field service\" in Users › Privileges."
            }
        }
        if presets.is_empty() {
            EmptyState { icon: Icon::Settings, title: "No presets yet", description: "For example: every Monday at 10:00 at the Kingdom Hall." }
        } else {
            Grid { cols: 3, gap: Gap::Sm,
                for p in presets {
                    {
                        let id = p.id;
                        let can = dir.conductors().iter().filter(|c| p.available.contains(&c.id)).count();
                        let mut tags = vec![Tag::new(format!("{can} of {conductors} can conduct"), if can == 0 { Tone::Warning } else { Tone::Neutral }).with_icon(Icon::Users)];
                        if !p.enabled {
                            tags.insert(0, Tag::new("Off: not filled", Tone::Neutral).with_icon(Icon::Hide));
                        }
                        rsx! {
                            RecordCard {
                                key: "{id}",
                                title: p.label(),
                                avatar: p.weekday_name().to_string(),
                                subtitle: p.place.clone(),
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
fn PresetDialog(id: Option<u32>, onclose: EventHandler<()>) -> Element {
    let mut state = use_app_state();
    let toaster = use_toaster();
    let prefs = state.preferences();
    let dir = state.directory();
    let existing = id.and_then(|i| dir.service_preset(i).cloned());
    let mut weekday = use_signal(|| existing.as_ref().map(|p| p.weekday).unwrap_or(1));
    let mut time = use_signal(|| existing.as_ref().map(|p| p.time.clone()).unwrap_or_else(|| "10:00".into()));
    let mut place = use_signal(|| existing.as_ref().map(|p| p.place.clone()).unwrap_or_else(|| "Kingdom Hall".into()));
    let mut available_ids = use_signal(|| existing.as_ref().map(|p| p.available.clone()).unwrap_or_default());
    let mut enabled = use_signal(|| existing.as_ref().is_none_or(|p| p.enabled));
    let mut errors = use_signal(Vec::<(&'static str, String)>::new);
    let mut confirm = use_signal(|| false);
    let err = move |f: &str| errors.read().iter().find(|(k, _)| *k == f).map(|(_, m)| m.clone());

    let days: Vec<SelectOption> = week_order(&prefs).into_iter().map(|w| SelectOption::new(w.to_string(), WEEKDAYS[w as usize])).collect();
    let conductors: Vec<&Person> = dir.conductors();
    let rows: Vec<Row> = conductors
        .iter()
        .map(|p| {
            let what = p.appointment.map(|a| a.label().to_string()).unwrap_or_else(|| p.publisher_type.label().to_string());
            Row::new(p.id.to_string(), p.display_name(&prefs)).with_subtitle(what)
        })
        .collect();
    let available: BTreeSet<String> = conductors.iter().filter(|p| available_ids().contains(&p.id)).map(|p| p.id.to_string()).collect();
    let (can, all) = (available.len(), conductors.len());
    let save = move |_| {
        let on = enabled();
        match state.update_directory(|d| {
            let r = d.save_service_preset(id, weekday(), &time(), &place(), available_ids());
            if let Ok(saved) = r {
                d.set_service_preset_enabled(saved, on);
            }
            r
        }) {
            Ok(_) => {
                toaster.success("Preset saved.");
                onclose.call(());
            }
            Err(e) => errors.set(e),
        }
    };
    rsx! {
        Dialog {
            open: true,
            onclose: move |_| onclose.call(()),
            title: if existing.is_some() { "Weekly meeting" } else { "Add preset" },
            description: "A meeting for field service that happens every week.",
            actions: rsx! {
                EditActions {
                    on_save: save,
                    on_cancel: move |_| onclose.call(()),
                    on_delete: existing.is_some().then_some(EventHandler::new(move |_| confirm.set(true))),
                }
            },
            Stack {
                Grid { cols: 2,
                    Field { label: "Day", error: err("weekday"),
                        Select { value: weekday().to_string(), options: days, onchange: move |v: String| weekday.set(v.parse().unwrap_or(1)) }
                    }
                    Field { label: "Time", required: true, error: err("time"),
                        TextInput { kind: InputKind::Time, value: time(), oninput: move |v| time.set(v) }
                    }
                }
                Field { label: "Place", required: true, error: err("place"),
                    TextInput { value: place(), placeholder: "For example: Kingdom Hall", oninput: move |v| place.set(v) }
                }
                Switch {
                    checked: enabled(),
                    label: "On",
                    description: "Off: Fill month skips it, for example for a few weeks. Meetings already made stay. Switch it on again when it is back.",
                    onchange: move |v| enabled.set(v),
                }
                Field { label: "Who can conduct it", help: format!("{can} of {all} with the privilege. Tick who is available at this time: only they are assigned."),
                    if rows.is_empty() {
                        Text { tone: TextTone::Muted, "Nobody has the privilege \"Meeting for field service\" yet." }
                    } else {
                        PickList {
                            rows,
                            selected: available,
                            on_toggle: move |pid: String| {
                                if let Ok(p) = pid.parse::<u32>() {
                                    let mut a = available_ids.write();
                                    if !a.remove(&p) {
                                        a.insert(p);
                                    }
                                }
                            },
                        }
                    }
                }
            }
        }
        ConfirmDialog {
            open: confirm(),
            title: "Delete this preset?",
            message: "Meetings already made from it stay. Fill month no longer adds it.",
            confirm: Action::Delete,
            oncancel: move |_| confirm.set(false),
            onconfirm: move |_| {
                if let Some(i) = id {
                    state.update_directory(|d| d.delete_service_preset(i));
                }
                toaster.success("Preset deleted.");
                onclose.call(());
            },
        }
    }
}

// ---------------------------------------------------------------------------------------------
// One meeting
// ---------------------------------------------------------------------------------------------

#[component]
fn MeetingDialog(id: Option<u32>, month: Ym, onclose: EventHandler<()>) -> Element {
    let mut state = use_app_state();
    let toaster = use_toaster();
    let prefs = state.preferences();
    let dir = state.directory();
    let existing = id.and_then(|i| dir.service_meeting(i).cloned());
    let default_date = {
        let now = this_month();
        let today = people::today_iso();
        if month == now { today } else { format!("{:04}-{:02}-01", month.0, month.1) }
    };
    let mut draft = use_signal(|| match &existing {
        Some(m) => ServiceMeetingDraft { date: m.date.clone(), time: m.time.clone(), place: m.place.clone(), conductor: m.conductor, preset: m.preset },
        None => ServiceMeetingDraft { date: default_date.clone(), time: "10:00".into(), place: "Kingdom Hall".into(), conductor: None, preset: None },
    });
    let mut errors = use_signal(Vec::<(&'static str, String)>::new);
    let mut query = use_signal(String::new);
    let mut confirm = use_signal(|| false);
    let err = move |f: &str| errors.read().iter().find(|(k, _)| *k == f).map(|(_, m)| m.clone());
    let d = draft();

    // Presets: choosing one fills the time and place (and links its availability).
    let first = week_order(&prefs)[0];
    let mut preset_options = vec![SelectOption::new("", "None: a one-off meeting")];
    preset_options.extend(dir.service_presets_sorted(first).iter().map(|p| SelectOption::new(p.id.to_string(), format!("{} · {}", p.label(), p.place))));

    // Conductors: people with the privilege first (available ones on top), then the other
    // baptised publishers, for when nobody is available.
    let tokens: Vec<String> = people::fold(&query()).split_whitespace().map(str::to_string).collect();
    let found = |p: &Person| {
        let hay = people::fold(&format!("{} {}", p.first_name, p.last_name));
        tokens.iter().all(|t| hay.contains(t.as_str()))
    };
    let order = |a: Availability| match a {
        Availability::Available => 0,
        Availability::NotForThisSlot | Availability::BusyThatDay => 1,
        Availability::Away => 2,
        Availability::NoPrivilege => 3,
    };
    let mut with: Vec<(&Person, Availability)> =
        dir.conductors().into_iter().filter(|p| found(p)).map(|p| (p, dir.availability(p.id, &d.date, d.preset, id))).collect();
    with.sort_by_cached_key(|(p, a)| (order(*a), people::fold(&p.display_name(&prefs))));
    let privileged: BTreeSet<u32> = dir.conductors().iter().map(|p| p.id).collect();
    let mut others: Vec<&Person> = dir
        .people
        .iter()
        .filter(|p| p.moved_away.is_none() && p.publisher_type.is_baptised() && !privileged.contains(&p.id) && found(p))
        .collect();
    others.sort_by_cached_key(|p| people::fold(&p.display_name(&prefs)));
    let row = |p: &Person, a: Availability| {
        let mut r = Row::new(p.id.to_string(), p.display_name(&prefs));
        if let Some(t) = availability_tag(a) {
            r = r.with_tag(t);
        }
        r
    };
    let groups = vec![
        RowGroup { title: "Meeting for field service".into(), rows: with.iter().map(|(p, a)| row(p, *a)).collect() },
        RowGroup {
            title: "Other baptised publishers".into(),
            rows: others
                .iter()
                .map(|p| {
                    let a = if dir.is_away(p.id, &d.date) { Availability::Away } else { Availability::NoPrivilege };
                    row(p, a).with_subtitle(p.publisher_type.label())
                })
                .collect(),
        },
    ];
    let chosen_warning = d.conductor.and_then(|c| match dir.availability(c, &d.date, d.preset, id) {
        Availability::Available => None,
        Availability::NoPrivilege => Some("This person does not have the privilege \"Meeting for field service\"."),
        Availability::Away => Some("This person is away that day."),
        Availability::NotForThisSlot => Some("This person is not available at this time."),
        Availability::BusyThatDay => Some("This person already conducts another meeting that day."),
    });
    let status = existing.as_ref().map(|m| m.status());
    let sent_text = existing.as_ref().and_then(|m| m.sent.clone()).map(|s| {
        format!("{} · {} · {} · {}", day_title(&s.date), s.time, s.place, name_of(&dir, &prefs, s.conductor).unwrap_or_else(|| "no conductor".into()))
    });

    let save = move |_| {
        let d = draft();
        match state.update_directory(|dir| dir.save_service_meeting(id, &d)) {
            Ok(_) => {
                toaster.success(if id.is_some() { "Meeting saved. Send it when the month is ready." } else { "Meeting added. Send it when the month is ready." });
                onclose.call(());
            }
            Err(e) => errors.set(e),
        }
    };

    rsx! {
        Dialog {
            open: true,
            onclose: move |_| onclose.call(()),
            title: if existing.is_some() { "Meeting for field service" } else { "Add meeting" },
            description: "When and where, and who conducts.",
            actions: rsx! {
                EditActions {
                    on_save: save,
                    on_cancel: move |_| onclose.call(()),
                    on_delete: existing.as_ref().is_some_and(|m| !m.removed).then_some(EventHandler::new(move |_| confirm.set(true))),
                }
            },
            Stack {
                match status {
                    Some(SendStatus::Changed) => rsx! {
                        Alert { tone: Tone::Warning, title: "Changed, not sent yet",
                            "Everyone still sees the sent version: {sent_text.clone().unwrap_or_default()}."
                        }
                    },
                    Some(SendStatus::Removed) => rsx! {
                        Alert { tone: Tone::Error, title: "Removed, not sent yet",
                            Stack { gap: Gap::Sm,
                                Text { "Everyone still sees this meeting until you send the month." }
                                Cluster {
                                    ActionButton {
                                        action: Action::Undo,
                                        label: "Keep it",
                                        onclick: move |_| {
                                            if let Some(i) = id {
                                                state.update_directory(|d| d.keep_service_meeting(i));
                                            }
                                            toaster.success("The meeting is kept.");
                                            onclose.call(());
                                        },
                                    }
                                }
                            }
                        }
                    },
                    Some(SendStatus::NotSent) => rsx! {
                        Text { tone: TextTone::Small, "Not sent yet: only planners see it." }
                    },
                    _ => rsx! {},
                }
                Field { label: "Preset",
                    Select {
                        value: d.preset.map(|p| p.to_string()).unwrap_or_default(),
                        options: preset_options,
                        onchange: move |v: String| {
                            let p = v.parse::<u32>().ok();
                            let preset = p.and_then(|p| state.directory().service_preset(p).cloned());
                            let mut w = draft.write();
                            w.preset = p;
                            if let Some(s) = preset {
                                w.time = s.time;
                                w.place = s.place;
                            }
                        },
                    }
                }
                Grid { cols: 2,
                    Field { label: "Date", required: true, error: err("date"),
                        TextInput { kind: InputKind::Date, value: d.date.clone(), oninput: move |v| draft.write().date = v }
                    }
                    Field { label: "Time", required: true, error: err("time"),
                        TextInput { kind: InputKind::Time, value: d.time.clone(), oninput: move |v| draft.write().time = v }
                    }
                }
                Field { label: "Place", required: true, error: err("place"),
                    TextInput { value: d.place.clone(), oninput: move |v| draft.write().place = v }
                }
                Field { label: "Conductor", error: err("conductor"), help: chosen_warning.map(str::to_string),
                    TextInput { kind: InputKind::Search, value: query(), icon: Icon::Search, placeholder: "Search by name", oninput: move |v| query.set(v) }
                }
                ChoiceList {
                    groups,
                    selected: d.conductor.map(|c| c.to_string()),
                    none_label: "No conductor yet",
                    on_choose: move |c: Option<String>| draft.write().conductor = c.and_then(|c| c.parse().ok()),
                }
            }
        }
        ConfirmDialog {
            open: confirm(),
            title: "Delete this meeting?",
            message: if existing.as_ref().is_some_and(|m| m.sent.is_some()) { "It was sent: everyone still sees it until you send the month." } else { "It was not sent, so nobody else saw it." },
            confirm: Action::Delete,
            oncancel: move |_| confirm.set(false),
            onconfirm: move |_| {
                if let Some(i) = id {
                    state.update_directory(|d| d.delete_service_meeting(i));
                }
                toaster.success("Meeting deleted.");
                onclose.call(());
            },
        }
    }
}
