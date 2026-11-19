//! *Ministry › Field service reports*: a month view (enter and check reports), statistics for a
//! service year, and the auxiliary pioneer goals. Also the report form and the reports card of
//! the person page. Behaviour: `docs/model/m10-field-service-reports.md`.

use super::users::{save_export, ExportMenu};
use crate::export::{Format, Table};
use crate::people::{self, Activity, Directory, Person, PublisherType};
use crate::reports::{
    self, month_label, reporting_month, service_year, service_year_label, service_year_months, short_month, GoalProgress,
    Kind, Report, ReportDraft, ReportStatus, Ym, AUX_GOAL, AUX_GOAL_SHORT, CREDIT_MAX, LATE_DAY, WARN_DAY, YEARLY_GOAL,
};
use crate::state::use_app_state;
use crate::Route;
use dioxus::prelude::*;
use theo_ui::*;
use theo_ui::Action; // explicit: dioxus::prelude also exports an `Action`

fn kind_label(k: Kind) -> &'static str {
    match k {
        Kind::Publisher => "Publisher",
        Kind::AuxiliaryMonth => "Auxiliary pioneer this month",
        Kind::ContinuousAuxiliary => "Continuous auxiliary pioneer",
        Kind::Pioneer => "Pioneer",
    }
}

/// "52 h (+8 credit) · 3 studies" or "Took part · 1 study".
fn summary(r: &Report) -> String {
    let studies = if r.studies == 1 { "1 study".to_string() } else { format!("{} studies", r.studies) };
    match r.hours {
        Some(h) => match r.credit {
            Some(c) => format!("{} h ({h} + {c} credit) · {studies}", r.total()),
            None => format!("{h} h · {studies}"),
        },
        None => format!("{} · {studies}", if r.took_part { "Took part" } else { "Did not take part" }),
    }
}

fn goal_note(g: &GoalProgress) -> (String, Tone) {
    match g.kind {
        Kind::Pioneer if g.on_track() => (format!("On track · {} h expected by now", g.expected), Tone::Success),
        Kind::Pioneer => (format!("{} h behind · {} h expected by now", g.expected - g.hours, g.expected), Tone::Warning),
        _ => {
            let tone = if g.months_met == g.months { Tone::Success } else { Tone::Warning };
            (format!("Goal met in {} of {} {}", g.months_met, g.months, if g.months == 1 { "month" } else { "months" }), tone)
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Page and tabs
// ---------------------------------------------------------------------------------------------

/// `/app/ministry/reports`
#[component]
pub fn ReportsPage() -> Element {
    let mut state = use_app_state();
    let congregation = state.active_congregation().map(|c| c.name).unwrap_or_default();
    let tab = (state.report_tab)();
    let (min, max) = state.directory().report_range();
    let ym = (state.report_month)().unwrap_or_else(reporting_month);
    rsx! {
        Page {
            title: "Field service reports",
            subtitle: "{congregation} · Reports, statistics and goals",
            // The tabs (and the month on the Month tab) stay in view while you scroll.
            StickyBar {
                Segmented {
                    value: tab.clone(),
                    options: vec![
                        Segment { value: "month".into(), label: "Month".into(), icon: Some(Icon::Calendar) },
                        Segment { value: "statistics".into(), label: "Statistics".into(), icon: Some(Icon::Chart) },
                        Segment { value: "goals".into(), label: "Goals".into(), icon: Some(Icon::Privilege) },
                    ],
                    onchange: move |v: String| state.report_tab.set(v),
                }
                if tab == "month" {
                    MonthPicker { value: ym, min, max, today: reporting_month(), full_width: true, onchange: move |v| state.report_month.set(Some(v)) }
                }
            }
            match tab.as_str() {
                "statistics" => rsx! { StatisticsTab {} },
                "goals" => rsx! { GoalsTab {} },
                _ => rsx! { MonthTab {} },
            }
        }
    }
}

/// The report status filter (the other filters are the standard people filters).
const F_REPORT_STATUS: &str = "report-status";

/// The people filters the reports list shares with *All users* (not status or consent).
const PEOPLE_FACETS: [&str; 6] =
    [people::FACET_TYPE, people::FACET_APPOINTMENT, people::FACET_GROUP, people::FACET_ACTIVITY, people::FACET_GENDER, people::FACET_ANOINTED];

#[component]
fn MonthTab() -> Element {
    let mut state = use_app_state();
    let toaster = use_toaster();
    let prefs = state.preferences();
    let dir = state.directory();
    let congregation = state.active_congregation().map(|c| c.name).unwrap_or_default();
    let ym = (state.report_month)().unwrap_or_else(reporting_month);
    let mut editing = use_signal(|| None::<(u32, Ym)>);
    let query = (state.report_query)();
    let sel = (state.report_filters)();

    let s = dir.month_summary(ym);
    let reporters: Vec<&Person> = dir.reporters();
    let status_dir = dir.clone();
    let extra = [people::ExtraFacet {
        def: (
            F_REPORT_STATUS,
            "Report status",
            Icon::Report,
            vec![
                ("received".into(), "Received".into()),
                ("not-yet".into(), "Not in yet".into()),
                ("missing".into(), "Missing".into()),
                ("late".into(), "Late".into()),
            ],
        ),
        values: Box::new(move |p: &Person| vec![status_dir.report_status(p.id, ym).value().to_string()]),
    }];
    let facets = people::facets_for(&dir, &reporters, &query, &sel, people::standard_facets(&dir, &PEOPLE_FACETS), &extra);
    let shown: Vec<&Person> = people::filter_pool(&dir, &reporters, &query, &sel, &extra, &prefs);
    let cards: Vec<(u32, String, Option<String>, Vec<Tag>)> = shown
        .iter()
        .map(|p| {
            let kind = dir.kind_for(p, ym);
            let mut tags = Vec::new();
            match dir.report(p.id, ym) {
                None => {
                    match dir.report_status(p.id, ym) {
                        ReportStatus::Late => tags.push(Tag::new("Late", Tone::Error).with_icon(Icon::Error)),
                        ReportStatus::Missing => tags.push(Tag::new("Missing", Tone::Warning).with_icon(Icon::Warning)),
                        _ => tags.push(Tag::new("Not in yet", Tone::Neutral).with_icon(Icon::Clock)),
                    }
                    if kind == Kind::AuxiliaryMonth {
                        tags.push(Tag::new("Auxiliary pioneer", Tone::Info));
                    }
                }
                Some(r) => {
                    if kind != Kind::Publisher {
                        tags.push(Tag::new(if kind == Kind::Pioneer { p.publisher_type.label() } else { "Auxiliary pioneer" }, Tone::Info));
                    }
                    match r.hours {
                        Some(_) => {
                            let goal = matches!(kind, Kind::AuxiliaryMonth | Kind::ContinuousAuxiliary).then(|| dir.aux_goal(ym));
                            let tone = match goal {
                                Some(g) if r.total() >= g => Tone::Success,
                                Some(_) => Tone::Warning,
                                None => Tone::Neutral,
                            };
                            let label = match goal {
                                Some(g) => format!("{} of {g} h", r.total()),
                                None => format!("{} h", r.total()),
                            };
                            tags.push(Tag::new(label, tone).with_icon(Icon::Clock));
                        }
                        None if r.took_part => tags.push(Tag::new("Took part", Tone::Success).with_icon(Icon::Check)),
                        None => tags.push(Tag::new("Did not take part", Tone::Neutral)),
                    }
                    if r.studies > 0 {
                        tags.push(Tag::new(if r.studies == 1 { "1 study".to_string() } else { format!("{} studies", r.studies) }, Tone::Neutral).with_icon(Icon::BookOpen));
                    }
                    if r.notes.is_some() {
                        tags.push(Tag::new("Note", Tone::Neutral).with_icon(Icon::Note));
                    }
                }
            }
            let group = dir.home_group(p.id).map(|g| g.name.clone());
            (p.id, p.display_name(&prefs), group, tags)
        })
        .collect();
    let summary_text = format!("{} of {} publishers", cards.len(), reporters.len());

    let on_export = {
        let (dir, prefs, congregation) = (dir.clone(), prefs.clone(), congregation.clone());
        let ids: Vec<u32> = shown.iter().map(|p| p.id).collect();
        move |id: String| {
            let Some(format) = Format::parse(&id) else { return };
            let rows: Vec<Vec<String>> = ids
                .iter()
                .filter_map(|n| dir.person(*n))
                .map(|p| {
                    let r = dir.report(p.id, ym);
                    let opt = |v: Option<u32>| v.map(|x| x.to_string()).unwrap_or_default();
                    vec![
                        p.display_name(&prefs),
                        dir.home_group(p.id).map(|g| g.name.clone()).unwrap_or_default(),
                        kind_label(dir.kind_for(p, ym)).to_string(),
                        match r {
                            None => if dir.report_status(p.id, ym) == ReportStatus::Late { "Late".into() } else { "Missing".into() },
                            Some(r) if r.participated() => "Yes".into(),
                            Some(_) => "No".into(),
                        },
                        opt(r.and_then(|r| r.hours)),
                        opt(r.and_then(|r| r.credit)),
                        r.map(|r| r.studies.to_string()).unwrap_or_default(),
                        if format == Format::Csv { r.and_then(|r| r.notes.clone()).unwrap_or_default() } else { String::new() },
                    ]
                })
                .collect();
            let mut columns = vec!["Name", "Group", "Kind", "Took part", "Hours", "Credit", "Bible studies"];
            let mut widths = vec![2.2, 1.0, 2.0, 1.0, 0.8, 0.8, 1.0];
            if format == Format::Csv {
                columns.push("Notes");
                widths.push(2.0);
            }
            let rows = if format == Format::Csv { rows } else { rows.into_iter().map(|mut r| { r.pop(); r }).collect() };
            let table = Table {
                title: format!("{congregation} · Field service reports · {}", month_label(ym)),
                subtitle: format!("{} publishers", rows.len()),
                columns: columns.iter().map(|c| c.to_string()).collect(),
                widths,
                rows,
            };
            save_export(&table, format, &congregation, &format!("reports-{}-{:02}", ym.0, ym.1), toaster);
        }
    };

    rsx! {
        Cluster { justify: Justify::Between,
            Text { tone: TextTone::Muted,
                if ym == reporting_month() {
                    "Publishers report {month_label(ym)} now. Reports are due by the {WARN_DAY}th; after the {LATE_DAY}th they are late."
                } else {
                    "Reports for {month_label(ym)}."
                }
            }
            ExportMenu { csv_description: "Every field, for spreadsheets", onselect: on_export }
        }
        Grid { cols: 4, gap: Gap::Sm, compact: true,
            Card { StatTile {
                label: "Received",
                value: "{s.received} / {s.expected}",
                icon: Icon::Report,
                delta: if s.expected > s.received { format!("{} missing", s.expected - s.received) } else { "All in".to_string() },
                trend: if s.expected > s.received { Trend::Down } else { Trend::Up },
            } }
            Card { StatTile { label: "Took part", value: "{s.took_part}", icon: Icon::Footprints } }
            Card { StatTile { label: "Hours (pioneers)", value: "{s.hours}", icon: Icon::Clock } }
            Card { StatTile { label: "Bible studies", value: "{s.studies}", icon: Icon::BookOpen } }
        }
        FilterBar {
            query,
            on_query: move |v| state.report_query.set(v),
            search_placeholder: "Search by name",
            facets,
            selected: sel,
            on_select: move |v| state.report_filters.set(v),
            summary: summary_text,
        }
        if cards.is_empty() {
            EmptyState { icon: Icon::Search, title: "No publishers match", description: "Try another search, or remove some filters." }
        } else {
            Grid { cols: 3, gap: Gap::Sm,
                for (id, name, subtitle, tags) in cards {
                    RecordCard {
                        key: "{id}",
                        title: name.clone(),
                        avatar: name,
                        subtitle,
                        tags,
                        onclick: move |_| editing.set(Some((id, ym))),
                    }
                }
            }
        }
        if let Some((person, month)) = editing() {
            ReportDialog { key: "{person}-{month.0}-{month.1}", person, month, onclose: move |_| editing.set(None) }
        }
    }
}

#[component]
fn ServiceYearSelect(value: i32, onchange: EventHandler<i32>) -> Element {
    let state = use_app_state();
    let (min, max) = state.directory().report_range();
    let options: Vec<SelectOption> = (service_year(min)..=service_year(max))
        .rev()
        .map(|y| SelectOption::new(y.to_string(), service_year_label(y)))
        .collect();
    rsx! {
        Field { label: "Service year (September to August)",
            Select { value: value.to_string(), options, onchange: move |v: String| if let Ok(y) = v.parse() { onchange.call(y) } }
        }
    }
}

#[component]
fn StatisticsTab() -> Element {
    let state = use_app_state();
    let dir = state.directory();
    let mut year = use_signal(|| service_year(reporting_month()));
    let months: Vec<Ym> = service_year_months(year()).into_iter().filter(|m| *m <= reporting_month()).collect();
    let sums: Vec<_> = months.iter().map(|m| (*m, dir.month_summary(*m))).collect();
    let n = sums.len().max(1) as f64;
    let avg_took = sums.iter().map(|(_, s)| s.took_part as f64).sum::<f64>() / n;
    let avg_received = sums.iter().map(|(_, s)| if s.expected == 0 { 0.0 } else { s.received as f64 / s.expected as f64 * 100.0 }).sum::<f64>() / n;
    let hours: u32 = sums.iter().map(|(_, s)| s.hours).sum();
    let avg_studies = sums.iter().map(|(_, s)| f64::from(s.studies)).sum::<f64>() / n;
    let chart: Vec<ChartPoint> = sums.iter().map(|(m, s)| ChartPoint::new(short_month(*m), s.took_part as f64)).collect();
    let groups = dir.group_stats(&months);
    let goals = dir.goals(year());
    let pioneers = goals.iter().filter(|g| g.kind == Kind::Pioneer).count();
    // Averages per publisher.
    let reporter_count = dir.reporters().len().max(1) as f64;
    let pioneer_hours: Vec<u32> = goals.iter().filter(|g| g.kind == Kind::Pioneer).map(|g| g.hours).collect();
    let avg_pioneer_month = if pioneer_hours.is_empty() { 0.0 } else { pioneer_hours.iter().sum::<u32>() as f64 / pioneer_hours.len() as f64 / n };
    let studies_per_publisher = avg_studies / reporter_count;

    rsx! {
        Grid { cols: 3,
            ServiceYearSelect { value: year(), onchange: move |y| year.set(y) }
        }
        if months.is_empty() {
            EmptyState { icon: Icon::Calendar, title: "No reports yet in this service year" }
        } else {
            Grid { cols: 4, gap: Gap::Sm, compact: true,
                Card { StatTile { label: "Took part (average per month)", value: format!("{avg_took:.1}"), icon: Icon::Footprints } }
                Card { StatTile { label: "Reports received (average)", value: format!("{avg_received:.0}%"), icon: Icon::Report } }
                Card { StatTile { label: "Hours of pioneers (total)", value: "{hours}", icon: Icon::Clock, delta: format!("{} months", months.len()) } }
                Card { StatTile { label: "Bible studies (average per month)", value: format!("{avg_studies:.1}"), icon: Icon::BookOpen } }
            }
            Grid { cols: 2,
                Card { title: "Publishers who took part, per month", icon: Icon::Chart,
                    BarChart { data: chart, highlight_last: true }
                }
                Card { title: "Averages per publisher", icon: Icon::Users,
                    ItemList {
                        items: vec![
                            ListEntry { icon: Icon::Clock, title: "Hours per pioneer per month".into(), subtitle: format!("{pioneers} regular or special pioneers and missionaries"), meta: format!("{avg_pioneer_month:.1} h") },
                            ListEntry { icon: Icon::BookOpen, title: "Bible studies per publisher per month".into(), subtitle: format!("{} publishers", dir.reporters().len()), meta: format!("{studies_per_publisher:.2}") },
                            ListEntry { icon: Icon::Footprints, title: "Share who took part".into(), subtitle: "Average of the months".into(), meta: format!("{:.0}%", avg_took / reporter_count * 100.0) },
                        ],
                    }
                }
            }
            Card { title: "Field service groups", icon: Icon::Group,
                ItemList {
                    items: groups.iter().map(|g| ListEntry {
                        icon: Icon::Group,
                        title: g.name.clone(),
                        subtitle: format!("{:.1} of {} take part per month · {:.1} Bible studies per month · {} h of pioneers", g.avg_took_part, g.members, g.avg_studies, g.hours),
                        meta: format!("{:.0}% received", g.received_pct),
                    }).collect(),
                }
            }
        }
    }
}

#[component]
fn GoalsTab() -> Element {
    let state = use_app_state();
    let prefs = state.preferences();
    let dir = state.directory();
    let mut year = use_signal(|| service_year(reporting_month()));
    let mut settings = use_signal(|| false);
    let goals = dir.goals(year());
    // One section per kind of publisher, in this order; empty sections are not shown.
    let sections: Vec<(&str, Vec<&GoalProgress>)> = [
        ("Regular pioneers", Some(PublisherType::RegularPioneer)),
        ("Special pioneers", Some(PublisherType::SpecialPioneer)),
        ("Missionaries", Some(PublisherType::Missionary)),
        ("Continuous auxiliary pioneers", Some(PublisherType::ContinuousAuxiliaryPioneer)),
        ("Auxiliary pioneers (single months)", None),
    ]
    .into_iter()
    .map(|(title, ty)| {
        let list: Vec<&GoalProgress> = goals
            .iter()
            .filter(|g| match ty {
                Some(t) => g.kind != Kind::AuxiliaryMonth && dir.person(g.person).is_some_and(|p| p.publisher_type == t),
                None => g.kind == Kind::AuxiliaryMonth,
            })
            .collect();
        (title, list)
    })
    .filter(|(_, l)| !l.is_empty())
    .collect();
    rsx! {
        Cluster { align: Align::End,
            ServiceYearSelect { value: year(), onchange: move |y| year.set(y) }
            ActionButton { action: Action::Settings, label: "Goal settings", onclick: move |_| settings.set(true) }
        }
        if sections.is_empty() {
            EmptyState { icon: Icon::Privilege, title: "Nobody has a goal in this service year" }
        }
        for (title, list) in sections {
            Card { key: "{title}", title: "{title}", icon: Icon::Privilege, actions: rsx! { Badge { tone: Tone::Neutral, "{list.len()}" } },
                Grid { cols: 2, gap: Gap::Lg,
                    for g in list {
                        {
                            let (note, tone) = goal_note(g);
                            let name = dir.person(g.person).map(|p| p.display_name(&prefs)).unwrap_or_default();
                            rsx! {
                                ProgressRow {
                                    key: "{g.person}",
                                    label: name,
                                    detail: format!("{} / {} h", g.hours, g.goal),
                                    value: f64::from(g.hours),
                                    max: f64::from(g.goal),
                                    marker: (g.kind == Kind::Pioneer).then_some(f64::from(g.expected)),
                                    note,
                                    tone,
                                }
                            }
                        }
                    }
                }
            }
        }
        if settings() {
            GoalSettingsDialog { year: year(), onclose: move |_| settings.set(false) }
        }
    }
}

/// The goals of a service year: 30 or 15 hours per month for auxiliary pioneers, and the goal of
/// each kind of publisher. Changes apply at once.
#[component]
fn GoalSettingsDialog(year: i32, onclose: EventHandler<()>) -> Element {
    let mut state = use_app_state();
    let dir = state.directory();
    let months = service_year_months(year);
    rsx! {
        Dialog {
            open: true,
            onclose: move |_| onclose.call(()),
            title: "Goal settings · {service_year_label(year)}",
            description: "Every month has a {AUX_GOAL}-hour goal for auxiliary pioneers. Choose {AUX_GOAL_SHORT} hours for months like the Memorial campaign, the regional convention campaign or the circuit overseer's visit.",
            actions: rsx! { ActionButton { action: Action::Done, onclick: move |_| onclose.call(()) } },
            Stack { gap: Gap::Lg,
                Grid { cols: 2, gap: Gap::Sm,
                    for m in months {
                        {
                            let short = dir.aux_goal(m) == AUX_GOAL_SHORT;
                            rsx! {
                                Field { key: "{m.0}-{m.1}", label: month_label(m),
                                    Segmented {
                                        value: if short { "short" } else { "full" },
                                        options: vec![
                                            Segment { value: "full".into(), label: format!("{AUX_GOAL} h"), icon: None },
                                            Segment { value: "short".into(), label: format!("{AUX_GOAL_SHORT} h"), icon: None },
                                        ],
                                        onchange: move |v: String| state.update_directory(|d| d.set_aux_goal(m, v == "short")),
                                    }
                                }
                            }
                        }
                    }
                }
                ItemList {
                    items: vec![
                        ListEntry { icon: Icon::Privilege, title: "Regular and special pioneers, missionaries".into(), subtitle: "Per service year".into(), meta: format!("{YEARLY_GOAL} h") },
                        ListEntry { icon: Icon::Clock, title: "Auxiliary pioneers".into(), subtitle: "Per month, as set above".into(), meta: format!("{AUX_GOAL} or {AUX_GOAL_SHORT} h") },
                        ListEntry { icon: Icon::Clock, title: "Credit hours".into(), subtitle: "Pioneers only; they count towards the goal".into(), meta: format!("max {CREDIT_MAX} h / month") },
                    ],
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// The report form
// ---------------------------------------------------------------------------------------------

/// Enter or change one person's report for one month. Mount it with a key per person and month.
#[component]
pub(super) fn ReportDialog(person: u32, month: Ym, onclose: EventHandler<()>) -> Element {
    let mut state = use_app_state();
    let toaster = use_toaster();
    let prefs = state.preferences();
    let dir = state.directory();
    let existing = dir.report(person, month).cloned();
    let planned = dir.is_aux(person, month);
    let mut draft = use_signal(|| {
        let mut d = existing.as_ref().map(ReportDraft::from_report).unwrap_or_default();
        d.auxiliary = planned;
        d
    });
    let mut errors = use_signal(Vec::<(&'static str, String)>::new);
    let Some(p) = dir.person(person).cloned() else { return rsx! {} };

    let d = draft();
    let err = move |f: &str| errors.read().iter().find(|(k, _)| *k == f).map(|(_, m)| m.clone());
    let can_aux = p.publisher_type == PublisherType::BaptisedPublisher;
    let kind = Kind::of(p.publisher_type, can_aux && d.auxiliary);
    let goal = match kind {
        Kind::AuxiliaryMonth | Kind::ContinuousAuxiliary => Some(dir.aux_goal(month)),
        _ => None,
    };
    let n = |s: &str| s.trim().parse::<u32>().unwrap_or(0);
    let total = n(&d.hours) + if kind.has_credit() { n(&d.credit) } else { 0 };
    let description = match (kind, goal) {
        (Kind::Pioneer, _) => format!("{} · goal {YEARLY_GOAL} h per service year", p.publisher_type.label()),
        (_, Some(g)) => format!("{} · goal {g} h this month", kind_label(kind)),
        _ => p.publisher_type.label().to_string(),
    };
    let title = format!("{} · {}", p.display_name(&prefs), month_label(month));

    let save = move |_| {
        let d = draft();
        match state.update_directory(|dir| dir.save_report(person, month, &d)) {
            Ok(()) => {
                toaster.success("Report saved.");
                onclose.call(());
            }
            Err(e) => errors.set(e),
        }
    };

    rsx! {
        Dialog {
            open: true,
            onclose: move |_| onclose.call(()),
            title,
            description,
            actions: rsx! {
                ActionButton { action: Action::Cancel, onclick: move |_| onclose.call(()) }
                ActionButton { action: Action::Save, onclick: save }
            },
            Stack {
                if can_aux {
                    Switch {
                        checked: d.auxiliary,
                        label: "Auxiliary pioneer this month",
                        description: format!("Goal {} h in {}.", dir.aux_goal(month), month_label(month)),
                        onchange: move |v| draft.write().auxiliary = v,
                    }
                }
                if kind.reports_hours() {
                    Grid { cols: 2,
                        Field { label: "Hours", required: true, error: err("hours"),
                            TextInput { kind: InputKind::Number, value: d.hours.clone(), invalid: err("hours").is_some(), oninput: move |v| draft.write().hours = v }
                        }
                        if kind.has_credit() {
                            Field { label: "Credit hours", error: err("credit"), help: format!("At most {CREDIT_MAX}. They count towards the goal."),
                                TextInput { kind: InputKind::Number, value: d.credit.clone(), invalid: err("credit").is_some(), oninput: move |v| draft.write().credit = v }
                            }
                        }
                    }
                    if let Some(g) = goal {
                        if total >= g {
                            Badge { tone: Tone::Success, icon: Icon::Check, "Total {total} h · goal of {g} h reached" }
                        } else {
                            Badge { tone: Tone::Warning, "Total {total} h · {g - total} h to the goal of {g} h" }
                        }
                    } else {
                        Text { tone: TextTone::Small, "Total: {total} h" }
                    }
                } else {
                    Field { label: "Took part in the ministry", required: true, error: err("took_part"),
                        Segmented {
                            value: d.took_part.clone(),
                            options: vec![
                                Segment { value: "yes".into(), label: "Yes".into(), icon: Some(Icon::Check) },
                                Segment { value: "no".into(), label: "No".into(), icon: Some(Icon::Close) },
                            ],
                            onchange: move |v: String| draft.write().took_part = v,
                        }
                    }
                }
                Field { label: "Bible studies", required: true, error: err("studies"),
                    TextInput { kind: InputKind::Number, value: d.studies.clone(), invalid: err("studies").is_some(), oninput: move |v| draft.write().studies = v }
                }
                Field { label: "Notes", error: err("notes"), help: "Anything to know about this month.",
                    TextArea { value: d.notes.clone(), rows: 3, invalid: err("notes").is_some(), oninput: move |v| draft.write().notes = v }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Person page card and dashboard
// ---------------------------------------------------------------------------------------------

/// The reports of one person: last 6 months, goal, recent reports, and the report to enter now.
#[component]
pub(super) fn PersonReportsCard(person: u32) -> Element {
    let state = use_app_state();
    let navigator = navigator();
    let dir = state.directory();
    let mut editing = use_signal(|| None::<Ym>);
    let mut show_all = use_signal(|| false);
    let Some(p) = dir.person(person).cloned() else { return rsx! {} };
    let now = reporting_month();
    let months: Vec<MonthMark> = (0..6)
        .map(|i| reports::shift(now, i - 5))
        .zip(dir.last_six(person))
        .map(|(m, done)| MonthMark { label: short_month(m), done })
        .collect();
    let (tone, label) = match dir.activity(&p) {
        Some(Activity::Active) => (Tone::Success, "Active"),
        Some(Activity::Irregular) => (Tone::Warning, "Irregular"),
        Some(Activity::Inactive) => (Tone::Error, "Inactive"),
        None => (Tone::Neutral, "No reports yet"),
    };
    let goal = dir.goal_progress(person, service_year(now));
    // Every month from the first report (or the last 6 months) to the reporting month, newest
    // first. Each one opens the report form, also months without a report.
    let first = dir.reports.iter().filter(|r| r.person == person).map(|r| r.month).min().unwrap_or(reports::shift(now, -5)).min(now);
    let mut all_months: Vec<Ym> = Vec::new();
    let mut m = now;
    while m >= first {
        all_months.push(m);
        m = reports::shift(m, -1);
    }
    let total_months = all_months.len();
    let toggle_text = if show_all() { "Show the last 6 months".to_string() } else { format!("Show all {total_months} months") };
    let shown_months: Vec<Ym> = if show_all() { all_months } else { all_months.into_iter().take(6).collect() };
    let entries: Vec<ListEntry> = shown_months
        .iter()
        .map(|m| match dir.report(person, *m) {
            Some(r) => ListEntry {
                icon: if r.auxiliary { Icon::Privilege } else if r.participated() { Icon::Check } else { Icon::Close },
                title: month_label(*m),
                subtitle: summary(r),
                meta: if r.auxiliary { "Aux. pioneer".into() } else { String::new() },
            },
            None => {
                let (icon, text) = match dir.report_status(person, *m) {
                    ReportStatus::Late => (Icon::Error, "Late"),
                    ReportStatus::Missing => (Icon::Warning, "Missing"),
                    _ => (Icon::Clock, "Not in yet"),
                };
                ListEntry { icon, title: month_label(*m), subtitle: format!("{text} · tap to enter"), meta: String::new() }
            }
        })
        .collect();
    let mut planning = use_signal(|| false);
    let can_plan = p.publisher_type == PublisherType::BaptisedPublisher;
    let planned: Vec<String> = dir.planned_aux(person).into_iter().map(month_label).collect();
    let planned_text = if planned.is_empty() { "No auxiliary pioneering planned.".to_string() } else { format!("Auxiliary pioneer in {}.", planned.join(", ")) };
    rsx! {
        Card { title: "Field service reports", icon: Icon::Report, actions: rsx! { Badge { tone, "{label}" } },
            MonthStrip { months }
            if let Some(g) = goal {
                {
                    let (note, tone) = goal_note(&g);
                    rsx! {
                        ProgressRow {
                            label: format!("Service year {}", service_year_label(service_year(now))),
                            detail: format!("{} / {} h", g.hours, g.goal),
                            value: f64::from(g.hours),
                            max: f64::from(g.goal),
                            marker: (g.kind == Kind::Pioneer).then_some(f64::from(g.expected)),
                            note,
                            tone,
                        }
                    }
                }
            }
            ItemList {
                items: entries,
                on_select: move |i: usize| {
                    if let Some(m) = shown_months.get(i) {
                        editing.set(Some(*m));
                    }
                },
            }
            if total_months > 6 {
                TextLink { onclick: move |_| show_all.set(!show_all()), icon: Icon::ChevronDown, "{toggle_text}" }
            }
            if can_plan {
                Text { tone: TextTone::Small, "{planned_text}" }
            }
            Cluster { gap: Gap::Sm,
                if can_plan {
                    ActionButton { action: Action::Edit, variant: ButtonVariant::Secondary, label: "Auxiliary pioneering", onclick: move |_| planning.set(true) }
                }
                ActionButton { action: Action::View, label: "All reports", variant: ButtonVariant::Ghost, onclick: move |_| { navigator.push(Route::ReportsPage {}); } }
            }
        }
        if let Some(month) = editing() {
            ReportDialog { key: "{person}-{month.0}-{month.1}", person, month, onclose: move |_| editing.set(None) }
        }
        if planning() {
            AuxPlanDialog { person, onclose: move |_| planning.set(false) }
        }
    }
}

/// Plan monthly auxiliary pioneering: the reporting month and the next 12 months.
/// Changes apply at once, like member changes on the group page.
#[component]
fn AuxPlanDialog(person: u32, onclose: EventHandler<()>) -> Element {
    let mut state = use_app_state();
    let prefs = state.preferences();
    let dir = state.directory();
    let name = dir.person(person).map(|p| p.display_name(&prefs)).unwrap_or_default();
    let months: Vec<Ym> = (0..13).map(|i| reports::shift(reporting_month(), i)).collect();
    rsx! {
        Dialog {
            open: true,
            onclose: move |_| onclose.call(()),
            title: "Auxiliary pioneering · {name}",
            description: "Choose the months, ahead of time or once the month has started. The goal of each month is shown.",
            actions: rsx! { ActionButton { action: Action::Done, onclick: move |_| onclose.call(()) } },
            Cluster { gap: Gap::Sm,
                for m in months {
                    ChipToggle {
                        key: "{m.0}-{m.1}",
                        label: format!("{} {} · {} h", short_month(m), m.0, dir.aux_goal(m)),
                        selected: dir.is_aux(person, m),
                        onclick: move |_| {
                            let on = !state.directory().is_aux(person, m);
                            state.update_directory(|d| d.set_aux_plan(person, m, on));
                        },
                    }
                }
            }
        }
    }
}

/// Dashboard: reports of the reporting month.
pub(super) fn reports_widget_values(dir: &Directory) -> (String, String, String, bool) {
    let ym = reporting_month();
    let s = dir.month_summary(ym);
    let missing = s.expected.saturating_sub(s.received);
    let late = dir.reporters().iter().filter(|p| dir.report_status(p.id, ym) == ReportStatus::Late).count();
    (
        format!("Reports for {}", theo_ui::month_name(ym.1)),
        format!("{} / {}", s.received, s.expected),
        if missing == 0 { "All in".into() } else if late > 0 { format!("{late} late") } else { format!("{missing} missing") },
        missing == 0,
    )
}

/// Dashboard: publishers and how many need attention.
pub(super) fn publishers_widget_values(dir: &Directory) -> (String, String) {
    let reporters = dir.reporters();
    let attention = reporters.iter().filter(|p| matches!(dir.activity(p), Some(Activity::Irregular | Activity::Inactive))).count();
    (reporters.len().to_string(), format!("{attention} irregular or inactive"))
}
