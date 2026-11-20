//! `/app` home: *For you* (personal widgets, everyone) first, then *Congregation* (widgets of
//! the pages the user may see). Widgets show more when they are made bigger.

use crate::mock;
use crate::people::{parse_date, today_iso, Activity, Directory, PublisherType};
use crate::reports::{month_label, reporting_month, service_year, service_year_label, Kind, ReportStatus, YEARLY_GOAL};
use crate::state::{use_app_state, AppState};
use crate::widgets;
use dioxus::prelude::*;
use theo_ui::*;
use theo_ui::Action; // explicit: dioxus::prelude also exports an `Action`

/// Is the widget big (more than one cell)?
fn big(w: &WidgetLayout) -> bool {
    w.cols >= 2 || w.rows >= 2
}

/// The height left for content in a widget `rows` high, after `lines` lines of 24 px (for
/// example a line with a link at the top). Widgets show what fits, never a scrollbar.
fn body_px(rows: u8, lines: usize) -> f64 {
    // A grid row is 160 px plus a 16 px gap; the header (44 px) and the padding (24 px) are
    // not for content (measured: a one-row widget has 92 px).
    rows as f64 * 176.0 - 84.0 - lines as f64 * 24.0
}

/// How many rows of `row_px` fit.
fn capacity(rows: u8, row_px: f64, lines: usize) -> usize {
    ((body_px(rows, lines) / row_px).floor() as usize).max(1)
}

/// Rows of a compact list (`ItemList { compact }`).
const COMPACT_ROW: f64 = 34.0;

/// Everything when it fits; otherwise as many as fit above an "and N more" line (20 px).
fn fit_rows<T>(items: Vec<T>, rows: u8, row_px: f64, lines: usize) -> (Vec<T>, usize) {
    let total = items.len();
    let body = body_px(rows, lines);
    if total as f64 * row_px <= body {
        return (items, 0);
    }
    let keep = (((body - 20.0) / row_px).floor() as usize).max(1);
    (items.into_iter().take(keep).collect(), total - keep)
}

fn more(n: usize) -> Element {
    rsx! {
        if n > 0 {
            Text { tone: TextTone::Small, "and {n} more." }
        }
    }
}

/// `/app` home: widgets that the user reorders and resizes.
#[component]
pub fn Dashboard() -> Element {
    let mut state = use_app_state();
    let mut editing = use_signal(|| false);
    let name = state.account().map(|a| a.display_name).unwrap_or_default();
    let first = name.split_whitespace().next().unwrap_or("").to_string();
    let congregation = state.active_congregation().map(|c| c.name).unwrap_or_default();
    let can = move |page: &str| state.can_read(page);
    // Only the widgets this user may see, personal ones first.
    let layout: Vec<WidgetLayout> = state.layout().into_iter().filter(|w| widgets::allowed(&w.id, can)).collect();
    let personal: Vec<WidgetLayout> = layout.iter().filter(|w| widgets::is_personal(&w.id)).cloned().collect();
    let congregation_widgets: Vec<WidgetLayout> = layout.iter().filter(|w| !widgets::is_personal(&w.id)).cloned().collect();
    // Widgets that were removed: they can be added back while customizing.
    let shown: Vec<String> = layout.iter().map(|w| w.id.clone()).collect();
    let hidden: Vec<&widgets::WidgetDef> = widgets::ALL.iter().filter(|d| widgets::allowed(d.id, can) && !shown.iter().any(|s| s == d.id)).collect();
    let any_congregation = widgets::ALL.iter().any(|d| !d.personal && widgets::allowed(d.id, can));
    let (p_now, c_now) = (personal.clone(), congregation_widgets.clone());

    rsx! {
        Page {
            title: "Welcome back, {first}",
            subtitle: "{congregation} · Some widgets still show sample data",
            actions: rsx! {
                if editing() && !hidden.is_empty() {
                    MenuButton {
                        label: "Add widget",
                        icon: Icon::Plus,
                        items: hidden.iter().map(|d| MenuItem::new(d.id, d.title, d.icon).with_description(if d.personal { "For you" } else { "Congregation" })).collect::<Vec<_>>(),
                        onselect: move |id: String| {
                            if let Some(d) = widgets::find(&id) {
                                let mut l = state.layout();
                                l.push(widgets::layout_of(d));
                                state.set_layout(l);
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
                Alert { tone: Tone::Info, "Drag a widget by its handle to move it, drag the corner to make it bigger (it shows more), and remove it with ×. Add widget brings it back." }
            }
            if any_congregation {
                Heading { level: HeadingLevel::H2, "For you" }
            }
            theo_ui::Dashboard {
                widgets: personal,
                editing: editing(),
                on_change: move |l: Vec<WidgetLayout>| state.set_layout(l.into_iter().chain(c_now.clone()).collect()),
                render: move |w: WidgetLayout| widget(&w, &state),
            }
            if any_congregation && (!congregation_widgets.is_empty() || editing()) {
                Heading { level: HeadingLevel::H2, "Congregation" }
                if congregation_widgets.is_empty() {
                    Text { tone: TextTone::Muted, "No congregation widgets. Add them with Add widget." }
                }
                theo_ui::Dashboard {
                    widgets: congregation_widgets,
                    editing: editing(),
                    on_change: move |l: Vec<WidgetLayout>| state.set_layout(p_now.clone().into_iter().chain(l).collect()),
                    render: move |w: WidgetLayout| widget(&w, &state),
                }
            }
        }
    }
}

fn not_linked(what: &str) -> Element {
    rsx! { Text { tone: TextTone::Muted, "Your {what} appear here when your account is linked to your person in Users." } }
}

/// "My assignments": what changed for me (until I mark it seen), then what is coming up.
#[component]
fn MyAssignments(cols: u8, rows: u8) -> Element {
    let mut state = use_app_state();
    let navigator = navigator();
    let prefs = state.preferences();
    let Some(me) = state.me() else { return not_linked("assignments") };
    let dir = state.directory();
    let today = today_iso();
    let large = cols >= 2;
    let when = |date: &str, time: &Option<String>| {
        let day = parse_date(date).map(|d| prefs.format_date(d)).unwrap_or_else(|| date.to_string());
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
    let row_px = if large { 66.0 } else { 54.0 };
    let all: Vec<AccentItem> = dir
        .my_assignments(me, &today)
        .into_iter()
        .map(|a| AccentItem {
            icon: a.kind.icon(),
            accent: a.kind.accent(),
            kind: a.kind.label().into(),
            text: format!("{} · {}", when(&a.date, &a.time), a.detail),
            badge: notices.iter().find(|n| n.what == a.kind && n.record == a.record && n.kind != crate::assignments::NoticeKind::Cancelled).map(|n| badge(n.kind)),
        })
        .collect();
    let (upcoming, hidden) = fit_rows(all, rows, row_px, 1 + cancelled.len() * 2);
    let changes = match notices.len() {
        0 => String::new(),
        1 => "1 change for you".to_string(),
        n => format!("{n} changes for you"),
    };
    let has_notices = !notices.is_empty();
    rsx! {
        Stack { gap: Gap::Sm,
            Cluster { justify: if has_notices { Justify::Between } else { Justify::End },
                if has_notices {
                    Cluster { gap: Gap::Xs,
                        Badge { tone: Tone::Info, icon: Icon::Sparkles, "{changes}" }
                        ActionButton { action: Action::Done, variant: ButtonVariant::Ghost, label: "Got it", onclick: move |_| state.mark_notices_seen(me) }
                    }
                }
                TextLink { icon: Icon::ArrowRight, onclick: move |_| { navigator.push(crate::Route::MyAssignmentsPage {}); }, "All" }
            }
            if !cancelled.is_empty() {
                AccentList { items: cancelled, large }
            }
            if upcoming.is_empty() {
                Text { tone: TextTone::Muted, "Nothing coming up for you." }
            } else {
                AccentList { items: upcoming, large }
            }
            {more(hidden)}
        }
    }
}

/// "My field service reports": the report to send now and this month so far (links on top),
/// then the months before when it is taller.
#[component]
fn MyReports(cols: u8, rows: u8) -> Element {
    let state = use_app_state();
    let navigator = navigator();
    let Some(me) = state.me() else { return not_linked("reports") };
    let dir = state.directory();
    let now = crate::ministry::this_month();
    let Some(_) = dir.my_kind(me, now) else {
        return rsx! { Text { tone: TextTone::Muted, "Bible students and visitors do not send a field service report." } };
    };
    let _ = cols;
    let rm = reporting_month();
    let sent = dir.report(me, rm).cloned();
    let log = dir.month_log(me, now);
    let so_far = if log.days > 0 {
        let studies = if log.week_studies > 0 { format!(" · {} studies", log.week_studies) } else { String::new() };
        format!("{} so far: {}{studies}", theo_ui::month_name(now.1), crate::ministry::hm_long(log.minutes))
    } else {
        format!("Nothing logged in {} yet.", theo_ui::month_name(now.1))
    };
    let earlier: Vec<ListEntry> = if rows >= 2 {
        (1..=12)
            .map(|i| crate::reports::shift(rm, -i))
            .filter_map(|m| dir.report(me, m).map(|r| (m, r)))
            .take(capacity(rows, COMPACT_ROW, 2))
            .map(|(m, r)| ListEntry { icon: Icon::Check, title: month_label(m), subtitle: super::ministry::report_text(r), meta: String::new() })
            .collect()
    } else {
        Vec::new()
    };
    let open = move |_| {
        navigator.push(crate::Route::MyMinistryPage {});
    };
    rsx! {
        Stack { gap: Gap::Sm,
            Cluster { justify: Justify::Between,
                match &sent {
                    Some(_) => rsx! { Badge { tone: Tone::Success, icon: Icon::Check, "{theo_ui::month_name(rm.1)} sent" } },
                    None => rsx! { Badge { tone: Tone::Warning, icon: Icon::Clock, "{theo_ui::month_name(rm.1)} not sent" } },
                }
                TextLink { icon: Icon::ArrowRight, onclick: open, if sent.is_some() { "Log today" } else { "Send" } }
            }
            Text { tone: TextTone::Small, "{so_far}" }
            if !earlier.is_empty() {
                ItemList { items: earlier, compact: true }
            }
        }
    }
}

/// "My goal": pioneers and auxiliary pioneers see their goal, anyone their personal goal. Small:
/// one bar; taller: this month as well, the personal goal, and month by month.
#[component]
fn MyGoal(cols: u8, rows: u8) -> Element {
    let state = use_app_state();
    let navigator = navigator();
    let Some(me) = state.me() else { return not_linked("goal") };
    let dir = state.directory();
    let Some(p) = dir.person(me).cloned() else { return rsx! {} };
    let now = crate::ministry::this_month();
    let kind = dir.kind_for(&p, now);
    let log = dir.month_log(me, now);
    let done = log.hours() + log.credit_hours();
    let personal = dir.personal_goal(me);
    let tall = rows >= 2;
    let open = move |_| {
        navigator.push(crate::Route::MyMinistryPage {});
    };
    let bar = |label: String, have: u32, goal: u32| {
        rsx! {
            ProgressRow {
                label,
                detail: format!("{have} / {goal} h"),
                value: have as f64,
                max: goal.max(1) as f64,
                note: if have >= goal { "Reached".to_string() } else { format!("{} h to go", goal - have) },
                tone: if have >= goal { Tone::Success } else { Tone::Neutral },
            }
        }
    };
    let what = match (kind, p.publisher_type) {
        (Kind::Pioneer, PublisherType::SpecialPioneer) => "Special pioneer",
        (Kind::Pioneer, PublisherType::Missionary) => "Missionary",
        (Kind::Pioneer, _) => "Regular pioneer",
        (Kind::ContinuousAuxiliary, _) => "Continuous auxiliary",
        (Kind::AuxiliaryMonth, _) => "Auxiliary pioneer",
        _ => "Publisher",
    };
    let top = rsx! {
        Cluster { justify: Justify::Between,
            Text { tone: TextTone::Small, "{what}" }
            TextLink { icon: Icon::ArrowRight, onclick: open, "Log" }
        }
    };
    let month = theo_ui::month_name(now.1);
    match kind {
        Kind::Publisher => match personal {
            Some(g) => rsx! {
                Stack { gap: Gap::Sm, {top} {bar(format!("{month} · my goal"), done, g)} }
            },
            None => rsx! {
                Stack { gap: Gap::Sm,
                    Text { tone: TextTone::Muted, "You are not pioneering this month." }
                    TextLink { icon: Icon::ArrowRight, onclick: open, "Set a personal goal" }
                }
            },
        },
        Kind::AuxiliaryMonth | Kind::ContinuousAuxiliary => {
            let goal = dir.aux_goal(now);
            rsx! {
                Stack { gap: Gap::Sm,
                    {top}
                    {bar(format!("{month} · goal"), done, goal)}
                    if tall {
                        if let Some(g) = personal {
                            {bar("My goal".to_string(), done, g)}
                        }
                    }
                }
            }
        }
        Kind::Pioneer => {
            let start = service_year(now);
            let reported = dir.goal_progress(me, start).map(|g| g.hours).unwrap_or(0);
            let hours = reported + if dir.report(me, now).is_none() { done } else { 0 };
            let months_done = crate::reports::service_year_months(start).iter().filter(|m| **m <= now).count() as u32;
            let expected = YEARLY_GOAL * months_done / 12;
            let months: Vec<StatItem> = crate::reports::service_year_months(start)
                .into_iter()
                .filter(|m| *m <= now)
                .map(|m| {
                    let h = if m == now { done } else { dir.report(me, m).map(|r| r.total()).unwrap_or(0) };
                    StatItem::new(theo_ui::month_name(m.1), format!("{h} h")).with_tone(if h >= YEARLY_GOAL / 12 { Tone::Success } else { Tone::Neutral })
                })
                .collect();
            rsx! {
                Stack { gap: Gap::Sm,
                    {top}
                    ProgressRow {
                        label: format!("Service year {}", service_year_label(start)),
                        detail: format!("{hours} / {YEARLY_GOAL} h"),
                        value: hours as f64,
                        max: YEARLY_GOAL as f64,
                        marker: expected as f64,
                        note: if hours >= expected { "On track".to_string() } else { format!("{} h behind", expected - hours) },
                        tone: if hours >= expected { Tone::Success } else { Tone::Warning },
                    }
                    if tall {
                        {bar(format!("{month} · monthly average"), done, YEARLY_GOAL / 12)}
                        if rows >= 3 || cols >= 2 {
                            StatList { items: months, columns: if cols >= 2 { 2 } else { 1 } }
                        }
                    }
                }
            }
        }
    }
}

/// "Meetings for field service": the sent meetings of the next 7 days.
#[component]
fn ServiceMeetings(rows: u8) -> Element {
    let state = use_app_state();
    let prefs = state.preferences();
    let dir = state.directory();
    let today = today_iso();
    let until = crate::events::add_days(&today, 7).unwrap_or_else(|| today.clone());
    let mut list: Vec<crate::service_meetings::MeetingVersion> = dir.service_meetings.iter().filter(|m| !m.removed).filter_map(|m| m.sent.clone()).filter(|v| v.date >= today && v.date <= until).collect();
    list.sort_by(|a, b| (&a.date, &a.time).cmp(&(&b.date, &b.time)));
    let items: Vec<ListEntry> = list
        .iter()
        .map(|v| ListEntry {
            icon: Icon::Group,
            title: format!("{} · {}", parse_date(&v.date).map(|d| prefs.format_date(d)).unwrap_or_default(), prefs.format_hhmm(&v.time)),
            subtitle: match v.conductor.and_then(|c| dir.person(c)) {
                Some(p) => format!("{} · {}", v.place, p.display_name(&prefs)),
                None => v.place.clone(),
            },
            meta: String::new(),
        })
        .collect();
    if items.is_empty() {
        return rsx! { Text { tone: TextTone::Muted, "No meetings for field service in the next 7 days." } };
    }
    let (items, hidden) = fit_rows(items, rows, COMPACT_ROW, 0);
    rsx! {
        ItemList { items, compact: true }
        {more(hidden)}
    }
}

/// "Upcoming events": the events I may see, from today.
#[component]
fn Events(rows: u8) -> Element {
    let state = use_app_state();
    let prefs = state.preferences();
    let dir = state.directory();
    let today = today_iso();
    let me = state.me();
    let mut list: Vec<&crate::events::Event> = dir.visible_events(me, state.i_am_admin()).into_iter().filter(|e| e.to >= today).collect();
    list.sort_by(|a, b| a.from.cmp(&b.from));
    let items: Vec<ListEntry> = list
        .iter()
        .map(|e| {
            let from = parse_date(&e.from).map(|d| prefs.format_date(d)).unwrap_or_default();
            let days = crate::territories::days_between(&today, &e.from).unwrap_or(0);
            ListEntry {
                icon: Icon::Calendar,
                title: e.title.clone(),
                subtitle: if e.from == e.to { from } else { format!("{from} – {}", parse_date(&e.to).map(|d| prefs.format_date(d)).unwrap_or_default()) },
                meta: match days {
                    d if d <= 0 => "Now".into(),
                    1 => "Tomorrow".into(),
                    d if d < 14 => format!("{d} days"),
                    d => format!("{} wk", d / 7),
                },
            }
        })
        .collect();
    if items.is_empty() {
        return rsx! { Text { tone: TextTone::Muted, "No upcoming events." } };
    }
    let (items, hidden) = fit_rows(items, rows, COMPACT_ROW, 0);
    rsx! {
        ItemList { items, compact: true }
        {more(hidden)}
    }
}

/// "Publishers": how many, and by kind when bigger.
fn publishers(dir: &Directory, w: &WidgetLayout) -> Element {
    let (value, delta) = super::reports::publishers_widget_values(dir);
    let tile = rsx! { StatTile { label: "Publishers", value, icon: Icon::Users, delta, trend: Trend::Flat } };
    if w.rows < 2 {
        return tile;
    }
    let people = dir.reporters();
    let now = crate::ministry::this_month();
    let count = |f: &dyn Fn(&&crate::people::Person) -> bool| people.iter().filter(|p| f(p)).count().to_string();
    let mut items = vec![
        StatItem::new("Elders", count(&|p| p.appointment == Some(crate::people::Appointment::Elder))),
        StatItem::new("Ministerial servants", count(&|p| p.appointment == Some(crate::people::Appointment::MinisterialServant))),
        StatItem::new("Regular pioneers", count(&|p| p.publisher_type == PublisherType::RegularPioneer)),
        StatItem::new("Special pioneers and missionaries", count(&|p| matches!(p.publisher_type, PublisherType::SpecialPioneer | PublisherType::Missionary))),
        StatItem::new("Continuous auxiliary pioneers", count(&|p| p.publisher_type == PublisherType::ContinuousAuxiliaryPioneer)),
        StatItem::new(format!("Auxiliary pioneers in {}", theo_ui::month_name(now.1)), count(&|p| p.publisher_type == PublisherType::BaptisedPublisher && dir.is_aux(p.id, now))),
        StatItem::new("Unbaptised publishers", count(&|p| p.publisher_type == PublisherType::UnbaptisedPublisher)),
        StatItem::new("Irregular", count(&|p| dir.activity(p) == Some(Activity::Irregular))).with_tone(Tone::Warning),
        StatItem::new("Inactive", count(&|p| dir.activity(p) == Some(Activity::Inactive))).with_tone(Tone::Error),
    ];
    // Only what fits (two per line when wide).
    let per_line = if w.cols >= 2 { 2 } else { 1 };
    items.truncate(capacity(w.rows, 33.0, 3) * per_line);
    rsx! {
        Stack { gap: Gap::Sm,
            {tile}
            StatList { items, columns: if w.cols >= 2 { 2 } else { 1 } }
        }
    }
}

/// "Field service reports": received of the reporting month, and who is missing when bigger.
#[component]
fn ReportsWidget(cols: u8, rows: u8) -> Element {
    let state = use_app_state();
    let navigator = navigator();
    let prefs = state.preferences();
    let dir = state.directory();
    let w = WidgetLayout::new("", "", Icon::Report, cols, rows);
    let (label, value, delta, done) = super::reports::reports_widget_values(&dir);
    let tile = rsx! { StatTile { label, value, icon: Icon::Footprints, delta, trend: if done { Trend::Up } else { Trend::Down } } };
    if !big(&w) {
        return tile;
    }
    let rm = reporting_month();
    let mut missing: Vec<(String, ReportStatus)> = dir.reporters().iter().map(|p| (p.display_name(&prefs), dir.report_status(p.id, rm))).filter(|(_, s)| *s != ReportStatus::Received).collect();
    missing.sort_by(|a, b| (a.1 != ReportStatus::Late, &a.0).cmp(&(b.1 != ReportStatus::Late, &b.0)));
    let n = missing.len();
    let per_line = if cols >= 2 { 2 } else { 1 };
    let shown = (capacity(rows, 33.0, 3).saturating_sub(0) * per_line).max(1);
    let items: Vec<StatItem> = missing
        .iter()
        .take(shown)
        .map(|(name, s)| match s {
            ReportStatus::Late => StatItem::new(name.clone(), "Late").with_tone(Tone::Error),
            ReportStatus::Missing => StatItem::new(name.clone(), "Missing").with_tone(Tone::Warning),
            _ => StatItem::new(name.clone(), "Not yet"),
        })
        .collect();
    rsx! {
        Stack { gap: Gap::Sm,
            {tile}
            Cluster { justify: Justify::Between,
                Text { tone: TextTone::Small, if n == 0 { "Everyone reported {month_label(rm)}." } else { "Missing for {month_label(rm)}:" } }
                TextLink { icon: Icon::ArrowRight, onclick: move |_| { navigator.push(crate::Route::ReportsPage {}); }, "Open" }
            }
            if n > 0 {
                StatList { items, columns: if cols >= 2 { 2 } else { 1 } }
                {more(n.saturating_sub(shown))}
            }
        }
    }
}

/// "Territories": how many are assigned, and the late ones when bigger.
fn territories(dir: &Directory, prefs: &crate::prefs::Preferences, w: &WidgetLayout) -> Element {
    let (value, delta, ok) = super::territories::territories_widget_values(dir);
    let tile = rsx! { StatTile { label: "Assigned", value, icon: Icon::Map, delta, trend: if ok { Trend::Flat } else { Trend::Down } } };
    if !big(w) {
        return tile;
    }
    let today = today_iso();
    let mut late: Vec<&crate::territories::TerritoryAssignment> = dir.territory_assignments.iter().filter(|a| dir.is_late(a, &today)).collect();
    late.sort_by(|a, b| a.assigned.cmp(&b.assigned));
    let items: Vec<StatItem> = late
        .iter()
        .take(capacity(w.rows, 33.0, 3))
        .filter_map(|a| {
            let t = dir.territory(a.territory)?;
            let who = dir.person(a.person).map(|p| p.display_name(prefs)).unwrap_or_default();
            let days = crate::territories::days_between(&a.assigned, &today).unwrap_or(0);
            Some(StatItem::new(format!("Territory {} · {who}", t.title), crate::territories::duration_label(days)).with_tone(Tone::Error))
        })
        .collect();
    let available = dir.territories.iter().filter(|t| dir.current_assignment(t.id).is_none()).count();
    rsx! {
        Stack { gap: Gap::Sm,
            {tile}
            Text { tone: TextTone::Small, "{available} available. Late:" }
            if items.is_empty() {
                Text { tone: TextTone::Small, "None." }
            } else {
                StatList { items }
            }
        }
    }
}

/// "Away this week": absences in the next 7 days.
fn away(dir: &Directory, prefs: &crate::prefs::Preferences, w: &WidgetLayout) -> Element {
    let today = today_iso();
    let until = crate::events::add_days(&today, 6).unwrap_or_else(|| today.clone());
    let mut list: Vec<&crate::absences::Absence> = dir.absences.iter().filter(|a| a.from <= until && a.to >= today).collect();
    list.sort_by(|a, b| a.from.cmp(&b.from));
    if list.is_empty() {
        return rsx! { Text { tone: TextTone::Muted, "Nobody is away this week." } };
    }
    let day = |s: &str| parse_date(s).map(|d| prefs.format_date(d)).unwrap_or_default();
    let items: Vec<ListEntry> = list
        .iter()
        .map(|a| ListEntry {
            icon: Icon::Absence,
            title: match a.person.and_then(|p| dir.person(p)) {
                Some(p) => p.display_name(prefs),
                None => "The whole congregation".into(),
            },
            subtitle: if a.from == a.to { day(&a.from) } else { format!("{} – {}", day(&a.from), day(&a.to)) },
            meta: String::new(),
        })
        .collect();
    let (items, hidden) = fit_rows(items, w.rows, COMPACT_ROW, 0);
    rsx! {
        ItemList { items, compact: true }
        {more(hidden)}
    }
}

fn widget(w: &WidgetLayout, state: &AppState) -> Element {
    let p = state.preferences();
    let dir = state.directory();
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
    let (cols, rows) = (w.cols, w.rows);
    match w.id.as_str() {
        "next-meeting" => rsx! {
            Stack { gap: Gap::Sm,
                Cluster { justify: Justify::Between,
                    Heading { level: HeadingLevel::H3, "{meeting}" }
                    Badge { tone: Tone::Success, icon: Icon::Check, "All parts assigned" }
                }
                Text { tone: TextTone::Muted, "{names}" }
            }
        },
        "assignments" => rsx! { MyAssignments { cols, rows } },
        "my-reports" => rsx! { MyReports { cols, rows } },
        "my-goal" => rsx! { MyGoal { cols, rows } },
        "my-territories" => rsx! { super::territories::MyTerritoriesWidget { capacity: capacity(rows, COMPACT_ROW, 0) } },
        "service-meetings" => rsx! { ServiceMeetings { rows } },
        "events" => rsx! { Events { rows } },
        "publishers" => publishers(&dir, w),
        "reports" => rsx! { ReportsWidget { cols, rows } },
        "territories" => territories(&dir, &p, w),
        "away" => away(&dir, &p, w),
        "attendance" => rsx! { BarChart { data: mock::attendance(), highlight_last: true } },
        "duties" => {
            let (items, hidden) = fit_rows(mock::duties(), rows, COMPACT_ROW, 0);
            rsx! {
                ItemList { items, compact: true }
                {more(hidden)}
            }
        }
        _ => rsx! { Skeleton {} },
    }
}
