//! *Me › My ministry* (`/app/me/ministry`): log each day's time, credit hours and Bible studies on
//! a calendar, send the month as your field service report, and see the months before.
//! Rules: `docs/model/m10-field-service-reports.md`, "My ministry".

use crate::ministry::{hm, hm_long, this_month};
use crate::people::Directory;
use crate::reports::{month_label, reporting_month, shift, Kind, Report, Ym, YEARLY_GOAL};
use crate::state::use_app_state;
use dioxus::prelude::*;
use theo_ui::*;
use theo_ui::Action; // explicit: dioxus::prelude also exports an `Action`

fn studies_text(n: u32) -> String {
    match n {
        0 => String::new(),
        1 => "1 study".into(),
        n => format!("{n} studies"),
    }
}

/// "12 h, 2 Bible studies" or "Took part, 1 Bible study".
pub(super) fn report_text(r: &Report) -> String {
    let studies = match r.studies {
        1 => "1 Bible study".to_string(),
        n => format!("{n} Bible studies"),
    };
    match r.hours {
        Some(h) => {
            let credit = r.credit.map(|c| format!(" + {c} credit")).unwrap_or_default();
            format!("{h} h{credit}, {studies}")
        }
        None if r.took_part => format!("Took part, {studies}"),
        None => "Did not take part".into(),
    }
}

/// The goal of a month for the kind of report (0: no goal).
pub(super) fn month_goal(dir: &Directory, kind: Kind, ym: Ym) -> u32 {
    match kind {
        Kind::Pioneer => YEARLY_GOAL / 12,
        Kind::AuxiliaryMonth | Kind::ContinuousAuxiliary => dir.aux_goal(ym),
        Kind::Publisher => 0,
    }
}

fn days_in_month((y, m): Ym) -> u32 {
    let (ny, nm) = if m == 12 { (y + 1, 1) } else { (y, m + 1) };
    let a = crate::prefs::days_from_ymd((y, m, 1));
    let b = crate::prefs::days_from_ymd((ny, nm, 1));
    (b - a) as u32
}

/// `/app/me/ministry`
#[component]
pub fn MyMinistryPage() -> Element {
    let state = use_app_state();
    let navigator = navigator();
    let prefs = state.preferences();
    let mut month = use_signal(this_month);
    let mut editing = use_signal(|| None::<u32>);
    let mut sending = use_signal(|| false);
    let mut goal_dialog = use_signal(|| false);
    let back = ("Me".to_string(), EventHandler::new(move |_| { navigator.push(crate::Route::Me {}); }));
    let Some(me) = state.me() else {
        return rsx! {
            Page { title: "My ministry", back,
                EmptyState { icon: Icon::User, title: "Your account is not linked to a person", description: "Ask an administrator to link your account to your person in Users." }
            }
        };
    };
    let dir = state.directory();
    let ym = month();
    let Some(kind) = dir.my_kind(me, ym) else {
        return rsx! {
            Page { title: "My ministry", back,
                EmptyState { icon: Icon::Footprints, title: "Only publishers report", description: "Bible students and visitors do not send a field service report." }
            }
        };
    };
    let log = dir.month_log(me, ym);
    let report = dir.report(me, ym).cloned();
    let changed = dir.log_changed_since_sent(me, ym);
    let (ty, tm, td) = crate::prefs::today();
    let current = ym == (ty, tm);
    let cells: Vec<DayCell> = dir
        .ministry_days(me, ym)
        .iter()
        .filter_map(|d| {
            let day = crate::people::parse_date(&d.date)?.2;
            let mut detail = studies_text(d.studies);
            if d.credit_minutes > 0 {
                if !detail.is_empty() {
                    detail.push_str(" · ");
                }
                detail.push_str(&format!("+{} credit", hm(d.credit_minutes)));
            }
            let value = if d.minutes > 0 { hm(d.minutes) } else if d.studies > 0 || d.credit_minutes > 0 { "✓".to_string() } else { "•".to_string() };
            Some(DayCell { day, value, detail })
        })
        .collect();
    let goal = month_goal(&dir, kind, ym);
    let personal = dir.personal_goal(me);
    let done = log.hours() + log.credit_hours();
    let left = days_in_month(ym).saturating_sub(td);
    let goal_note = move |g: u32| {
        if done >= g {
            "Reached".to_string()
        } else if current {
            format!("{} h to go · {left} days left", g - done)
        } else {
            format!("{} h short", g - done)
        }
    };
    let history: Vec<Ym> = dir.my_ministry_months(me).into_iter().filter(|m| *m != ym).take(12).collect();
    let status: Element = match (&report, changed) {
        (Some(_), true) => rsx! {
            Alert { tone: Tone::Warning, title: "Changed since you sent it", "Your log of {month_label(ym)} no longer matches the report. Send it again to update it." }
        },
        (Some(r), false) => rsx! {
            Alert { tone: Tone::Success, title: "{month_label(ym)} is sent", "{report_text(r)}." }
        },
        (None, _) if current => rsx! {
            Alert { tone: Tone::Info, title: "{month_label(ym)} is in progress", "Log your ministry each day. At the end of the month, press Send report: the totals become your report." }
        },
        (None, _) => rsx! {
            Alert { tone: Tone::Warning, title: "{month_label(ym)} is not sent yet", "Check your days and press Send report." }
        },
    };
    let entries: Vec<ListEntry> = history
        .iter()
        .map(|m| {
            let l = dir.month_log(me, *m);
            let r = dir.report(me, *m);
            let logged = if l.days > 0 { format!("Logged {}", hm_long(l.minutes)) } else { "Nothing logged".into() };
            ListEntry {
                icon: if r.is_some() { Icon::Check } else { Icon::Clock },
                title: month_label(*m),
                subtitle: match r {
                    Some(r) => format!("{logged} · Sent: {}", report_text(r)),
                    None => format!("{logged} · Not sent"),
                },
                meta: if r.is_some() { "Sent".into() } else if *m <= reporting_month() { "Not sent".into() } else { String::new() },
            }
        })
        .collect();
    let history_months = history.clone();

    rsx! {
        Page {
            title: "My ministry",
            subtitle: "Log each day, then send the month as your field service report",
            back,
            actions: rsx! {
                ActionButton { action: Action::Settings, variant: ButtonVariant::Secondary, label: if personal.is_some() { "My goal" } else { "Set a goal" }, onclick: move |_| goal_dialog.set(true) }
                ActionButton {
                    action: Action::Send,
                    label: if report.is_some() && !changed { "Send again".to_string() } else { "Send report".to_string() },
                    onclick: move |_| sending.set(true),
                }
            },
            StickyBar {
                MonthPicker { value: ym, min: shift(this_month(), -24), max: this_month(), today: this_month(), full_width: true, onchange: move |m| month.set(m) }
            }
            {status}
            Grid { cols: 4, gap: Gap::Sm, compact: true,
                Card { StatTile { label: "Time", value: if log.minutes > 0 { hm_long(log.minutes) } else { "0 h".into() }, icon: Icon::Clock, delta: format!("{} days", log.days) } }
                if kind.has_credit() {
                    Card { StatTile { label: "Credit", value: if log.credit_minutes > 0 { hm_long(log.credit_minutes) } else { "0 h".into() }, icon: Icon::Sparkles, delta: "for the report".to_string() } }
                }
                Card { StatTile { label: "Bible studies", value: log.week_studies.to_string(), icon: Icon::BookOpen, delta: "most in one week".to_string() } }
                if goal > 0 {
                    Card {
                        ProgressRow {
                            label: if kind == Kind::Pioneer { "Monthly average".to_string() } else { "Pioneer goal".to_string() },
                            detail: format!("{done} / {goal} h"),
                            value: done as f64,
                            max: goal as f64,
                            note: goal_note(goal),
                            tone: if done >= goal { Tone::Success } else { Tone::Neutral },
                        }
                    }
                }
                if let Some(g) = personal {
                    Card {
                        ProgressRow {
                            label: "My goal",
                            detail: format!("{done} / {g} h"),
                            value: done as f64,
                            max: g.max(1) as f64,
                            note: goal_note(g),
                            tone: if done >= g { Tone::Success } else { Tone::Neutral },
                        }
                    }
                }
            }
            Card { title: "Days", icon: Icon::Calendar,
                Text { tone: TextTone::Small, "Tap a day to log your time, credit hours and Bible studies." }
                DayGrid {
                    month: ym,
                    today: (ty, tm, td),
                    cells,
                    week_starts_sunday: prefs.week_start == crate::prefs::WeekStart::Sunday,
                    last_day: Some((ty, tm, td)),
                    on_day: move |d| editing.set(Some(d)),
                }
            }
            Card { title: "Earlier months", icon: Icon::History,
                if entries.is_empty() {
                    Text { tone: TextTone::Muted, "Nothing yet." }
                } else {
                    ItemList {
                        items: entries,
                        on_select: move |i: usize| {
                            if let Some(m) = history_months.get(i) {
                                month.set(*m);
                            }
                        },
                    }
                }
            }
        }
        if let Some(day) = editing() {
            DayDialog { key: "{ym:?}-{day}", person: me, date: format!("{:04}-{:02}-{day:02}", ym.0, ym.1), credit: kind.has_credit(), onclose: move |_| editing.set(None) }
        }
        if sending() {
            SendDialog { person: me, month: ym, onclose: move |_| sending.set(false) }
        }
        if goal_dialog() {
            GoalDialog { person: me, onclose: move |_| goal_dialog.set(false) }
        }
    }
}

#[component]
fn DayDialog(person: u32, date: String, credit: bool, onclose: EventHandler<()>) -> Element {
    let mut state = use_app_state();
    let toaster = use_toaster();
    let prefs = state.preferences();
    let existing = state.directory().ministry_day(person, &date).cloned();
    let m0 = existing.as_ref().map(|d| d.minutes).unwrap_or(0);
    let c0 = existing.as_ref().map(|d| d.credit_minutes).unwrap_or(0);
    let mut hours = use_signal(|| (m0 / 60).to_string());
    let mut minutes = use_signal(|| (m0 % 60).to_string());
    let mut credit_h = use_signal(|| (c0 / 60).to_string());
    let mut credit_m = use_signal(|| (c0 % 60).to_string());
    let mut studies = use_signal(|| existing.as_ref().map(|d| d.studies).unwrap_or(0).to_string());
    let mut note = use_signal(|| existing.as_ref().and_then(|d| d.note.clone()).unwrap_or_default());
    let mut error = use_signal(|| None::<String>);
    let minute_options: Vec<SelectOption> = (0..60).step_by(5).map(|m| SelectOption::new(m.to_string(), format!("{m:02} min"))).collect();
    let title = crate::people::parse_date(&date).map(|d| prefs.format_date(d)).unwrap_or_else(|| date.clone());
    let n = |s: String| s.trim().parse::<u32>().unwrap_or(0);
    let d1 = date.clone();
    let save = move |_| {
        let total = n(hours()) * 60 + n(minutes());
        let total_credit = n(credit_h()) * 60 + n(credit_m());
        match state.update_directory(|d| d.save_ministry_day(person, &d1, total, total_credit, n(studies()), &note())) {
            Ok(()) => {
                toaster.success("Day saved.");
                onclose.call(());
            }
            Err(e) => error.set(Some(e)),
        }
    };
    let d2 = date.clone();
    let clear = move |_| {
        let _ = state.update_directory(|d| d.save_ministry_day(person, &d2, 0, 0, 0, ""));
        toaster.success("Day cleared.");
        onclose.call(());
    };
    rsx! {
        Dialog {
            open: true,
            onclose: move |_| onclose.call(()),
            title,
            description: "Your ministry of this day. Only you see this log.",
            actions: rsx! {
                EditActions {
                    on_save: save,
                    on_cancel: move |_| onclose.call(()),
                    on_delete: existing.is_some().then_some(EventHandler::new(clear)),
                }
            },
            Stack {
                if let Some(e) = error() {
                    Alert { tone: Tone::Error, "{e}" }
                }
                Grid { cols: 2,
                    Field { label: "Hours",
                        TextInput { kind: InputKind::Count, value: hours(), oninput: move |v| hours.set(v) }
                    }
                    Field { label: "Minutes",
                        Select { value: minutes(), options: minute_options.clone(), onchange: move |v| minutes.set(v) }
                    }
                }
                if credit {
                    Grid { cols: 2,
                        Field { label: "Credit hours", help: "For example a theocratic project, as the elders said.",
                            TextInput { kind: InputKind::Count, value: credit_h(), oninput: move |v| credit_h.set(v) }
                        }
                        Field { label: "Credit minutes",
                            Select { value: credit_m(), options: minute_options, onchange: move |v| credit_m.set(v) }
                        }
                    }
                }
                Field { label: "Bible studies conducted",
                    TextInput { kind: InputKind::Count, value: studies(), oninput: move |v| studies.set(v) }
                }
                Field { label: "Note", help: "Only for you, for example who you visited.",
                    TextInput { value: note(), oninput: move |v| note.set(v) }
                }
            }
        }
    }
}

#[component]
fn SendDialog(person: u32, month: Ym, onclose: EventHandler<()>) -> Element {
    let mut state = use_app_state();
    let toaster = use_toaster();
    let dir = state.directory();
    let log = dir.month_log(person, month);
    let kind = dir.my_kind(person, month).unwrap_or(Kind::Publisher);
    let sent = dir.report(person, month).map(|r| r.studies);
    let mut studies = use_signal(|| sent.unwrap_or(log.week_studies).to_string());
    let mut error = use_signal(|| None::<String>);
    let (y, m, d) = crate::prefs::today();
    let early = month == (y, m) && d < days_in_month(month);
    let left_over = dir.leftover_minutes(person, month);
    let next = crate::reports::shift(month, 1);
    let took_part = log.minutes > 0 || log.credit_minutes > 0 || studies().trim().parse::<u32>().unwrap_or(0) > 0;
    let send = move |_| {
        let n = studies().trim().parse::<u32>().unwrap_or(0);
        match state.update_directory(|d| d.send_my_report(person, month, n)) {
            Ok(()) => {
                toaster.success(format!("Report for {} sent.", month_label(month)));
                onclose.call(());
            }
            Err(e) => error.set(Some(e)),
        }
    };
    rsx! {
        Dialog {
            open: true,
            onclose: move |_| onclose.call(()),
            title: format!("Send {}", month_label(month)),
            description: "Your report is made from your log. The secretary sees it at once; sending again replaces it.",
            actions: rsx! {
                ActionButton { action: Action::Cancel, onclick: move |_| onclose.call(()) }
                ActionButton { action: Action::Send, label: "Send report", onclick: send }
            },
            Stack {
                if let Some(e) = error() {
                    Alert { tone: Tone::Error, "{e}" }
                }
                if early {
                    Alert { tone: Tone::Info, "The month is not over yet. You can send now and send again later: the newest report counts." }
                }
                StatList {
                    items: {
                        let mut items = Vec::new();
                        if kind.reports_hours() {
                            items.push(StatItem::new("Hours", log.hours().to_string()));
                            if kind.has_credit() {
                                items.push(StatItem::new("Credit hours", log.credit_hours().min(crate::reports::CREDIT_MAX).to_string()));
                            }
                        } else {
                            items.push(StatItem::new("Took part in the ministry", if took_part { "Yes" } else { "No" }).with_tone(if took_part { Tone::Success } else { Tone::Warning }));
                        }
                        items.push(StatItem::new("Days logged", log.days.to_string()));
                        items
                    },
                }
                if left_over > 0 {
                    Cluster { justify: Justify::Between,
                        Text { tone: TextTone::Small, "Reports use whole hours: {left_over} min are left over." }
                        if !early {
                            ActionButton {
                                action: Action::Next,
                                variant: ButtonVariant::Ghost,
                                label: format!("Carry {left_over} min to {}", theo_ui::month_name(next.1)),
                                onclick: move |_| match state.update_directory(|d| d.carry_over(person, month)) {
                                    Ok((n, m)) => toaster.success(format!("{m} min added to 1 {}.", theo_ui::month_name(n.1))),
                                    Err(e) => toaster.error(e),
                                },
                            }
                        }
                    }
                }
                Field { label: "Different Bible studies this month", help: "The number of different people you studied with. It starts with the most you had in one week.",
                    TextInput { kind: InputKind::Count, value: studies(), oninput: move |v| studies.set(v) }
                }
            }
        }
    }
}

/// A personal goal of hours per month (also for publishers who are not pioneering).
#[component]
fn GoalDialog(person: u32, onclose: EventHandler<()>) -> Element {
    let mut state = use_app_state();
    let toaster = use_toaster();
    let current = state.directory().personal_goal(person);
    let mut hours = use_signal(|| current.map(|h| h.to_string()).unwrap_or_default());
    rsx! {
        Dialog {
            open: true,
            onclose: move |_| onclose.call(()),
            title: "My goal",
            description: "Hours per month you would like to reach. Only you see it; it does not change your report.",
            actions: rsx! {
                if current.is_some() {
                    ActionButton {
                        action: Action::Remove,
                        onclick: move |_| {
                            state.update_directory(|d| d.set_personal_goal(person, None));
                            toaster.success("Goal removed.");
                            onclose.call(());
                        },
                    }
                }
                ActionButton { action: Action::Cancel, onclick: move |_| onclose.call(()) }
                ActionButton {
                    action: Action::Save,
                    onclick: move |_| {
                        let h = hours().trim().parse::<u32>().ok();
                        state.update_directory(|d| d.set_personal_goal(person, h));
                        toaster.success(if h.unwrap_or(0) > 0 { "Goal saved." } else { "Goal removed." });
                        onclose.call(());
                    },
                }
            },
            Field { label: "Hours per month", help: "For example 10. Shown on this page and in the My goal widget.",
                TextInput { kind: InputKind::Count, value: hours(), placeholder: "10", oninput: move |v| hours.set(v) }
            }
        }
    }
}
