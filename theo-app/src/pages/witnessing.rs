//! *Ministry › Public witnessing*: shifts (list or calendar), weekly presets with the people a
//! shift needs and who is available, shift supervisors, filling and assigning a month, sending,
//! and statistics. Same flow as meetings for field service.
//! Rules: `docs/model/m16-public-witnessing.md`.

use crate::people::{self, parse_date, Directory, Person};
use crate::prefs::{Preferences, WeekStart};
use crate::public_witnessing::{resize_slots, Fit, Needs, Role, ShiftDraft, ShiftVersion, Slot, WEEKDAYS};
use crate::reports::{month_label, Ym};
use crate::service_meetings::{weekday_of, SendStatus};
use crate::state::use_app_state;
use dioxus::prelude::*;
use std::collections::BTreeSet;
use theo_ui::*;
use theo_ui::Action; // explicit: dioxus::prelude also exports an `Action`

const PAGE: &str = "ministry/witnessing";

fn this_month() -> Ym {
    let (y, m, _) = crate::prefs::today();
    (y, m)
}

fn day_title(iso: &str) -> String {
    match (parse_date(iso), weekday_of(iso)) {
        (Some((_, m, d)), Some(w)) => format!("{} {d} {}", WEEKDAYS[w as usize], theo_ui::month_name(m)),
        _ => iso.to_string(),
    }
}

fn week_order(prefs: &Preferences) -> Vec<u8> {
    let first = if prefs.week_start == WeekStart::Sunday { 0 } else { 1 };
    (0..7).map(|i| (first + i) % 7).collect()
}

fn names(dir: &Directory, prefs: &Preferences, slots: &[Slot]) -> String {
    slots.iter().filter_map(|s| s.person).filter_map(|p| dir.person(p)).map(|p| p.display_name(prefs)).collect::<Vec<_>>().join(", ")
}

fn status_tag(s: SendStatus) -> Option<Tag> {
    match s {
        SendStatus::Sent => None,
        SendStatus::NotSent => Some(Tag::new("Not sent", Tone::Warning).with_icon(Icon::Clock)),
        SendStatus::Changed => Some(Tag::new("Changed · not sent", Tone::Warning).with_icon(Icon::Edit)),
        SendStatus::Removed => Some(Tag::new("Removed · not sent", Tone::Error).with_icon(Icon::Trash)),
    }
}

fn filled_tag(filled: usize, places: usize) -> Tag {
    match (filled, places) {
        (0, _) => Tag::new("Nobody yet", Tone::Error).with_icon(Icon::Warning),
        (f, p) if f < p => Tag::new(format!("{f} of {p} people"), Tone::Warning).with_icon(Icon::Users),
        (_, p) => Tag::new(if p == 1 { "1 person".to_string() } else { format!("{p} people") }, Tone::Success).with_icon(Icon::Users),
    }
}

fn fit_tag(f: Fit) -> Option<Tag> {
    match f {
        Fit::Available => Some(Tag::new(f.label(), Tone::Success).with_icon(Icon::Check)),
        Fit::NotForThisSlot | Fit::BusyThatDay => Some(Tag::new(f.label(), Tone::Warning).with_icon(Icon::Clock)),
        Fit::Away => Some(Tag::new(f.label(), Tone::Error).with_icon(Icon::Absence)),
        Fit::NotSupervisor | Fit::WrongGender | Fit::NoPrivilege => None,
    }
}

/// `/app/ministry/witnessing`
#[component]
pub fn WitnessingPage() -> Element {
    let mut state = use_app_state();
    let toaster = use_toaster();
    let congregation = state.active_congregation().map(|c| c.name).unwrap_or_default();
    let planner = state.can_write(PAGE);
    let dir = state.directory();
    let mut view = (state.pw_view)();
    // Presets have their own page now; an old "presets" choice falls back to the list.
    if view == "presets" || (!planner && view == "statistics") {
        view = "list".into();
    }
    let navigator = navigator();
    let month = (state.pw_month)().unwrap_or_else(this_month);
    let mut editing = use_signal(|| None::<Option<u32>>);
    let mut confirm_send = use_signal(|| false);
    let mut confirm_clear = use_signal(|| false);
    let unsent = dir.unsent_shift_changes(month);
    let count = |s: SendStatus| unsent.iter().filter(|(_, x)| *x == s).count();
    let (new, changed, removed) = (count(SendStatus::NotSent), count(SendStatus::Changed), count(SendStatus::Removed));

    let on_fill = move |id: String| {
        if (id == "fill" || id == "fill-assign") && state.directory().shift_presets.iter().all(|p| !p.enabled) {
            toaster.error("Add the weekly shifts first (Presets), or switch one on.");
            navigator.push(crate::Route::WitnessingPresetsPage {});
            return;
        }
        match id.as_str() {
            "fill" | "fill-assign" => {
                let r = state.update_directory(|d| d.fill_shift_month(month, id == "fill-assign"));
                let mut text = match r.added {
                    0 => "The month already has every shift of the presets.".to_string(),
                    1 => "1 shift added.".to_string(),
                    n => format!("{n} shifts added."),
                };
                if id == "fill-assign" {
                    text.push_str(&format!(" {} places filled.", r.assigned));
                }
                if !r.skipped_days.is_empty() {
                    text.push_str(&format!(" {} days skipped: the congregation is away.", r.skipped_days.len()));
                }
                if r.empty > 0 {
                    toaster.show(Tone::Warning, format!("{text} {} places empty: nobody fits.", r.empty));
                } else {
                    toaster.success(text);
                }
            }
            "assign" => {
                let (a, left) = state.update_directory(|d| d.assign_shifts(month));
                if left > 0 {
                    toaster.show(Tone::Warning, format!("{a} places filled. {left} still empty: nobody fits."));
                } else if a == 0 {
                    toaster.success("Every place already has someone.");
                } else {
                    toaster.success(format!("{a} places filled."));
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
            let rows: Vec<Vec<String>> = shown_shifts(&dir, month, planner)
                .into_iter()
                .filter(|(_, _, s)| *s != Some(SendStatus::Removed))
                .map(|(_, v, _)| {
                    let sup = v.slots.iter().find(|x| x.role == Role::Supervisor).and_then(|x| x.person).and_then(|p| dir.person(p)).map(|p| p.display_name(&prefs)).unwrap_or_default();
                    let others: Vec<String> = v.slots.iter().filter(|x| x.role != Role::Supervisor).filter_map(|x| x.person).filter_map(|p| dir.person(p)).map(|p| p.display_name(&prefs)).collect();
                    vec![day_title(&v.date), format!("{}–{}", v.start, v.end), v.place.clone(), sup, others.join(", ")]
                })
                .collect();
            if rows.is_empty() {
                toaster.error("There are no shifts to export in this month.");
                return;
            }
            let columns = ["Date", "Time", "Place", "Shift supervisor", "People"];
            let table = crate::export::Table {
                title: format!("{congregation} · Public witnessing"),
                subtitle: month_label(month),
                columns: columns.iter().map(|c| c.to_string()).collect(),
                widths: vec![2.0, 1.2, 2.0, 1.6, 3.2],
                rows,
            };
            super::users::save_export(&table, format, &congregation, &format!("public-witnessing-{:04}-{:02}", month.0, month.1), toaster);
        }
    };
    let export = rsx! { super::users::ExportMenu { csv_description: "The shifts of the month, for spreadsheets", onselect: on_export } };
    let actions: Element = if !planner {
        export
    } else if view == "statistics" {
        rsx! { ActionButton { action: Action::Settings, variant: ButtonVariant::Secondary, label: "Presets", onclick: move |_| { navigator.push(crate::Route::WitnessingPresetsPage {}); } } }
    } else {
        rsx! {
            ActionButton { action: Action::Settings, variant: ButtonVariant::Secondary, label: "Presets", onclick: move |_| { navigator.push(crate::Route::WitnessingPresetsPage {}); } }
            {export}
            MenuButton {
                label: "Fill month",
                icon: Icon::Magic,
                items: vec![
                    MenuItem::new("fill", "Fill", Icon::Calendar).with_description("Every preset on its days, without people"),
                    MenuItem::new("fill-assign", "Fill and assign", Icon::Magic).with_description("Every preset, and people for every place"),
                    MenuItem::new("assign", "Assign the empty places", Icon::User).with_description("People for places that have nobody"),
                    MenuItem::new("clear", "Clear the month", Icon::Trash).with_description("Delete every shift of the month"),
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
            ActionButton { action: Action::Add, label: "Add shift", onclick: move |_| editing.set(Some(None)) }
        }
    };

    rsx! {
        Page {
            title: "Public witnessing",
            subtitle: "{congregation} · Shifts at the carts and stands, and who takes part",
            actions,
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
                    onchange: move |v: String| state.pw_view.set(v),
                }
                if view == "list" {
                    MonthPicker {
                        value: month,
                        min: crate::reports::shift(this_month(), -24),
                        max: crate::reports::shift(this_month(), 12),
                        today: this_month(),
                        full_width: true,
                        onchange: move |m| state.pw_month.set(Some(m)),
                    }
                }
            }
            match view.as_str() {
                "statistics" => rsx! { StatisticsTab {} },
                _ => rsx! {
                    if planner {
                        Summary { month }
                    } else {
                        Text { tone: TextTone::Muted, "The public witnessing shifts of {month_label(month)}. Changes appear here when they are sent." }
                    }
                    if view == "calendar" {
                        ShiftsCalendar { month, planner, on_open: move |id| editing.set(Some(Some(id))) }
                    } else {
                        ShiftsList { month, planner, on_open: move |id| editing.set(Some(Some(id))) }
                    }
                },
            }
        }
        if let Some(id) = editing() {
            ShiftDialog { key: "{id:?}", id, month, onclose: move |_| editing.set(None) }
        }
        ConfirmDialog {
            open: confirm_send(),
            title: format!("Send {}?", month_label(month)),
            message: {
                let mut parts = Vec::new();
                if new > 0 { parts.push(format!("{new} new")); }
                if changed > 0 { parts.push(format!("{changed} changed")); }
                if removed > 0 { parts.push(format!("{removed} removed")); }
                format!("{}. Everyone who can see public witnessing will see the shifts as they are now, and the people concerned get a notice.", parts.join(", "))
            },
            confirm: Action::Send,
            oncancel: move |_| confirm_send.set(false),
            onconfirm: move |_| {
                let n = state.update_directory(|d| d.send_shift_month(month));
                confirm_send.set(false);
                toaster.success(if n == 1 { "1 change sent.".to_string() } else { format!("{n} changes sent.") });
            },
        }
        ConfirmDialog {
            open: confirm_clear(),
            title: format!("Clear {}?", month_label(month)),
            message: "Every shift of the month is deleted. Shifts that were sent stay visible to everyone until you send again.",
            confirm: Action::Delete,
            oncancel: move |_| confirm_clear.set(false),
            onconfirm: move |_| {
                state.update_directory(|d| d.clear_shift_month(month));
                confirm_clear.set(false);
                toaster.success("The month is cleared.");
            },
        }
    }
}

#[component]
fn Summary(month: Ym) -> Element {
    let state = use_app_state();
    let dir = state.directory();
    let shifts: Vec<_> = dir.shifts_in(month).into_iter().filter(|s| !s.removed).collect();
    let unsent = dir.unsent_shift_changes(month).len();
    let empty: usize = shifts.iter().map(|s| s.filled()).map(|(f, p)| p - f).sum();
    let without_supervisor = if dir.witnessing.supervisors_on { shifts.iter().filter(|s| !s.has_supervisor()).count() } else { 0 };
    if shifts.is_empty() && unsent == 0 {
        return rsx! {
            Alert { tone: Tone::Info, title: "Nothing planned for {month_label(month)}",
                "Use Fill month to add the weekly shifts from the presets, or Add shift for a single one."
            }
        };
    }
    let mut lines = Vec::new();
    if unsent > 0 {
        lines.push(format!("{unsent} {} not sent yet: only planners see {}.", if unsent == 1 { "change is" } else { "changes are" }, if unsent == 1 { "it" } else { "them" }));
    }
    if empty > 0 {
        lines.push(format!("{empty} {} without anyone.", if empty == 1 { "place is" } else { "places are" }));
    }
    if without_supervisor > 0 {
        lines.push(format!("{without_supervisor} {} no shift supervisor.", if without_supervisor == 1 { "shift has" } else { "shifts have" }));
    }
    if lines.is_empty() {
        return rsx! { Text { tone: TextTone::Small, "Everything is sent and every place is filled." } };
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

/// (id, version shown, status for planners).
fn shown_shifts(dir: &Directory, month: Ym, planner: bool) -> Vec<(u32, ShiftVersion, Option<SendStatus>)> {
    if planner {
        dir.shifts_in(month).iter().map(|s| (s.id, s.current(), Some(s.status()))).collect()
    } else {
        dir.sent_shifts_in(month).into_iter().map(|(id, v)| (id, v, None)).collect()
    }
}

#[component]
fn ShiftsList(month: Ym, planner: bool, on_open: EventHandler<u32>) -> Element {
    let state = use_app_state();
    let prefs = state.preferences();
    let dir = state.directory();
    let supervisors_on = dir.witnessing.supervisors_on;
    let shown = shown_shifts(&dir, month, planner);
    if shown.is_empty() {
        return rsx! {
            EmptyState { icon: Icon::PublicWitnessing, title: "No shifts this month", description: if planner { "Fill the month from the presets, or add a shift." } else { "They appear here when they are sent." } }
        };
    }
    rsx! {
        Grid { cols: 3, gap: Gap::Sm,
            for (id, v, status) in shown {
                {
                    let who = names(&dir, &prefs, &v.slots);
                    let filled = v.slots.iter().filter(|s| s.person.is_some()).count();
                    let mut tags = vec![filled_tag(filled, v.slots.len())];
                    match v.slots.iter().find(|s| s.role == Role::Supervisor && s.person.is_some()).and_then(|s| s.person).and_then(|p| dir.person(p)) {
                        Some(sup) => tags.push(Tag::new(sup.display_name(&prefs), Tone::Neutral).with_icon(Icon::Shield)),
                        None if supervisors_on => tags.push(Tag::new("No supervisor", Tone::Warning).with_icon(Icon::Shield)),
                        None => {}
                    }
                    if let Some(t) = status.and_then(status_tag) {
                        tags.push(t);
                    }
                    rsx! {
                        RecordCard {
                            key: "{id}",
                            title: format!("{} · {}–{}", day_title(&v.date), v.start, v.end),
                            avatar: v.place.clone(),
                            subtitle: if who.is_empty() { v.place.clone() } else { format!("{} · {who}", v.place) },
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
fn ShiftsCalendar(month: Ym, planner: bool, on_open: EventHandler<u32>) -> Element {
    let mut state = use_app_state();
    let prefs = state.preferences();
    let dir = state.directory();
    let (y, m, d) = crate::prefs::today();
    let versions: Vec<(u32, ShiftVersion, Option<SendStatus>)> = if planner {
        dir.shifts.iter().filter(|s| !s.removed).map(|s| (s.id, s.current(), Some(s.status()))).collect()
    } else {
        dir.shifts.iter().filter_map(|s| s.sent.clone().map(|v| (s.id, v, None))).collect()
    };
    let entries: Vec<CalendarEntry> = versions
        .into_iter()
        .filter_map(|(id, v, status)| {
            let date = parse_date(&v.date)?;
            let filled = v.slots.iter().filter(|s| s.person.is_some()).count();
            let tone = match (filled, status) {
                (0, _) => Tone::Error,
                (f, _) if f < v.slots.len() => Tone::Warning,
                (_, Some(SendStatus::NotSent | SendStatus::Changed)) => Tone::Warning,
                _ => Tone::Info,
            };
            let who = names(&dir, &prefs, &v.slots);
            Some(CalendarEntry {
                id: id.to_string(),
                title: format!("{} {}", v.start, v.place),
                subtitle: format!("{}–{} · {}", v.start, v.end, if who.is_empty() { "Nobody yet".into() } else { who.clone() }),
                from: date,
                to: date,
                tone,
                icon: Icon::PublicWitnessing,
                detail: Some(if who.is_empty() { "Nobody yet".into() } else { who }),
            })
        })
        .collect();
    rsx! {
        if planner {
            Text { tone: TextTone::Small, "Blue: sent and full. Yellow: not sent yet or places empty. Red: nobody yet." }
        }
        Calendar {
            month,
            entries,
            today: (y, m, d),
            min: crate::reports::shift(this_month(), -24),
            max: crate::reports::shift(this_month(), 12),
            week_starts_sunday: prefs.week_start == WeekStart::Sunday,
            empty_text: "No shifts in this month.",
            on_month: move |v| state.pw_month.set(Some(v)),
            on_select: move |id: String| {
                if planner && let Ok(id) = id.parse::<u32>() {
                    on_open.call(id);
                }
            },
        }
    }
}

// ---------------------------------------------------------------------------------------------
// The people of a shift: how many of each kind
// ---------------------------------------------------------------------------------------------

/// Number inputs for the people a shift needs (supervisors only when they are switched on).
#[component]
fn NeedsEditor(needs: Needs, supervisors_on: bool, onchange: EventHandler<Needs>, #[props(default)] error: Option<String>) -> Element {
    let roles: Vec<Role> = Role::ALL.into_iter().filter(|r| *r != Role::Supervisor || supervisors_on).collect();
    rsx! {
        Field { label: "People per shift", error, help: needs.describe(),
            Grid { cols: 4, gap: Gap::Sm,
                for r in roles {
                    Field { key: "{r:?}", label: r.short_plural(),
                        TextInput {
                            kind: InputKind::Number,
                            value: needs.get(r).to_string(),
                            oninput: move |v: String| {
                                let n = v.trim().parse::<u8>().unwrap_or(0).min(9);
                                let mut x = needs;
                                x.set(r, n);
                                onchange.call(x);
                            },
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Presets and shift supervisors
// ---------------------------------------------------------------------------------------------

/// `/app/ministry/witnessing/presets`: the weekly shifts and the shift supervisors (planners).
#[component]
pub fn WitnessingPresetsPage() -> Element {
    let state = use_app_state();
    let navigator = navigator();
    let congregation = state.active_congregation().map(|c| c.name).unwrap_or_default();
    let mut editing = use_signal(|| None::<Option<u32>>);
    let back = ("Public witnessing".to_string(), EventHandler::new(move |_| { navigator.push(crate::Route::WitnessingPage {}); }));
    if !state.can_write(PAGE) {
        return rsx! {
            Page { title: "Presets", back,
                EmptyState { icon: Icon::Lock, title: "Only planners can change the presets", description: "Ask an administrator for write access to Public witnessing." }
            }
        };
    }
    rsx! {
        Page {
            title: "Presets",
            subtitle: "{congregation} · Public witnessing: the weekly shifts and the shift supervisors",
            back,
            actions: rsx! { ActionButton { action: Action::Add, label: "Add preset", onclick: move |_| editing.set(Some(None)) } },
            PresetsTab { on_open: move |id| editing.set(Some(Some(id))) }
        }
        if let Some(id) = editing() {
            PresetDialog { key: "{id:?}", id, onclose: move |_| editing.set(None) }
        }
    }
}

#[component]
fn PresetsTab(on_open: EventHandler<u32>) -> Element {
    let mut state = use_app_state();
    let prefs = state.preferences();
    let dir = state.directory();
    let witnesses = dir.witnesses().len();
    let first = week_order(&prefs)[0];
    let presets = dir.shift_presets_sorted(first);
    let mut choosing = use_signal(|| false);
    let settings = dir.witnessing.clone();
    rsx! {
        Text { tone: TextTone::Muted,
            "The usual shifts of a week. Fill month adds them to every matching day, and assigns only people who are ticked as available."
        }
        if witnesses == 0 {
            Alert { tone: Tone::Warning, title: "Nobody can take part yet",
                "Give the privilege \"Public witnessing\" in Users › Privileges."
            }
        }
        Card { title: "Shift supervisors", icon: Icon::Shield,
            Switch {
                checked: settings.supervisors_on,
                label: "Shifts have a supervisor",
                description: "A brother or sister responsible for the people of the shift. Each preset says how many.",
                onchange: move |on| state.update_directory(|d| d.set_supervisors_on(on)),
            }
            if settings.supervisors_on {
                Cluster { justify: Justify::Between,
                    Text { tone: TextTone::Small, "{settings.supervisors.len()} of {witnesses} can be a shift supervisor." }
                    ActionButton { action: Action::Edit, variant: ButtonVariant::Secondary, label: "Choose who", onclick: move |_| choosing.set(true) }
                }
            }
        }
        if presets.is_empty() {
            EmptyState { icon: Icon::Settings, title: "No presets yet", description: "For example: every Tuesday 09:00–11:00 at the market square, with 1 supervisor and 2 publishers." }
        } else {
            Grid { cols: 3, gap: Gap::Sm,
                for p in presets {
                    {
                        let id = p.id;
                        let can = dir.witnesses().iter().filter(|w| p.available.contains(&w.id)).count();
                        let mut tags = vec![
                            Tag::new(p.needs.describe(), Tone::Neutral).with_icon(Icon::Users),
                            Tag::new(format!("{can} of {witnesses} available"), if can < p.needs.total() as usize { Tone::Warning } else { Tone::Neutral }).with_icon(Icon::Check),
                        ];
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
        if choosing() {
            SupervisorsDialog { onclose: move |_| choosing.set(false) }
        }
    }
}

#[component]
fn SupervisorsDialog(onclose: EventHandler<()>) -> Element {
    let mut state = use_app_state();
    let toaster = use_toaster();
    let prefs = state.preferences();
    let dir = state.directory();
    let mut chosen = use_signal(|| dir.witnessing.supervisors.clone());
    let witnesses: Vec<&Person> = dir.witnesses();
    let rows: Vec<Row> = witnesses
        .iter()
        .map(|p| {
            let what = p.appointment.map(|a| a.label().to_string()).unwrap_or_else(|| p.publisher_type.label().to_string());
            Row::new(p.id.to_string(), p.display_name(&prefs)).with_subtitle(what)
        })
        .collect();
    let selected: BTreeSet<String> = chosen().iter().map(|p| p.to_string()).collect();
    rsx! {
        Dialog {
            open: true,
            onclose: move |_| onclose.call(()),
            title: "Shift supervisors",
            description: "Who can be a shift supervisor. Only people with the privilege \"Public witnessing\".",
            actions: rsx! {
                ActionButton { action: Action::Cancel, onclick: move |_| onclose.call(()) }
                ActionButton {
                    action: Action::Save,
                    onclick: move |_| {
                        state.update_directory(|d| d.set_supervisors(chosen()));
                        toaster.success("Shift supervisors saved.");
                        onclose.call(());
                    },
                }
            },
            Text { tone: TextTone::Small, "{chosen().len()} chosen." }
            PickList {
                rows,
                selected,
                on_toggle: move |pid: String| {
                    if let Ok(p) = pid.parse::<u32>() {
                        let mut c = chosen.write();
                        if !c.remove(&p) {
                            c.insert(p);
                        }
                    }
                },
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
    let supervisors_on = dir.witnessing.supervisors_on;
    let existing = id.and_then(|i| dir.shift_preset(i).cloned());
    let mut weekday = use_signal(|| existing.as_ref().map(|p| p.weekday).unwrap_or(2));
    let mut start = use_signal(|| existing.as_ref().map(|p| p.start.clone()).unwrap_or_else(|| "09:00".into()));
    let mut end = use_signal(|| existing.as_ref().map(|p| p.end.clone()).unwrap_or_else(|| "11:00".into()));
    let mut place = use_signal(|| existing.as_ref().map(|p| p.place.clone()).unwrap_or_default());
    let mut needs = use_signal(|| existing.as_ref().map(|p| p.needs).unwrap_or(Needs { supervisors: u8::from(supervisors_on), anyone: 2, ..Default::default() }));
    let mut available = use_signal(|| existing.as_ref().map(|p| p.available.clone()).unwrap_or_default());
    let mut enabled = use_signal(|| existing.as_ref().is_none_or(|p| p.enabled));
    let mut errors = use_signal(Vec::<(&'static str, String)>::new);
    let mut confirm = use_signal(|| false);
    let err = move |f: &str| errors.read().iter().find(|(k, _)| *k == f).map(|(_, m)| m.clone());

    let days: Vec<SelectOption> = week_order(&prefs).into_iter().map(|w| SelectOption::new(w.to_string(), WEEKDAYS[w as usize])).collect();
    let witnesses: Vec<&Person> = dir.witnesses();
    let rows: Vec<Row> = witnesses
        .iter()
        .map(|p| {
            let mut r = Row::new(p.id.to_string(), p.display_name(&prefs)).with_subtitle(p.gender.label());
            if supervisors_on && dir.witnessing.supervisors.contains(&p.id) {
                r = r.with_tag(Tag::new("Supervisor", Tone::Neutral).with_icon(Icon::Shield));
            }
            r
        })
        .collect();
    let ticked: BTreeSet<String> = witnesses.iter().filter(|p| available().contains(&p.id)).map(|p| p.id.to_string()).collect();
    let (can, all) = (ticked.len(), witnesses.len());
    let save = move |_| {
        match state.update_directory(|d| d.save_shift_preset(id, weekday(), &start(), &end(), &place(), needs(), available(), enabled())) {
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
            title: if existing.is_some() { "Weekly shift" } else { "Add preset" },
            description: "A public witnessing shift that happens every week.",
            actions: rsx! {
                EditActions {
                    on_save: save,
                    on_cancel: move |_| onclose.call(()),
                    on_delete: existing.is_some().then_some(EventHandler::new(move |_| confirm.set(true))),
                }
            },
            Stack {
                Grid { cols: 3,
                    Field { label: "Day", error: err("weekday"),
                        Select { value: weekday().to_string(), options: days, onchange: move |v: String| weekday.set(v.parse().unwrap_or(2)) }
                    }
                    Field { label: "Start", required: true, error: err("start"),
                        TextInput { kind: InputKind::Time, value: start(), oninput: move |v| start.set(v) }
                    }
                    Field { label: "End", required: true, error: err("end"),
                        TextInput { kind: InputKind::Time, value: end(), oninput: move |v| end.set(v) }
                    }
                }
                Field { label: "Place", required: true, error: err("place"),
                    TextInput { value: place(), placeholder: "For example: Market square", oninput: move |v| place.set(v) }
                }
                NeedsEditor { needs: needs(), supervisors_on, error: err("needs"), onchange: move |n| needs.set(n) }
                Switch {
                    checked: enabled(),
                    label: "On",
                    description: "Off: Fill month skips it, for example for a few weeks. Shifts already made stay.",
                    onchange: move |v| enabled.set(v),
                }
                Field { label: "Who is available", help: format!("{can} of {all} with the privilege. Tick who can take part at this time: only they are assigned."),
                    if rows.is_empty() {
                        Text { tone: TextTone::Muted, "Nobody has the privilege \"Public witnessing\" yet." }
                    } else {
                        PickList {
                            rows,
                            selected: ticked,
                            on_toggle: move |pid: String| {
                                if let Ok(p) = pid.parse::<u32>() {
                                    let mut a = available.write();
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
            message: "Shifts already made from it stay. Fill month no longer adds it.",
            confirm: Action::Delete,
            oncancel: move |_| confirm.set(false),
            onconfirm: move |_| {
                if let Some(i) = id {
                    state.update_directory(|d| d.delete_shift_preset(i));
                }
                toaster.success("Preset deleted.");
                onclose.call(());
            },
        }
    }
}

// ---------------------------------------------------------------------------------------------
// One shift
// ---------------------------------------------------------------------------------------------

#[component]
fn ShiftDialog(id: Option<u32>, month: Ym, onclose: EventHandler<()>) -> Element {
    let mut state = use_app_state();
    let toaster = use_toaster();
    let prefs = state.preferences();
    let dir = state.directory();
    let supervisors_on = dir.witnessing.supervisors_on;
    let existing = id.and_then(|i| dir.shift(i).cloned());
    let default_date = if month == this_month() { people::today_iso() } else { format!("{:04}-{:02}-01", month.0, month.1) };
    let mut draft = use_signal(|| match &existing {
        Some(s) => ShiftDraft { date: s.date.clone(), start: s.start.clone(), end: s.end.clone(), place: s.place.clone(), preset: s.preset, slots: s.slots.clone() },
        None => ShiftDraft {
            date: default_date.clone(),
            start: "09:00".into(),
            end: "11:00".into(),
            place: String::new(),
            preset: None,
            slots: Vec::new(),
        },
    });
    let mut picking = use_signal(|| None::<usize>);
    // Places that were there before this dialog opened (a place added here and not filled goes away).
    let existing_places = existing.as_ref().map(|s| s.slots.len()).unwrap_or(0);
    let mut errors = use_signal(Vec::<(&'static str, String)>::new);
    let mut confirm = use_signal(|| false);
    let err = move |f: &str| errors.read().iter().find(|(k, _)| *k == f).map(|(_, m)| m.clone());
    let d = draft();

    let first = week_order(&prefs)[0];
    let mut preset_options = vec![SelectOption::new("", "None: a one-off shift")];
    preset_options.extend(dir.shift_presets_sorted(first).iter().map(|p| SelectOption::new(p.id.to_string(), format!("{} · {}", p.label(), p.place))));

    // The places: who, and whether they fit.
    let rows: Vec<Row> = d
        .slots
        .iter()
        .enumerate()
        .map(|(i, s)| match s.person.and_then(|p| dir.person(p)) {
            Some(p) => {
                let mut r = Row::new(i.to_string(), p.display_name(&prefs)).with_subtitle(s.role.label()).with_action();
                if let Some(t) = fit_tag(dir.shift_fit(p.id, s.role, &d.date, d.preset, id)).filter(|t| t.tone != Tone::Success) {
                    r = r.with_tag(t);
                }
                r
            }
            None => Row::new(i.to_string(), format!("Choose a {}", s.role.label().to_lowercase())).with_subtitle("Empty"),
        })
        .collect();
    let status = existing.as_ref().map(|s| s.status());
    let no_supervisor = supervisors_on && !d.slots.is_empty() && !d.slots.iter().any(|s| s.role == Role::Supervisor && s.person.is_some());

    let save = move |_| {
        let d = draft();
        match state.update_directory(|dir| dir.save_shift(id, &d)) {
            Ok(_) => {
                toaster.success(if id.is_some() { "Shift saved. Send it when the month is ready." } else { "Shift added. Send it when the month is ready." });
                onclose.call(());
            }
            Err(e) => errors.set(e),
        }
    };

    rsx! {
        Dialog {
            open: true,
            onclose: move |_| onclose.call(()),
            title: if existing.is_some() { "Public witnessing shift" } else { "Add shift" },
            description: "When and where, and who takes part.",
            actions: rsx! {
                EditActions {
                    on_save: save,
                    on_cancel: move |_| onclose.call(()),
                    on_delete: existing.as_ref().is_some_and(|s| !s.removed).then_some(EventHandler::new(move |_| confirm.set(true))),
                }
            },
            Stack {
                match status {
                    Some(SendStatus::Changed) => rsx! { Alert { tone: Tone::Warning, title: "Changed, not sent yet", "Everyone still sees the version you sent until you send the month." } },
                    Some(SendStatus::Removed) => rsx! {
                        Alert { tone: Tone::Error, title: "Removed, not sent yet",
                            Stack { gap: Gap::Sm,
                                Text { "Everyone still sees this shift until you send the month." }
                                Cluster {
                                    ActionButton {
                                        action: Action::Undo,
                                        label: "Keep it",
                                        onclick: move |_| {
                                            if let Some(i) = id {
                                                state.update_directory(|d| d.keep_shift(i));
                                            }
                                            toaster.success("The shift is kept.");
                                            onclose.call(());
                                        },
                                    }
                                }
                            }
                        }
                    },
                    Some(SendStatus::NotSent) => rsx! { Text { tone: TextTone::Small, "Not sent yet: only planners see it." } },
                    _ => rsx! {},
                }
                Field { label: "Preset",
                    Select {
                        value: d.preset.map(|p| p.to_string()).unwrap_or_default(),
                        options: preset_options,
                        onchange: move |v: String| {
                            let p = v.parse::<u32>().ok();
                            let preset = p.and_then(|p| state.directory().shift_preset(p).cloned());
                            let mut w = draft.write();
                            w.preset = p;
                            if let Some(s) = preset {
                                w.start = s.start;
                                w.end = s.end;
                                w.place = s.place;
                                w.slots = resize_slots(&w.slots, s.needs);
                            }
                        },
                    }
                }
                Grid { cols: 3,
                    Field { label: "Date", required: true, error: err("date"),
                        TextInput { kind: InputKind::Date, value: d.date.clone(), oninput: move |v| draft.write().date = v }
                    }
                    Field { label: "Start", required: true, error: err("start"),
                        TextInput { kind: InputKind::Time, value: d.start.clone(), oninput: move |v| draft.write().start = v }
                    }
                    Field { label: "End", required: true, error: err("end"),
                        TextInput { kind: InputKind::Time, value: d.end.clone(), oninput: move |v| draft.write().end = v }
                    }
                }
                Field { label: "Place", required: true, error: err("place"),
                    TextInput { value: d.place.clone(), placeholder: "For example: Market square", oninput: move |v| draft.write().place = v }
                }
                Field { label: "Who takes part", error: err("slots"), help: "Tap someone to change them. × takes them out (an empty place goes away). How many of each kind a shift needs is set in its preset.",
                    if !rows.is_empty() {
                        RowList {
                            rows,
                            on_open: move |i: String| picking.set(i.parse().ok()),
                            action: (Icon::Close, "Remove".to_string()),
                            on_action: move |i: String| {
                                if let Ok(i) = i.parse::<usize>() {
                                    let mut w = draft.write();
                                    match w.slots.get(i).map(|s| s.person.is_some()) {
                                        Some(true) => w.slots[i].person = None,
                                        Some(false) => {
                                            w.slots.remove(i);
                                        }
                                        None => {}
                                    }
                                }
                            },
                        }
                    }
                }
                Cluster { gap: Gap::Sm,
                    ActionButton {
                        action: Action::Add,
                        variant: ButtonVariant::Secondary,
                        label: "Add person",
                        onclick: move |_| {
                            let mut w = draft.write();
                            w.slots.push(Slot { role: Role::Anyone, person: None });
                            picking.set(Some(w.slots.len() - 1));
                        },
                    }
                    if supervisors_on {
                        ActionButton {
                            action: Action::Add,
                            variant: ButtonVariant::Secondary,
                            label: "Add supervisor",
                            onclick: move |_| {
                                let mut w = draft.write();
                                w.slots.insert(0, Slot { role: Role::Supervisor, person: None });
                                picking.set(Some(0));
                            },
                        }
                    }
                }
                if no_supervisor {
                    Alert { tone: Tone::Warning, title: "No shift supervisor",
                        "Shift supervisors are on, but nobody in this shift is one. Add a supervisor, or save it as it is."
                    }
                }
            }
        }
        if let Some(i) = picking() {
            PersonPicker {
                key: "{i}",
                draft: draft(),
                shift: id,
                index: i,
                onpick: move |p: Option<u32>| {
                    if let Some(s) = draft.write().slots.get_mut(i) {
                        s.person = p;
                    }
                    picking.set(None);
                },
                onclose: move |_| {
                    let mut w = draft.write();
                    if w.slots.get(i).is_some_and(|s| s.person.is_none()) && w.slots.len() > existing_places {
                        w.slots.remove(i);
                    }
                    picking.set(None);
                },
            }
        }
        ConfirmDialog {
            open: confirm(),
            title: "Delete this shift?",
            message: if existing.as_ref().is_some_and(|s| s.sent.is_some()) { "It was sent: everyone still sees it until you send the month." } else { "It was not sent, so nobody else saw it." },
            confirm: Action::Delete,
            oncancel: move |_| confirm.set(false),
            onconfirm: move |_| {
                if let Some(i) = id {
                    state.update_directory(|d| d.delete_shift(i));
                }
                toaster.success("Shift deleted.");
                onclose.call(());
            },
        }
    }
}

/// Choose the person for one place: who fits first (with whether they are available), then the
/// other baptised publishers of the right gender, for when nobody fits.
#[component]
fn PersonPicker(draft: ShiftDraft, shift: Option<u32>, index: usize, onpick: EventHandler<Option<u32>>, onclose: EventHandler<()>) -> Element {
    let state = use_app_state();
    let prefs = state.preferences();
    let dir = state.directory();
    let mut query = use_signal(String::new);
    let Some(slot) = draft.slots.get(index).copied() else { return rsx! {} };
    let taken: BTreeSet<u32> = draft.slots.iter().enumerate().filter(|(i, _)| *i != index).filter_map(|(_, s)| s.person).collect();
    let tokens: Vec<String> = people::fold(&query()).split_whitespace().map(str::to_string).collect();
    let found = |p: &Person| {
        let hay = people::fold(&format!("{} {}", p.first_name, p.last_name));
        tokens.iter().all(|t| hay.contains(t.as_str()))
    };
    let order = |f: Fit| match f {
        Fit::Available => 0,
        Fit::NotForThisSlot | Fit::BusyThatDay => 1,
        Fit::Away => 2,
        _ => 3,
    };
    let fits_role = |f: Fit| !matches!(f, Fit::WrongGender | Fit::NotSupervisor);
    let mut with: Vec<(&Person, Fit)> = dir
        .witnesses()
        .into_iter()
        .filter(|p| !taken.contains(&p.id) && found(p))
        .map(|p| (p, dir.shift_fit(p.id, slot.role, &draft.date, draft.preset, shift)))
        .filter(|(_, f)| fits_role(*f))
        .collect();
    with.sort_by_cached_key(|(p, f)| (order(*f), people::fold(&p.display_name(&prefs))));
    let privileged: BTreeSet<u32> = dir.witnesses().iter().map(|p| p.id).collect();
    let gender_ok = |p: &Person| match slot.role {
        Role::Man => p.gender == crate::people::Gender::Male,
        Role::Woman => p.gender == crate::people::Gender::Female,
        _ => true,
    };
    let mut others: Vec<&Person> = if slot.role == Role::Supervisor {
        Vec::new()
    } else {
        dir.people
            .iter()
            .filter(|p| p.moved_away.is_none() && p.publisher_type.is_baptised() && !privileged.contains(&p.id) && !taken.contains(&p.id) && gender_ok(p) && found(p))
            .collect()
    };
    others.sort_by_cached_key(|p| people::fold(&p.display_name(&prefs)));
    let row = |p: &Person, f: Fit| {
        let mut r = Row::new(p.id.to_string(), p.display_name(&prefs));
        if let Some(t) = fit_tag(f) {
            r = r.with_tag(t);
        }
        r
    };
    let groups = vec![
        RowGroup { title: if slot.role == Role::Supervisor { "Shift supervisors".into() } else { "Public witnessing".into() }, rows: with.iter().map(|(p, f)| row(p, *f)).collect() },
        RowGroup {
            title: "Other baptised publishers".into(),
            rows: others
                .iter()
                .map(|p| {
                    let f = if dir.is_away(p.id, &draft.date) { Fit::Away } else { Fit::NoPrivilege };
                    row(p, f).with_subtitle(p.publisher_type.label())
                })
                .collect(),
        },
    ];
    rsx! {
        Dialog {
            open: true,
            onclose: move |_| onclose.call(()),
            title: "Choose a {slot.role.label().to_lowercase()}",
            description: "{day_title(&draft.date)} · {draft.start}–{draft.end}",
            actions: rsx! { ActionButton { action: Action::Cancel, onclick: move |_| onclose.call(()) } },
            Stack {
                TextInput { kind: InputKind::Search, value: query(), icon: Icon::Search, placeholder: "Search by name", oninput: move |v| query.set(v) }
                ChoiceList {
                    groups,
                    selected: slot.person.map(|p| p.to_string()),
                    none_label: "Nobody (leave empty)",
                    on_choose: move |c: Option<String>| onpick.call(c.and_then(|c| c.parse().ok())),
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
    let months: i64 = if period() == "12" { 12 } else { 6 };
    let today = people::today_iso();
    let from = crate::events::add_days(&today, -months * 30).unwrap_or_else(|| today.clone());
    let (shifts, filled, places) = dir.shift_totals(&from, &today);
    let per_person = dir.shifts_per_person(&from, &today);
    let witnesses = per_person.len();
    let took_part = per_person.iter().filter(|r| r.shifts > 0).count();
    let none: Vec<String> = per_person.iter().filter(|r| r.shifts == 0).filter_map(|r| dir.person(r.person)).map(|p| p.display_name(&prefs)).collect();
    let top = per_person.first().map(|r| r.shifts).unwrap_or(0).max(1);
    let fill_pct = (filled * 100).checked_div(places).unwrap_or(0);
    let first = week_order(&prefs)[0];
    // Presets by how many can take part, fewest first.
    let mut presets: Vec<(String, String, usize, u8)> = dir
        .shift_presets_sorted(first)
        .iter()
        .map(|p| (p.label(), p.place.clone(), dir.witnesses().iter().filter(|w| p.available.contains(&w.id)).count(), p.needs.total()))
        .collect();
    presets.sort_by_key(|p| p.2);
    let busiest = per_person.first().filter(|r| r.shifts > 0).and_then(|r| dir.person(r.person).map(|p| (p.display_name(&prefs), r.shifts)));

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
            Card { StatTile { label: "Shifts", value: shifts.to_string(), icon: Icon::PublicWitnessing, delta: "sent shifts".to_string() } }
            Card { StatTile { label: "Places filled", value: format!("{fill_pct}%"), icon: Icon::Check, delta: format!("{filled} of {places}") } }
            Card { StatTile { label: "Took part", value: format!("{took_part} of {witnesses}"), icon: Icon::Users, delta: "with the privilege".to_string() } }
            Card { StatTile { label: "Shifts per person", value: if witnesses > 0 { format!("{:.1}", filled as f64 / witnesses as f64) } else { "–".into() }, icon: Icon::User, delta: "on average".to_string() } }
        }
        Grid { cols: 2,
            Card { title: "Shifts per person", icon: Icon::Users,
                if per_person.is_empty() {
                    Text { tone: TextTone::Muted, "Nobody has the privilege yet." }
                } else {
                    Stack { gap: Gap::Sm,
                        for r in per_person.iter() {
                            {
                                let name = dir.person(r.person).map(|p| p.display_name(&prefs)).unwrap_or_default();
                                let note = (r.as_supervisor > 0).then(|| format!("{} as shift supervisor", r.as_supervisor));
                                rsx! {
                                    ProgressRow {
                                        key: "{r.person}",
                                        label: name,
                                        detail: if r.shifts == 1 { "1 shift".to_string() } else { format!("{} shifts", r.shifts) },
                                        value: r.shifts as f64,
                                        max: top as f64,
                                        note,
                                        tone: if r.shifts == 0 { Tone::Warning } else { Tone::Neutral },
                                    }
                                }
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
                            for (i, (label, place, can, need)) in presets.iter().enumerate() {
                                ProgressRow {
                                    key: "{i}",
                                    label: label.clone(),
                                    detail: format!("{can} of {witnesses} available"),
                                    value: *can as f64,
                                    max: witnesses.max(1) as f64,
                                    note: if (*can as u8) < *need { format!("{place} · not enough for one shift") } else { place.clone() },
                                    tone: if (*can as u8) < *need { Tone::Error } else if (*can as u8) < need * 2 { Tone::Warning } else { Tone::Success },
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
                                items.push(ListEntry { icon: Icon::Warning, title: format!("Fewest available: {}", low.0), subtitle: format!("{} · {} of {witnesses}", low.1, low.2), meta: String::new() });
                                if presets.len() > 1 {
                                    items.push(ListEntry { icon: Icon::Check, title: format!("Most available: {}", high.0), subtitle: format!("{} · {} of {witnesses}", high.1, high.2), meta: String::new() });
                                }
                            }
                            if let Some((name, n)) = &busiest {
                                items.push(ListEntry { icon: Icon::User, title: format!("Most shifts: {name}"), subtitle: format!("{n} shifts in the last {months} months"), meta: String::new() });
                            }
                            if !none.is_empty() {
                                items.push(ListEntry {
                                    icon: Icon::Users,
                                    title: format!("{} without a shift", none.len()),
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
