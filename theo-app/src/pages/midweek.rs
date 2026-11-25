//! *Meetings › Midweek meeting*: the plan of each week (from the workbook, read on this device),
//! assigning each part with the best candidates first, the auxiliary classroom, filling, sending,
//! and statistics. Rules: `docs/model/m03-midweek-meeting.md`.

use crate::midweek::{meeting_date, monday_of, weekday_offset, MidweekWeek, MwCandidate, MwFit, MwPart, MwRole, PartDraft, PartKind, SectionKind};
use crate::people::{parse_date, today_iso, Directory};
use crate::prefs::Preferences;
use crate::reports::{month_label, Ym};
use crate::service_meetings::SendStatus;
use crate::state::{use_app_state, AppState};
use crate::workbook::{issues_for, jw_language, Workbook, WeekContent};
use dioxus::prelude::*;
use std::collections::BTreeSet;
use theo_ui::*;
use theo_ui::Action; // explicit: dioxus::prelude also exports an `Action`

const PAGE: &str = "meetings/midweek";

fn this_month() -> Ym {
    let (y, m, _) = crate::prefs::today();
    (y, m)
}

fn add_days(iso: &str, n: i64) -> String {
    crate::events::add_days(iso, n).unwrap_or_else(|| iso.to_string())
}

/// Days after Monday of the congregation's midweek meeting.
fn weekday(state: &AppState) -> i64 {
    state.active_congregation().map(|c| weekday_offset(&c.midweek.day)).unwrap_or(3)
}

fn language(state: &AppState) -> &'static str {
    jw_language(&state.active_congregation().map(|c| c.language).unwrap_or_default())
}

/// The week of the next meeting (this week's, until its day has passed).
fn next_monday(weekday: i64) -> String {
    let today = today_iso();
    let m = monday_of(&today);
    if meeting_date(&m, weekday) < today { add_days(&m, 7) } else { m }
}

fn day_title(prefs: &Preferences, iso: &str) -> String {
    let names = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];
    match parse_date(iso) {
        Some(d) => {
            let wd = (crate::prefs::days_from_ymd(d) + 3).rem_euclid(7) as usize;
            format!("{} {}", names[wd], prefs.format_date(d))
        }
        None => iso.to_string(),
    }
}

/// The Mondays of the meetings that fall in a month.
fn mondays_in(ym: Ym, weekday: i64) -> Vec<String> {
    let first = format!("{:04}-{:02}-01", ym.0, ym.1);
    let mut m = monday_of(&first);
    let mut out = Vec::new();
    for _ in 0..7 {
        let date = meeting_date(&m, weekday);
        if date.get(..7) == first.get(..7) {
            out.push(m.clone());
        } else if !out.is_empty() {
            break;
        }
        m = add_days(&m, 7);
    }
    out
}

/// The workbook content of a week on this device, if any.
fn content_of(state: &AppState, w: &MidweekWeek) -> Option<WeekContent> {
    let s = w.source.as_ref()?;
    state.workbook(&s.issue, &s.lang)?.week(&w.monday).cloned()
}

/// Title and description of a part: the planner's, else the workbook's, else the kind.
fn part_text(p: &MwPart, content: Option<&WeekContent>) -> (String, String) {
    let c = content.and_then(|c| p.source.and_then(|n| c.parts.iter().find(|x| x.number == n)));
    let title = p.title.clone().or_else(|| c.map(|c| c.title.clone())).unwrap_or_else(|| p.kind.label().to_string());
    let description = p.description.clone().or_else(|| c.map(|c| c.description.clone())).unwrap_or_default();
    (title, description)
}

fn name(dir: &Directory, prefs: &Preferences, id: u32) -> String {
    dir.person(id).map(|p| p.display_name(prefs)).unwrap_or_else(|| "Unknown".into())
}

fn status_tag(s: SendStatus) -> Option<Tag> {
    match s {
        SendStatus::Sent => Some(Tag::new("Sent", Tone::Success).with_icon(Icon::Check)),
        SendStatus::NotSent => Some(Tag::new("Not sent", Tone::Warning).with_icon(Icon::Clock)),
        SendStatus::Changed => Some(Tag::new("Changed · not sent", Tone::Warning).with_icon(Icon::Edit)),
        SendStatus::Removed => None,
    }
}

/// Downloads the workbook for a week (trying the issues that may hold it), keeps it on this
/// device and, for planners, creates or refreshes the plans of its weeks.
async fn pull(mut state: AppState, monday: String, planner: bool, lang: &'static str) -> Result<usize, crate::workbook::DownloadError> {
    let mut last = None;
    for issue in issues_for(&monday) {
        match crate::workbook::download(&issue, lang).await {
            Ok(wb) => {
                let has = wb.week(&monday).is_some();
                let n = save_workbook(&mut state, wb, planner);
                if has {
                    return Ok(n);
                }
            }
            Err(e) => {
                if e.epub_url.is_some() {
                    return Err(e);
                }
                last = Some(e);
            }
        }
    }
    Err(last.unwrap_or(crate::workbook::DownloadError { message: "This week is not in the workbooks yet.".into(), epub_url: None }))
}

/// Keeps a workbook and (planners) plans its weeks. Returns the number of weeks.
fn save_workbook(state: &mut AppState, wb: Workbook, planner: bool) -> usize {
    let n = wb.weeks.len();
    if planner {
        let weeks = wb.weeks.clone();
        let (issue, lang) = (wb.issue.clone(), wb.lang.clone());
        state.update_directory(|d| {
            for w in &weeks {
                d.import_midweek_week(w, Some(crate::midweek::WeekSource { issue: issue.clone(), lang: lang.clone(), doc: w.doc.clone() }));
            }
        });
    }
    state.store_workbook(wb);
    n
}

/// `/app/meetings/midweek`
#[component]
pub fn MidweekPage() -> Element {
    let mut state = use_app_state();
    let toaster = use_toaster();
    if !state.can_read(PAGE) {
        return rsx! {
            Page { title: "Midweek meeting",
                EmptyState { icon: Icon::Lock, title: "You do not have access to the midweek meeting", description: "Your own parts are in My assignments on the home page." }
            }
        };
    }
    let planner = state.can_write(PAGE);
    let prefs = state.preferences();
    let dir = state.directory();
    let wd = weekday(&state);
    let lang = language(&state);
    let congregation = state.active_congregation().map(|c| c.name).unwrap_or_default();
    let tab = (state.mw_tab)();
    let monday = (state.mw_monday)().unwrap_or_else(|| next_monday(wd));
    let month = (state.mw_month)().unwrap_or_else(this_month);
    let mut downloading = use_signal(|| false);
    // (week, the official EPUB address, message) when the browser could not download it.
    let mut need_file = use_signal(|| None::<(String, String, String)>);
    let mut tried = use_signal(BTreeSet::<String>::new);
    let mut confirm_send = use_signal(|| false);
    let mut settings = use_signal(|| false);
    let mut exporting = use_signal(|| false);

    // The workbook is fetched in the background when the shown week needs it (once per week and
    // session). On the web the browser may refuse; then the page offers to pick the file.
    let needed = {
        let w = dir.midweek_week(&monday);
        let has_content = w.is_some_and(|w| w.source.is_none() || content_of(&state, w).is_some());
        let cached = issues_for(&monday).iter().any(|i| state.workbook(i, lang).is_some_and(|wb| wb.week(&monday).is_some()));
        // Once per workbook issue and session, and never while a download is running.
        let issue = issues_for(&monday).first().cloned().unwrap_or_default();
        tab == "week" && !(has_content && w.is_some()) && !cached && !downloading() && !tried.read().contains(&issue)
    };
    use_effect(use_reactive!(|needed, monday| {
        if !needed {
            return;
        }
        tried.write().insert(issues_for(&monday).first().cloned().unwrap_or_default());
        downloading.set(true);
        spawn(async move {
            match pull(state, monday.clone(), planner, lang).await {
                Ok(_) => need_file.set(None),
                Err(e) => match e.epub_url {
                    Some(url) => need_file.set(Some((monday.clone(), url, e.message))),
                    None => toaster.show(Tone::Warning, e.message),
                },
            }
            downloading.set(false);
        });
    }));
    // One click: download, read and plan (or refresh) the weeks of the workbook.
    let has_plan = dir.midweek_week(&monday).is_some();
    let pull_now = {
        let monday = monday.clone();
        move |_| {
            let monday = monday.clone();
            downloading.set(true);
            spawn(async move {
                match pull(state, monday.clone(), planner, lang).await {
                    Ok(n) => {
                        need_file.set(None);
                        toaster.success(format!("Meeting plan updated from the workbook ({n} weeks)."));
                    }
                    Err(e) => match e.epub_url {
                        Some(url) => need_file.set(Some((monday.clone(), url, e.message))),
                        None => toaster.error(e.message),
                    },
                }
                downloading.set(false);
            });
        }
    };
    // The file help is only for the week that needs it, while it has no plan.
    let file_help = need_file().filter(|(m, _, _)| *m == monday && !has_plan && tab == "week");

    let scope: Vec<String> = if tab == "month" { mondays_in(month, wd) } else { vec![monday.clone()] };
    let unsent = dir.unsent_midweek(&scope, wd);
    let scope_send = scope.clone();
    let scope_fill = scope.clone();
    let on_fill = move |id: String| {
        let weeks = scope_fill.clone();
        match id.as_str() {
            "fill" => {
                let (filled, left) = state.update_directory(|d| d.autofill_midweek(&weeks, wd));
                if left > 0 {
                    toaster.show(Tone::Warning, format!("{filled} places filled. {left} still empty: nobody fits."));
                } else if filled == 0 {
                    toaster.success("Every place already has someone.");
                } else {
                    toaster.success(format!("{filled} places filled."));
                }
            }
            "clear" => {
                state.update_directory(|d| d.clear_midweek(&weeks));
                toaster.success("Assignments cleared.");
            }
            _ => {}
        }
    };
    let actions = rsx! {
        ActionButton { action: Action::Export, variant: ButtonVariant::Secondary, onclick: move |_| exporting.set(true) }
        if planner {
            ActionButton { action: Action::Settings, variant: ButtonVariant::Secondary, label: "Settings", onclick: move |_| settings.set(true) }
            ActionButton {
                action: Action::Regenerate,
                variant: ButtonVariant::Secondary,
                label: if downloading() { "Getting the plan…" } else if has_plan { "Refresh plan" } else { "Get the plan" },
                loading: downloading(),
                disabled: downloading(),
                onclick: pull_now,
            }
            MenuButton {
                label: "Autofill",
                icon: Icon::Magic,
                items: vec![
                    MenuItem::new("fill", if tab == "month" { "Fill the month" } else { "Fill this week" }, Icon::Magic).with_description("The best candidate for every empty place"),
                    MenuItem::new("clear", if tab == "month" { "Clear the month" } else { "Clear this week" }, Icon::Trash).with_description("Empty every place"),
                ],
                onselect: on_fill,
            }
            ActionButton {
                action: Action::Send,
                label: if unsent.is_empty() { "Send".to_string() } else { format!("Send ({})", unsent.len()) },
                disabled: unsent.is_empty(),
                onclick: move |_| confirm_send.set(true),
            }
        }
    };

    rsx! {
        Page {
            title: "Midweek meeting",
            subtitle: "{congregation} · Our Christian Life and Ministry",
            actions,
            StickyBar {
                Segmented {
                    value: tab.clone(),
                    options: vec![
                        Segment { value: "week".into(), label: "Week".into(), icon: Some(Icon::Calendar) },
                        Segment { value: "month".into(), label: "Month".into(), icon: Some(Icon::Report) },
                        Segment { value: "statistics".into(), label: "Statistics".into(), icon: Some(Icon::Chart) },
                    ],
                    onchange: move |v: String| state.mw_tab.set(v),
                }
                if tab == "week" {
                    StepPicker {
                        label: day_title(&prefs, &meeting_date(&monday, wd)),
                        on_prev: { let m = monday.clone(); move |_| state.mw_monday.set(Some(add_days(&m, -7))) },
                        on_next: { let m = monday.clone(); move |_| state.mw_monday.set(Some(add_days(&m, 7))) },
                        on_today: (monday != next_monday(wd)).then_some(EventHandler::new(move |_| state.mw_monday.set(None))),
                    }
                } else if tab == "month" {
                    MonthPicker { value: month, min: crate::reports::shift(this_month(), -24), max: crate::reports::shift(this_month(), 6), today: this_month(), full_width: true, onchange: move |m| state.mw_month.set(Some(m)) }
                }
            }
            if downloading() {
                Alert { tone: Tone::Info, title: "Getting the meeting plan…", "The workbook is downloaded from jw.org and read on this device." }
            }
            if let Some((_, url, message)) = file_help {
                FileNeeded { url, message, planner, onclose: move |_| need_file.set(None) }
            }
            match tab.as_str() {
                "statistics" => rsx! { StatisticsTab {} },
                "month" => rsx! { MonthTab { month, planner } },
                _ => rsx! { WeekTab { monday: monday.clone(), planner } },
            }
        }
        ConfirmDialog {
            open: confirm_send(),
            title: if tab == "month" { format!("Send {}?", month_label(month)) } else { "Send this week?".to_string() },
            message: "Everyone who can see the midweek meeting sees the assignments as they are now, and only the people whose parts are new, changed or removed get a notice.",
            confirm: Action::Send,
            oncancel: move |_| confirm_send.set(false),
            onconfirm: move |_| {
                let n = state.update_directory(|d| d.send_midweek(&scope_send, wd));
                confirm_send.set(false);
                toaster.success(if n == 1 { "1 week sent.".to_string() } else { format!("{n} weeks sent.") });
            },
        }
        if settings() {
            SettingsDialog { onclose: move |_| settings.set(false) }
        }
        if exporting() {
            ExportDialog { first: if tab == "month" { month } else { meeting_date(&monday, wd).get(..7).and_then(|m| Some((m.get(..4)?.parse().ok()?, m.get(5..7)?.parse().ok()?))).unwrap_or_else(this_month) }, planner, onclose: move |_| exporting.set(false) }
        }
    }
}

/// Shown when the browser cannot download the workbook: the official file, then pick it.
#[component]
fn FileNeeded(url: String, message: String, planner: bool, onclose: EventHandler<()>) -> Element {
    let mut state = use_app_state();
    let toaster = use_toaster();
    let lang = language(&state);
    let monday = (state.mw_monday)().unwrap_or_else(|| next_monday(weekday(&state)));
    rsx! {
        Alert { tone: Tone::Info, title: "Get the workbook in two steps",
            Stack { gap: Gap::Sm,
                Text { "{message} Download the official file from jw.org, then choose it here. It is read on this device only." }
                Cluster { gap: Gap::Sm,
                    LinkButton { href: url.clone(), label: "1. Download from jw.org", icon: Icon::Download }
                    FileButton {
                        label: "2. Choose the file",
                        icon: Icon::Upload,
                        accept: ".epub",
                        on_file: move |(file, bytes): (String, Vec<u8>)| {
                            let (issue, l) = crate::workbook::issue_from_file_name(&file).unwrap_or_else(|| (issues_for(&monday).first().cloned().unwrap_or_default(), lang.to_string()));
                            match crate::workbook::read_epub(&bytes, &issue, &l) {
                                Ok(wb) => {
                                    let n = save_workbook(&mut state, wb, planner);
                                    toaster.success(format!("Workbook read: {n} weeks."));
                                    onclose.call(());
                                }
                                Err(e) => toaster.error(e),
                            }
                        },
                    }
                }
            }
        }
    }
}

#[component]
fn SettingsDialog(onclose: EventHandler<()>) -> Element {
    let mut state = use_app_state();
    let on = state.directory().midweek_settings.aux_classroom;
    rsx! {
        Dialog {
            open: true,
            onclose: move |_| onclose.call(()),
            title: "Midweek meeting settings",
            actions: rsx! { ActionButton { action: Action::Done, onclick: move |_| onclose.call(()) } },
            Switch {
                checked: on,
                label: "Auxiliary classroom",
                description: "A second hall for the Bible reading and the student parts, with its own counselor. New weeks use it; each week can switch it off.",
                onchange: move |v: bool| state.update_directory(|d| d.midweek_settings.aux_classroom = v),
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Week
// ---------------------------------------------------------------------------------------------

/// A place the planner is choosing for.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Choosing(MwRole);

fn slot_views(dir: &Directory, prefs: &Preferences, w: &MidweekWeek, places: &[(String, MwRole)], planner: bool, date: &str) -> Vec<SlotView> {
    let sent = w.sent.as_ref();
    places
        .iter()
        .map(|(label, role)| {
            // Readers see what was sent.
            let person = if planner { w.person_at(*role) } else { sent.and_then(|s| s.places.iter().find(|(k, _)| *k == crate::midweek::role_key(*role)).map(|(_, p)| *p)) };
            let away = person.is_some_and(|p| dir.is_away(p, date));
            SlotView { label: label.clone(), person: person.map(|p| name(dir, prefs, p)), warning: (planner && away).then(|| "Away that day".to_string()) }
        })
        .collect()
}

#[component]
fn WeekTab(monday: String, planner: bool) -> Element {
    let mut state = use_app_state();
    let prefs = state.preferences();
    let dir = state.directory();
    let wd = weekday(&state);
    let date = meeting_date(&monday, wd);
    let mut choosing = use_signal(|| None::<Choosing>);
    let mut editing = use_signal(|| None::<(u32, Option<u32>)>);
    let mut adding_section = use_signal(|| false);
    let mut editing_header = use_signal(|| false);
    let mut confirm_section = use_signal(|| None::<u32>);
    let Some(w) = dir.midweek_week(&monday).cloned() else {
        return rsx! {
            EmptyState {
                icon: Icon::BookOpen,
                title: "No plan for this week yet",
                description: if planner { "The workbook is fetched by itself when it is available. You can also plan the week by hand." } else { "It appears here when the planners have made it." },
            }
            if planner {
                Cluster { justify: Justify::Center,
                    ActionButton { action: Action::Add, variant: ButtonVariant::Secondary, label: "Plan this week by hand", onclick: { let m = monday.clone(); move |_| state.update_directory(|d| d.add_midweek_week(&m)) } }
                }
            }
        };
    };
    let content = content_of(&state, &w);
    let status = w.status(&date);
    if !planner && w.sent.is_none() {
        return rsx! { EmptyState { icon: Icon::Clock, title: "Not sent yet", description: "The assignments of this week appear here when they are sent." } };
    }
    // Events in the week (M-12, decision E1).
    let sunday = add_days(&monday, 6);
    let events: Vec<crate::events::Event> = dir.visible_events(state.me(), state.i_am_admin()).into_iter().filter(|e| e.from <= sunday && e.to >= monday && e.kind != crate::events::EventKind::Custom).cloned().collect();
    let songs: Vec<String> = w.songs.iter().flatten().map(|n| n.to_string()).collect();
    let (filled, places) = w.filled();
    let opening_places = {
        let mut v = vec![("Chairman".to_string(), MwRole::Chairman)];
        if w.aux_on {
            v.push(("Aux. classroom counselor".to_string(), MwRole::AuxCounselor));
        }
        v
    };
    let opening_roles: Vec<MwRole> = opening_places.iter().map(|(_, r)| *r).collect();
    // Parts are numbered in meeting order (also the ones a planner added or moved).
    let numbers: Vec<(u32, usize)> = w.parts().enumerate().map(|(i, p)| (p.id, i + 1)).collect();
    let section_count = w.sections.len();
    let first_living = w.sections.iter().position(|s| s.kind == SectionKind::Living);
    let song_title = |n: Option<u16>| n.map(|n| format!("Song {n}")).unwrap_or_else(|| "Song".to_string());
    // Times from the congregation's start time (Settings › meetings).
    let start = state.active_congregation().and_then(|c| crate::midweek::minutes_of(&c.midweek.time)).unwrap_or(19 * 60);
    let tl = w.timeline(start);
    let at = |m: u32| prefs.format_hhmm(&crate::midweek::hhmm(m));
    let mut tags = Vec::new();
    if planner {
        if let Some(t) = status_tag(status) {
            tags.push(t);
        }
        tags.push(Tag::new(format!("{filled} of {places} places"), if filled == places { Tone::Success } else { Tone::Neutral }).with_icon(Icon::Users));
    }
    if w.source.is_some() && content.is_none() {
        tags.push(Tag::new("Workbook not on this device", Tone::Warning).with_icon(Icon::Download));
    }
    let m = monday.clone();
    rsx! {
        for e in events.iter() {
            Alert {
                key: "{e.id}",
                tone: if matches!(e.kind, crate::events::EventKind::RegionalConvention | crate::events::EventKind::CircuitAssembly) { Tone::Warning } else { Tone::Info },
                title: "{e.title}",
                if matches!(e.kind, crate::events::EventKind::RegionalConvention | crate::events::EventKind::CircuitAssembly) {
                    "There is usually no midweek meeting this week. Switch on \"No meeting this week\"."
                } else {
                    "This event is in the week of the meeting."
                }
            }
        }
        Card {
            Stack { gap: Gap::Sm,
                Cluster { justify: Justify::Between,
                    Stack { gap: Gap::Xs,
                        Heading { level: HeadingLevel::H3, "{day_title(&prefs, &date)} · {at(tl.opening_song)} – {at(tl.end)}" }
                        Text { tone: TextTone::Muted,
                            if w.reading.is_empty() { "Bible reading not set" } else { "{w.reading}" }
                            if !songs.is_empty() { " · Songs {songs.join(\", \")}" }
                        }
                    }
                    if planner {
                        ActionButton { action: Action::Edit, variant: ButtonVariant::Ghost, label: "Reading and songs", onclick: move |_| editing_header.set(true) }
                    }
                }
                Cluster { gap: Gap::Sm,
                    for (i, t) in tags.iter().enumerate() {
                        Badge { key: "{i}", tone: t.tone, icon: t.icon.unwrap_or(Icon::Info), "{t.label}" }
                    }
                }
                if planner {
                    Grid { cols: 2,
                        Switch { checked: w.no_meeting, label: "No meeting this week", description: "For example a convention or an assembly week.", onchange: { let m = m.clone(); move |v: bool| state.update_directory(|d| d.set_midweek_no_meeting(&m, v)) } }
                        if dir.midweek_settings.aux_classroom {
                            Switch { checked: w.aux_on, label: "Auxiliary classroom this week", description: "The Bible reading and student parts are given in both halls.", onchange: { let m = m.clone(); move |v: bool| state.update_directory(|d| d.set_midweek_aux(&m, v)) } }
                        }
                    }
                }
            }
        }
        if w.no_meeting {
            EmptyState { icon: Icon::Calendar, title: "No meeting this week", description: "Nothing to assign." }
        } else {
            MeetingSection { title: "Opening", subtitle: format!("{} – {}", at(tl.opening_song), at(tl.parts.first().map(|x| x.1).unwrap_or(tl.concluding))), icon: Icon::Song,
                AssignmentRow {
                    number: "",
                    icon: Icon::User,
                    title: "Chairman",
                    minutes: at(tl.opening_song),
                    detail: "Presides over the whole meeting.",
                    slots: slot_views(&dir, &prefs, &w, &opening_places, planner, &date),
                    on_slot: if planner { Some(EventHandler::new(move |i: usize| if let Some(r) = opening_roles.get(i) { choosing.set(Some(Choosing(*r))) })) } else { None },
                }
                AssignmentRow { number: "", icon: Icon::Song, title: song_title(w.songs[0]), minutes: format!("{} · {} min", at(tl.opening_song), crate::midweek::OPENING_SONG), slots: Vec::new() }
                AssignmentRow {
                    number: "",
                    icon: Icon::User,
                    title: "Opening prayer",
                    minutes: format!("{} · {} min", at(tl.opening_prayer), crate::midweek::OPENING_PRAYER),
                    slots: slot_views(&dir, &prefs, &w, &[("Prayer".to_string(), MwRole::OpeningPrayer)], planner, &date),
                    on_slot: if planner { Some(EventHandler::new(move |_| choosing.set(Some(Choosing(MwRole::OpeningPrayer))))) } else { None },
                }
                AssignmentRow { number: "", icon: Icon::User, title: "Opening comments", minutes: format!("{} · {} min", at(tl.opening_comments), crate::midweek::OPENING_COMMENTS), detail: "By the chairman.", slots: Vec::new() }
            }
            for (si, s) in w.sections.iter().enumerate() {
                {
                    let section_id = s.id;
                    let title = s.title.clone().unwrap_or_else(|| s.kind.label().to_string());
                    let (icon, tone) = match s.kind {
                        SectionKind::Treasures => (Icon::BookOpen, SectionTone::Teal),
                        SectionKind::Ministry => (Icon::Footprints, SectionTone::Gold),
                        SectionKind::Living => (Icon::Users, SectionTone::Maroon),
                        SectionKind::Other => (Icon::Note, SectionTone::Neutral),
                    };
                    let minutes: u32 = s.parts.iter().map(|p| p.minutes as u32 + if p.kind.has_counsel() { crate::midweek::COUNSEL } else { 0 }).sum::<u32>()
                        + if s.kind == SectionKind::Living && Some(si) == first_living { crate::midweek::MIDDLE_SONG } else { 0 };
                    let span = s.parts.first().and_then(|p| tl.part(p.id)).map(|t| {
                        let begin = if s.kind == SectionKind::Living && Some(si) == first_living { tl.middle_song.unwrap_or(t) } else { t };
                        format!("{} – {} · ", at(begin), at(begin + minutes))
                    });
                    let last_section = si + 1 == section_count;
                    let middle_song = s.kind == SectionKind::Living && Some(si) == first_living;
                    let m_up = monday.clone();
                    let m_down = monday.clone();
                    rsx! {
                        MeetingSection {
                            key: "{section_id}",
                            title,
                            subtitle: format!("{}{minutes} min", span.unwrap_or_default()),
                            icon,
                            tone,
                            actions: if planner {
                                Some(rsx! {
                                    if si > 0 {
                                        IconButton { icon: Icon::MoveUp, label: "Move the section up", onclick: move |_| state.update_directory(|d| d.move_midweek_section(&m_up, section_id, -1)) }
                                    }
                                    if !last_section {
                                        IconButton { icon: Icon::MoveDown, label: "Move the section down", onclick: move |_| state.update_directory(|d| d.move_midweek_section(&m_down, section_id, 1)) }
                                    }
                                })
                            } else {
                                None
                            },
                            if middle_song {
                                AssignmentRow { number: "", icon: Icon::Song, title: song_title(w.songs[1]), minutes: format!("{} · {} min", at(tl.middle_song.unwrap_or(0)), crate::midweek::MIDDLE_SONG), slots: Vec::new() }
                            }
                            if s.parts.is_empty() {
                                Text { tone: TextTone::Muted, "No parts." }
                            }
                            for p in s.parts.iter() {
                                {
                                    let (title, detail) = part_text(p, content.as_ref());
                                    let mut places: Vec<(String, MwRole)> = Vec::new();
                                    let main_label = match p.kind {
                                        PartKind::BibleStudy => "Conductor",
                                        k if k.second().is_some() || k == PartKind::BibleReading || k == PartKind::StudentTalk => "Student",
                                        _ => "Speaker",
                                    };
                                    places.push((main_label.to_string(), MwRole::Main(p.id)));
                                    if let Some(second) = p.kind.second() {
                                        places.push((second.to_string(), MwRole::Assistant(p.id)));
                                    }
                                    if w.aux_on && p.kind.in_aux_classroom() {
                                        places.push(("Aux. classroom".to_string(), MwRole::AuxMain(p.id)));
                                        if p.kind.second().is_some() {
                                            places.push(("Aux. assistant".to_string(), MwRole::AuxAssistant(p.id)));
                                        }
                                    }
                                    let roles: Vec<MwRole> = places.iter().map(|(_, r)| *r).collect();
                                    let pid = p.id;
                                    let number = numbers.iter().find(|(id, _)| *id == pid).map(|(_, n)| n.to_string()).unwrap_or_default();
                                    let first = numbers.first().map(|x| x.0) == Some(pid);
                                    let last = numbers.last().map(|x| x.0) == Some(pid);
                                    let (mu, md) = (monday.clone(), monday.clone());
                                    rsx! {
                                        AssignmentRow {
                                            key: "{pid}",
                                            number,
                                            title,
                                            minutes: match tl.part(pid) {
                                                Some(t) if p.kind.has_counsel() => format!("{} · {} min + {} counsel", at(t), p.minutes, crate::midweek::COUNSEL),
                                                Some(t) => format!("{} · {} min", at(t), p.minutes),
                                                None => format!("{} min", p.minutes),
                                            },
                                            detail,
                                            slots: slot_views(&dir, &prefs, &w, &places, planner, &date),
                                            on_slot: if planner { Some(EventHandler::new(move |i: usize| if let Some(r) = roles.get(i) { choosing.set(Some(Choosing(*r))) })) } else { None },
                                            on_edit: if planner { Some(EventHandler::new(move |_| editing.set(Some((section_id, Some(pid)))))) } else { None },
                                            on_up: if planner && !first { Some(EventHandler::new(move |_| state.update_directory(|d| d.move_midweek_part(&mu, pid, -1)))) } else { None },
                                            on_down: if planner && !last { Some(EventHandler::new(move |_| state.update_directory(|d| d.move_midweek_part(&md, pid, 1)))) } else { None },
                                        }
                                    }
                                }
                            }
                            if planner {
                                Cluster { gap: Gap::Sm,
                                    ActionButton { action: Action::Add, variant: ButtonVariant::Ghost, label: "Add part", onclick: move |_| editing.set(Some((section_id, None))) }
                                    ActionButton { action: Action::Delete, variant: ButtonVariant::Ghost, label: "Delete section", onclick: move |_| confirm_section.set(Some(section_id)) }
                                }
                            }
                        }
                    }
                }
            }
            MeetingSection { title: "Closing", subtitle: format!("{} – {}", at(tl.concluding), at(tl.end)), icon: Icon::Song,
                AssignmentRow { number: "", icon: Icon::User, title: "Concluding comments", minutes: format!("{} · {} min", at(tl.concluding), crate::midweek::CONCLUDING_COMMENTS), detail: "By the chairman.", slots: Vec::new() }
                AssignmentRow { number: "", icon: Icon::Song, title: song_title(w.songs[2]), minutes: format!("{} · {} min", at(tl.closing_song), crate::midweek::CLOSING_SONG), slots: Vec::new() }
                AssignmentRow {
                    number: "",
                    icon: Icon::User,
                    title: "Closing prayer",
                    minutes: format!("{} · {} min", at(tl.closing_prayer), crate::midweek::CLOSING_PRAYER),
                    slots: slot_views(&dir, &prefs, &w, &[("Prayer".to_string(), MwRole::ClosingPrayer)], planner, &date),
                    on_slot: if planner { Some(EventHandler::new(move |_| choosing.set(Some(Choosing(MwRole::ClosingPrayer))))) } else { None },
                }
            }
            if planner {
                Cluster { justify: Justify::Center,
                    ActionButton { action: Action::Add, variant: ButtonVariant::Secondary, label: "Add section", onclick: move |_| adding_section.set(true) }
                }
            }
        }
        if let Some(Choosing(role)) = choosing() {
            ChooseDialog { monday: monday.clone(), role, onclose: move |_| choosing.set(None) }
        }
        if let Some((section, part)) = editing() {
            PartDialog { key: "{section}-{part:?}", monday: monday.clone(), section, part, onclose: move |_| editing.set(None) }
        }
        if adding_section() {
            SectionDialog { monday: monday.clone(), onclose: move |_| adding_section.set(false) }
        }
        if editing_header() {
            HeaderDialog { monday: monday.clone(), onclose: move |_| editing_header.set(false) }
        }
        ConfirmDialog {
            open: confirm_section().is_some(),
            title: "Delete this section?",
            message: "Its parts and their assignments are deleted from this week.",
            confirm: Action::Delete,
            oncancel: move |_| confirm_section.set(None),
            onconfirm: {
                let m = monday.clone();
                move |_| {
                    if let Some(s) = confirm_section() {
                        state.update_directory(|d| d.delete_midweek_section(&m, s));
                    }
                    confirm_section.set(None);
                }
            },
        }
    }
}

fn weeks_ago(from: &str, to: &str) -> String {
    let days = crate::territories::days_between(from, to).unwrap_or(0);
    match days / 7 {
        0 => "this week".into(),
        1 => "1 week ago".into(),
        n if n < 9 => format!("{n} weeks ago"),
        n => format!("{} months ago", n * 7 / 30),
    }
}

/// Choose a person for a place: available first, then busy that week, then away; within each,
/// the longest without this kind of part, then without any part.
#[component]
fn ChooseDialog(monday: String, role: MwRole, onclose: EventHandler<()>) -> Element {
    let mut state = use_app_state();
    let prefs = state.preferences();
    let dir = state.directory();
    let wd = weekday(&state);
    let mut query = use_signal(String::new);
    let Some(w) = dir.midweek_week(&monday).cloned() else { return rsx! {} };
    let label = w.role_label(role);
    let candidates: Vec<MwCandidate> = dir.midweek_candidates(&monday, role, wd);
    let q = crate::people::fold(&query());
    let row = |c: &MwCandidate| {
        let this = match &c.last_this {
            Some(d) => format!("This part {}", weeks_ago(d, &monday)),
            None => "Never had this part".to_string(),
        };
        let any = match &c.last_any {
            Some(d) => format!("any part {}", weeks_ago(d, &monday)),
            None => "never any part".to_string(),
        };
        let mut r = Row::new(c.person.to_string(), name(&dir, &prefs, c.person)).with_subtitle(format!("{this} · {any}"));
        if c.fit == MwFit::Away {
            r = r.with_tag(Tag::new("Away", Tone::Error).with_icon(Icon::Absence));
        } else if let Some(b) = c.busy.first() {
            r = r.with_tag(Tag::new(format!("Also: {b}"), Tone::Warning).with_icon(Icon::Warning));
        }
        r
    };
    let matches = |c: &&MwCandidate| q.is_empty() || crate::people::fold(&name(&dir, &prefs, c.person)).contains(&q);
    let groups = vec![
        RowGroup { title: "Available".into(), rows: candidates.iter().filter(matches).filter(|c| c.fit == MwFit::Available && c.busy.is_empty()).map(row).collect() },
        RowGroup { title: "Already has a part this week".into(), rows: candidates.iter().filter(matches).filter(|c| c.fit == MwFit::Available && !c.busy.is_empty()).map(row).collect() },
        RowGroup { title: "Away that day".into(), rows: candidates.iter().filter(matches).filter(|c| c.fit == MwFit::Away).map(row).collect() },
    ]
    .into_iter()
    .filter(|g| !g.rows.is_empty())
    .collect::<Vec<_>>();
    let current = w.person_at(role).map(|p| p.to_string());
    let m = monday.clone();
    rsx! {
        Dialog {
            open: true,
            onclose: move |_| onclose.call(()),
            title: "{label}",
            description: format!("{} · The longest without this part first.", day_title(&prefs, &meeting_date(&monday, wd))),
            Stack {
                TextInput { kind: InputKind::Search, value: query(), placeholder: "Search", oninput: move |v| query.set(v) }
                if groups.is_empty() {
                    Text { tone: TextTone::Muted, "Nobody has the privilege for this part. Give it in Users › Privileges." }
                } else {
                    ChoiceList {
                        groups,
                        selected: current,
                        none_label: "Nobody",
                        on_choose: move |v: Option<String>| {
                            state.update_directory(|d| d.assign_midweek(&m, role, v.and_then(|x| x.parse().ok())));
                            onclose.call(());
                        },
                    }
                }
            }
        }
    }
}

#[component]
fn PartDialog(monday: String, section: u32, part: Option<u32>, onclose: EventHandler<()>) -> Element {
    let mut state = use_app_state();
    let toaster = use_toaster();
    let dir = state.directory();
    let w = dir.midweek_week(&monday).cloned();
    let existing = part.and_then(|p| w.as_ref().and_then(|w| w.part(p).cloned()));
    let content = w.as_ref().and_then(|w| content_of(&state, w));
    let (wb_title, wb_desc) = existing.as_ref().map(|p| part_text(&MwPart { title: None, description: None, ..p.clone() }, content.as_ref())).unwrap_or_default();
    let (cur_title, cur_desc) = existing.as_ref().map(|p| part_text(p, content.as_ref())).unwrap_or_default();
    let mut kind = use_signal(|| existing.as_ref().map(|p| p.kind.value().to_string()).unwrap_or_else(|| PartKind::Other.value().to_string()));
    let mut title = use_signal(|| cur_title.clone());
    let mut description = use_signal(|| cur_desc.clone());
    let mut minutes = use_signal(|| existing.as_ref().map(|p| p.minutes.to_string()).unwrap_or_else(|| "5".into()));
    let mut error = use_signal(|| None::<String>);
    let options: Vec<SelectOption> = PartKind::ALL.iter().map(|k| SelectOption::new(k.value(), k.label())).collect();
    let m = monday.clone();
    let from_workbook = existing.as_ref().is_some_and(|p| p.source.is_some());
    let save = move |_| {
        // Text that is still the workbook's is not stored (it stays on the device, R-033).
        let t = if from_workbook && title() == wb_title { String::new() } else { title() };
        let d = if from_workbook && description() == wb_desc { String::new() } else { description() };
        let draft = PartDraft { kind: Some(PartKind::parse(&kind())), title: if part.is_none() { title() } else { t }, description: d, minutes: minutes() };
        match state.update_directory(|x| x.save_midweek_part(&m, section, part, &draft)) {
            Ok(_) => {
                toaster.success("Part saved.");
                onclose.call(());
            }
            Err(e) => error.set(Some(e)),
        }
    };
    let m2 = monday.clone();
    rsx! {
        Dialog {
            open: true,
            onclose: move |_| onclose.call(()),
            title: if existing.is_some() { "Edit part" } else { "Add part" },
            description: "Changes stay when the workbook is refreshed.",
            actions: rsx! {
                EditActions {
                    on_save: save,
                    on_cancel: move |_| onclose.call(()),
                    on_delete: part.map(|p| EventHandler::new(move |_| {
                        state.update_directory(|d| d.delete_midweek_part(&m2, p));
                        toaster.success("Part deleted.");
                        onclose.call(());
                    })),
                }
            },
            Stack {
                if let Some(e) = error() {
                    Alert { tone: Tone::Error, "{e}" }
                }
                Grid { cols: 2,
                    Field { label: "Kind", help: "Decides who can have it.",
                        Select { value: kind(), options, onchange: move |v| kind.set(v) }
                    }
                    Field { label: "Minutes", required: true,
                        TextInput { kind: InputKind::Count, value: minutes(), oninput: move |v| minutes.set(v) }
                    }
                }
                Field { label: "Title", required: part.is_none(),
                    TextInput { value: title(), oninput: move |v| title.set(v) }
                }
                Field { label: "Description",
                    TextArea { value: description(), rows: 4, oninput: move |v| description.set(v) }
                }
            }
        }
    }
}

#[component]
fn SectionDialog(monday: String, onclose: EventHandler<()>) -> Element {
    let mut state = use_app_state();
    let toaster = use_toaster();
    let mut title = use_signal(String::new);
    let mut error = use_signal(|| None::<String>);
    rsx! {
        Dialog {
            open: true,
            onclose: move |_| onclose.call(()),
            title: "Add section",
            description: "For example for the circuit overseer's visit.",
            actions: rsx! {
                ActionButton { action: Action::Cancel, onclick: move |_| onclose.call(()) }
                ActionButton {
                    action: Action::Add,
                    onclick: move |_| match state.update_directory(|d| d.add_midweek_section(&monday, &title())) {
                        Ok(_) => {
                            toaster.success("Section added.");
                            onclose.call(());
                        }
                        Err(e) => error.set(Some(e)),
                    },
                }
            },
            Field { label: "Title", required: true, error: error(),
                TextInput { value: title(), oninput: move |v| title.set(v) }
            }
        }
    }
}

#[component]
fn HeaderDialog(monday: String, onclose: EventHandler<()>) -> Element {
    let mut state = use_app_state();
    let w = state.directory().midweek_week(&monday).cloned();
    let mut reading = use_signal(|| w.as_ref().map(|w| w.reading.clone()).unwrap_or_default());
    let song = |i: usize| w.as_ref().and_then(|w| w.songs[i]).map(|n| n.to_string()).unwrap_or_default();
    let mut s1 = use_signal(|| song(0));
    let mut s2 = use_signal(|| song(1));
    let mut s3 = use_signal(|| song(2));
    rsx! {
        Dialog {
            open: true,
            onclose: move |_| onclose.call(()),
            title: "Bible reading and songs",
            actions: rsx! {
                ActionButton { action: Action::Cancel, onclick: move |_| onclose.call(()) }
                ActionButton {
                    action: Action::Save,
                    onclick: move |_| {
                        let n = |s: String| s.trim().parse::<u16>().ok();
                        state.update_directory(|d| d.set_midweek_header(&monday, &reading(), [n(s1()), n(s2()), n(s3())]));
                        onclose.call(());
                    },
                }
            },
            Stack {
                Field { label: "Bible reading",
                    TextInput { value: reading(), oninput: move |v| reading.set(v) }
                }
                Grid { cols: 3,
                    Field { label: "Opening song", TextInput { kind: InputKind::Count, value: s1(), oninput: move |v| s1.set(v) } }
                    Field { label: "Middle song", TextInput { kind: InputKind::Count, value: s2(), oninput: move |v| s2.set(v) } }
                    Field { label: "Closing song", TextInput { kind: InputKind::Count, value: s3(), oninput: move |v| s3.set(v) } }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Month
// ---------------------------------------------------------------------------------------------

#[component]
fn MonthTab(month: Ym, planner: bool) -> Element {
    let mut state = use_app_state();
    let prefs = state.preferences();
    let dir = state.directory();
    let wd = weekday(&state);
    let mondays = mondays_in(month, wd);
    rsx! {
        Grid { cols: 2,
            for m in mondays {
                {
                    let date = meeting_date(&m, wd);
                    let w = dir.midweek_week(&m).cloned();
                    let open = { let m = m.clone(); move |_| { state.mw_monday.set(Some(m.clone())); state.mw_tab.set("week".into()); } };
                    match w {
                        None => rsx! {
                            Card { key: "{m}", title: day_title(&prefs, &date), icon: Icon::Calendar,
                                Text { tone: TextTone::Muted, "No plan yet." }
                                TextLink { icon: Icon::ArrowRight, onclick: open, "Open the week" }
                            }
                        },
                        Some(w) => {
                            let content = content_of(&state, &w);
                            let visible = planner || w.sent.is_some();
                            let mut items: Vec<StatItem> = Vec::new();
                            if visible && !w.no_meeting {
                                let who = |role: MwRole| -> Option<String> {
                                    let p = if planner { w.person_at(role) } else { w.sent.as_ref().and_then(|s| s.places.iter().find(|(k, _)| *k == crate::midweek::role_key(role)).map(|(_, p)| *p)) };
                                    p.map(|p| name(&dir, &prefs, p))
                                };
                                items.push(StatItem::new("Chairman", who(MwRole::Chairman).unwrap_or_else(|| "–".into())));
                                for p in w.parts() {
                                    let (t, _) = part_text(p, content.as_ref());
                                    let mut v = who(MwRole::Main(p.id)).unwrap_or_else(|| "–".into());
                                    if p.kind.second().is_some()
                                        && let Some(a) = who(MwRole::Assistant(p.id))
                                    {
                                        v.push_str(&format!(" · {a}"));
                                    }
                                    let tone = if v == "–" && planner { Tone::Warning } else { Tone::Neutral };
                                    items.push(StatItem::new(t, v).with_tone(tone));
                                }
                            }
                            let status = w.status(&date);
                            let reading_text = if w.no_meeting { "No meeting this week".to_string() } else { w.reading.clone() };
                            rsx! {
                                Card { key: "{m}", title: day_title(&prefs, &date), icon: Icon::BookOpen,
                                    Cluster { justify: Justify::Between,
                                        Text { tone: TextTone::Small, "{reading_text}" }
                                        if planner && let Some(t) = status_tag(status) {
                                            Badge { tone: t.tone, icon: t.icon.unwrap_or(Icon::Info), "{t.label}" }
                                        }
                                    }
                                    if visible && !items.is_empty() {
                                        StatList { items }
                                    } else if !visible {
                                        Text { tone: TextTone::Muted, "Not sent yet." }
                                    }
                                    TextLink { icon: Icon::ArrowRight, onclick: open, "Open the week" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Statistics
// ---------------------------------------------------------------------------------------------

#[component]
fn StatisticsTab() -> Element {
    let state = use_app_state();
    let prefs = state.preferences();
    let dir = state.directory();
    let mut period = use_signal(|| "6".to_string());
    let today = today_iso();
    let months: i32 = period().parse().unwrap_or(6);
    let from = crate::territories::add_months(&today, -months).unwrap_or_else(|| today.clone());
    let to = monday_of(&today);
    let weeks: Vec<&MidweekWeek> = dir.midweek_weeks.iter().filter(|w| w.monday >= from && w.monday <= to && !w.no_meeting).collect();
    let (filled, places) = weeks.iter().map(|w| w.filled()).fold((0, 0), |a, b| (a.0 + b.0, a.1 + b.1));
    let per_person = dir.midweek_per_person(&from, &to);
    let with_part = per_person.iter().filter(|x| x.1 > 0).count();
    let top = per_person.first().map(|x| x.1).unwrap_or(1).max(1);
    let last_any = dir.midweek_last_any(&add_days(&to, 7));
    let mut longest: Vec<(u32, Option<String>)> = per_person.iter().map(|(p, _)| (*p, last_any.get(p).cloned())).collect();
    longest.sort_by(|a, b| a.1.cmp(&b.1));
    let per_kind = dir.midweek_per_kind(&from, &to);
    let kind_top = per_kind.first().map(|x| x.1).unwrap_or(1).max(1);
    rsx! {
        Segmented {
            value: period(),
            options: vec![
                Segment { value: "3".into(), label: "Last 3 months".into(), icon: Some(Icon::Calendar) },
                Segment { value: "6".into(), label: "Last 6 months".into(), icon: Some(Icon::Calendar) },
                Segment { value: "12".into(), label: "Last 12 months".into(), icon: Some(Icon::History) },
            ],
            onchange: move |v: String| period.set(v),
        }
        Grid { cols: 4, gap: Gap::Sm, compact: true,
            Card { StatTile { label: "Meetings", value: weeks.len().to_string(), icon: Icon::BookOpen, delta: "planned weeks".to_string() } }
            Card { StatTile { label: "Places filled", value: format!("{}%", (filled * 100).checked_div(places).unwrap_or(0)), icon: Icon::Check, delta: format!("{filled} of {places}") } }
            Card { StatTile { label: "Had a part", value: format!("{with_part} of {}", per_person.len()), icon: Icon::Users, delta: "with a privilege".to_string() } }
            Card { StatTile { label: "Parts per person", value: if with_part > 0 { format!("{:.1}", filled as f64 / with_part as f64) } else { "–".into() }, icon: Icon::User, delta: "on average".to_string() } }
        }
        Grid { cols: 2,
            Card { title: "Assignments per person", icon: Icon::Users,
                if per_person.is_empty() {
                    Text { tone: TextTone::Muted, "Nobody has a midweek privilege yet." }
                } else {
                    Stack { gap: Gap::Sm,
                        for (p, n) in per_person.iter().take(20) {
                            ProgressRow {
                                key: "{p}",
                                label: name(&dir, &prefs, *p),
                                detail: if *n == 1 { "1 part".to_string() } else { format!("{n} parts") },
                                value: *n as f64,
                                max: top as f64,
                                tone: if *n == 0 { Tone::Warning } else { Tone::Neutral },
                            }
                        }
                    }
                }
            }
            Stack {
                Card { title: "Longest without a part", icon: Icon::History,
                    ItemList {
                        compact: true,
                        items: longest
                            .iter()
                            .take(8)
                            .map(|(p, last)| ListEntry {
                                icon: if last.is_none() { Icon::Warning } else { Icon::User },
                                title: name(&dir, &prefs, *p),
                                subtitle: match last {
                                    Some(d) => format!("Last part {}", weeks_ago(d, &to)),
                                    None => "Never had a part".into(),
                                },
                                meta: String::new(),
                            })
                            .collect::<Vec<_>>(),
                    }
                }
                Card { title: "Parts by kind", icon: Icon::Chart,
                    Stack { gap: Gap::Sm,
                        for (k, n) in per_kind.iter() {
                            ProgressRow { key: "{k:?}", label: k.label(), detail: format!("{n}"), value: *n as f64, max: kind_top as f64, tone: Tone::Neutral }
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Export
// ---------------------------------------------------------------------------------------------

/// The assignments of whole months, one line per part, as a table (PDF, Word or CSV).
fn export_table(state: &AppState, months: &[Ym], planner: bool) -> crate::export::Table {
    let prefs = state.preferences();
    let dir = state.directory();
    let wd = weekday(state);
    let start = state.active_congregation().and_then(|c| crate::midweek::minutes_of(&c.midweek.time)).unwrap_or(19 * 60);
    let at = |m: u32| prefs.format_hhmm(&crate::midweek::hhmm(m));
    let mondays: Vec<String> = months.iter().flat_map(|m| mondays_in(*m, wd)).collect();
    let weeks: Vec<MidweekWeek> = mondays.iter().filter_map(|m| dir.midweek_week(m).cloned()).filter(|w| planner || w.sent.is_some()).collect();
    let aux = weeks.iter().any(|w| w.aux_on && !w.no_meeting);
    let mut rows: Vec<Vec<String>> = Vec::new();
    for w in &weeks {
        let date = day_title(&prefs, &meeting_date(&w.monday, wd));
        if w.no_meeting {
            let mut r = vec![date, String::new(), "No meeting".into(), String::new(), String::new()];
            if aux {
                r.extend([String::new(), String::new()]);
            }
            rows.push(r);
            continue;
        }
        // Readers get what was sent.
        let who = |role: MwRole| -> String {
            let p = if planner { w.person_at(role) } else { w.sent.as_ref().and_then(|s| s.places.iter().find(|(k, _)| *k == crate::midweek::role_key(role)).map(|(_, p)| *p)) };
            p.map(|p| name(&dir, &prefs, p)).unwrap_or_default()
        };
        let content = content_of(state, w);
        let tl = w.timeline(start);
        let mut line = |time: String, what: String, main: String, second: String, aux_main: String, aux_second: String| {
            let mut r = vec![date.clone(), time, what, main, second];
            if aux {
                r.extend([aux_main, aux_second]);
            }
            rows.push(r);
        };
        line(at(tl.opening_song), "Chairman".into(), who(MwRole::Chairman), String::new(), if w.aux_on { who(MwRole::AuxCounselor) } else { String::new() }, String::new());
        line(at(tl.opening_prayer), w.songs[0].map(|n| format!("Song {n} · opening prayer")).unwrap_or_else(|| "Opening prayer".into()), who(MwRole::OpeningPrayer), String::new(), String::new(), String::new());
        for (i, p) in w.parts().enumerate() {
            let (title, _) = part_text(p, content.as_ref());
            let in_aux = w.aux_on && p.kind.in_aux_classroom();
            line(
                tl.part(p.id).map(at).unwrap_or_default(),
                format!("{}. {title} ({} min)", i + 1, p.minutes),
                who(MwRole::Main(p.id)),
                if p.kind.second().is_some() { who(MwRole::Assistant(p.id)) } else { String::new() },
                if in_aux { who(MwRole::AuxMain(p.id)) } else { String::new() },
                if in_aux && p.kind.second().is_some() { who(MwRole::AuxAssistant(p.id)) } else { String::new() },
            );
        }
        line(at(tl.closing_prayer), w.songs[2].map(|n| format!("Song {n} · closing prayer")).unwrap_or_else(|| "Closing prayer".into()), who(MwRole::ClosingPrayer), String::new(), String::new(), String::new());
    }
    let mut columns: Vec<String> = ["Date", "Time", "Part", "Assigned", "Assistant or reader"].iter().map(|c| c.to_string()).collect();
    let mut widths = vec![1.6, 0.6, 3.2, 1.6, 1.6];
    if aux {
        columns.extend(["Auxiliary classroom".to_string(), "Aux. assistant".to_string()]);
        widths.extend([1.6, 1.6]);
    }
    let range = match (months.first(), months.last()) {
        (Some(a), Some(b)) if a != b => format!("{} – {}", month_label(*a), month_label(*b)),
        (Some(a), _) => month_label(*a),
        _ => String::new(),
    };
    crate::export::Table {
        title: format!("{} · Midweek meeting", state.active_congregation().map(|c| c.name).unwrap_or_default()),
        subtitle: if planner { range } else { format!("{range} · as sent") },
        columns,
        widths,
        rows,
    }
}

/// Export whole months of assignments: the first month, how many, and the format.
#[component]
fn ExportDialog(first: Ym, planner: bool, onclose: EventHandler<()>) -> Element {
    let state = use_app_state();
    let toaster = use_toaster();
    let mut from = use_signal(|| first);
    let mut count = use_signal(|| 1usize);
    let months: Vec<Ym> = (0..count()).map(|i| crate::reports::shift(from(), i as i32)).collect();
    let preview = export_table(&state, &months, planner);
    let meetings = preview.rows.iter().map(|r| r[0].clone()).collect::<BTreeSet<_>>().len();
    let meetings_text = if meetings == 1 { "1 meeting.".to_string() } else { format!("{meetings} meetings.") };
    let save = move |format: crate::export::Format| {
        let months: Vec<Ym> = (0..count()).map(|i| crate::reports::shift(from(), i as i32)).collect();
        let table = export_table(&state, &months, planner);
        if table.rows.is_empty() {
            toaster.error("There are no meetings to export in these months.");
            return;
        }
        let congregation = state.active_congregation().map(|c| c.name).unwrap_or_default();
        let f = from();
        super::users::save_export(&table, format, &congregation, &format!("midweek-{:04}-{:02}-{}m", f.0, f.1, count()), toaster);
        onclose.call(());
    };
    let counts: Vec<SelectOption> = (1..=12).map(|n| SelectOption::new(n.to_string(), if n == 1 { "1 month".to_string() } else { format!("{n} months") })).collect();
    rsx! {
        Dialog {
            open: true,
            onclose: move |_| onclose.call(()),
            title: "Export the midweek meeting",
            description: "Every meeting of the chosen months, one line per part, with the time and who has it.",
            actions: rsx! {
                ActionButton { action: Action::Cancel, onclick: move |_| onclose.call(()) }
            },
            Stack {
                Field { label: "From",
                    MonthPicker { value: from(), min: crate::reports::shift(this_month(), -24), max: crate::reports::shift(this_month(), 12), today: this_month(), full_width: true, onchange: move |m| from.set(m) }
                }
                Field { label: "How many months",
                    Select { value: count().to_string(), options: counts, onchange: move |v: String| count.set(v.parse().unwrap_or(1)) }
                }
                Text { tone: TextTone::Small, "{meetings_text}" }
                Grid { cols: 3,
                    Button { variant: ButtonVariant::Secondary, icon: Icon::FilePdf, onclick: move |_| save(crate::export::Format::Pdf), "PDF" }
                    Button { variant: ButtonVariant::Secondary, icon: Icon::FileDocx, onclick: move |_| save(crate::export::Format::Docx), "Word" }
                    Button { variant: ButtonVariant::Secondary, icon: Icon::FileCsv, onclick: move |_| save(crate::export::Format::Csv), "CSV" }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// This week's meeting, as sent (home widget)
// ---------------------------------------------------------------------------------------------

/// One line of the sent plan: (time and part, who, it is me).
pub(super) type SentLine = (String, String, bool);

/// The next meeting as it was sent: (its day and times, the lines). None when it is not sent.
pub(super) fn next_meeting_lines(state: &AppState, with_times: bool) -> Option<(String, Vec<SentLine>)> {
    let prefs = state.preferences();
    let dir = state.directory();
    let wd = weekday(state);
    let monday = next_monday(wd);
    let w = dir.midweek_week(&monday)?.clone();
    let sent = w.sent.clone()?;
    let date = meeting_date(&monday, wd);
    let start = state.active_congregation().and_then(|c| crate::midweek::minutes_of(&c.midweek.time)).unwrap_or(19 * 60);
    let tl = w.timeline(start);
    let at = |m: u32| prefs.format_hhmm(&crate::midweek::hhmm(m));
    let title = format!("{} · {} – {}", day_title(&prefs, &sent.date), at(tl.opening_song), at(tl.end));
    // "19:26 " before each line, or nothing (the widget has no room for it).
    let time = |m: u32| if with_times { format!("{} ", at(m)) } else { String::new() };
    if sent.no_meeting {
        return Some((format!("{} · no meeting", day_title(&prefs, &date)), Vec::new()));
    }
    let me = state.me();
    let who = |role: MwRole| sent.places.iter().find(|(k, _)| *k == crate::midweek::role_key(role)).map(|(_, p)| *p);
    let content = content_of(state, &w);
    let mut lines: Vec<SentLine> = Vec::new();
    let mut push = |label: String, people: Vec<Option<u32>>| {
        let ids: Vec<u32> = people.into_iter().flatten().collect();
        if ids.is_empty() {
            return;
        }
        let mine = me.is_some_and(|m| ids.contains(&m));
        lines.push((label, ids.iter().map(|p| name(&dir, &prefs, *p)).collect::<Vec<_>>().join(" · "), mine));
    };
    push(format!("{}Chairman", time(tl.opening_song)), vec![who(MwRole::Chairman)]);
    if w.aux_on {
        push(format!("{}Auxiliary classroom", time(tl.opening_song)), vec![who(MwRole::AuxCounselor)]);
    }
    push(format!("{}Opening prayer", time(tl.opening_prayer)), vec![who(MwRole::OpeningPrayer)]);
    for (i, part) in w.parts().enumerate() {
        let (t, _) = part_text(part, content.as_ref());
        let start = tl.part(part.id).map(time).unwrap_or_default();
        push(format!("{start}{}. {t}", i + 1), vec![who(MwRole::Main(part.id)), who(MwRole::Assistant(part.id))]);
        if w.aux_on && part.kind.in_aux_classroom() {
            push(format!("{start}{}. {t} · aux. classroom", i + 1), vec![who(MwRole::AuxMain(part.id)), who(MwRole::AuxAssistant(part.id))]);
        }
    }
    push(format!("{}Closing prayer", time(tl.closing_prayer)), vec![who(MwRole::ClosingPrayer)]);
    Some((title, lines))
}
