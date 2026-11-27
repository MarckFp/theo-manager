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

/// `/app/settings`: details, meeting times, display defaults, backup and the activity log of the
/// open congregation. Opened with the cog in the congregation switcher.
#[component]
pub fn SettingsPage() -> Element {
    let mut state = use_app_state();
    let toaster = use_toaster();
    let mut editing = use_signal(|| false);
    let Some(c) = state.active_congregation() else { return rsx! {} };
    let p = state.preferences();
    // Administrators, and whoever may write Congregation settings.
    let can_edit = state.can_write("settings");
    let (mode_title, mode_sub) = match &c.mode {
        StorageMode::Local => ("Local".to_string(), "Stored only on this device".to_string()),
        StorageMode::Cloud { server } => ("Cloud".to_string(), server.clone()),
    };
    rsx! {
        Page { title: "Settings", subtitle: "{c.name} · Congregation settings",
            Grid { cols: 2,
                Card { title: "Details", icon: Icon::Congregation,
                    actions: if can_edit { Some(rsx! { ActionButton { action: Action::Edit, variant: ButtonVariant::Ghost, onclick: move |_| editing.set(true) } }) } else { None },
                    ItemList {
                        items: vec![
                            ListEntry { icon: Icon::Congregation, title: c.name.clone(), subtitle: if c.number.is_empty() { "No number".into() } else { format!("Number {}", c.number) }, meta: c.language.to_uppercase() },
                            ListEntry { icon: Icon::Clock, title: c.time_zone.clone(), subtitle: "Time zone".into(), meta: String::new() },
                            ListEntry { icon: c.mode.icon(), title: mode_title, subtitle: mode_sub, meta: String::new() },
                        ],
                    }
                }
                Card { title: "Meetings", icon: Icon::Meetings,
                    actions: if can_edit { Some(rsx! { ActionButton { action: Action::Edit, variant: ButtonVariant::Ghost, onclick: move |_| editing.set(true) } }) } else { None },
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
            if state.can_manage_data() {
                DataCard {}
            }
            if editing() {
                EditDetailsDialog { onclose: move |_| editing.set(false) }
            }
            if state.manages_cloud() {
                ServerLogCard {}
            }
            {
                let log = state.directory().log;
                rsx! {
                    Card { title: "Activity log", icon: Icon::History,
                        if log.is_empty() {
                            Text { tone: TextTone::Muted, "No changes recorded yet. Permission changes are recorded here; more kinds of changes follow." }
                        } else {
                            ItemList {
                                items: log.iter().rev().map(|e| ListEntry {
                                    icon: if e.text.starts_with("Permissions") { Icon::Permission } else { Icon::History },
                                    title: e.text.clone(),
                                    subtitle: format!("{} · {} UTC", e.actor, e.at),
                                    meta: String::new(),
                                }).collect(),
                            }
                        }
                    }
                }
            }
        }
    }
}

/// One line of the server's audit log in words: (icon, text).
fn describe_audit(e: &theo_db::surreal::AuditEntry) -> (Icon, String) {
    let target = e.target.clone().unwrap_or_default();
    let verb = |created: &str, changed: &str, deleted: &str| match e.action.as_str() {
        "create" => created.to_string(),
        "delete" => deleted.to_string(),
        _ => changed.to_string(),
    };
    match e.what.as_str() {
        "record" => {
            let collection = target.split(':').next().unwrap_or_default().replace('_', " ");
            (Icon::Edit, format!("{} a record in {collection}", verb("Added", "Changed", "Deleted")))
        }
        "member" => (Icon::User, verb(&format!("{target} joined"), &format!("Membership of {target} changed"), &format!("{target} left or was removed"))),
        "invite" if e.action == "use" => (Icon::Invitation, format!("An invite was used ({target})")),
        "invite" => (Icon::Invitation, verb(&format!("Invite made ({target})"), "Invite changed", "Invite cancelled or removed")),
        "password" => (Icon::Key, verb("Congregation password set", "Congregation password changed", "Congregation password removed")),
        "key" => (Icon::Key, verb(&format!("Key {target} made"), &format!("Key {target} changed"), &format!("Key {target} removed"))),
        "grant" => (Icon::Permission, verb(&format!("Keys given: {target}"), &format!("Keys changed: {target}"), &format!("Keys taken back: {target}"))),
        other => (Icon::History, format!("{} {other}", e.action)),
    }
}

/// The server's audit log of a Cloud congregation (R-227): who changed what, and when. Written by
/// the server itself, so nobody can change it; it holds no content, only metadata.
#[component]
fn ServerLogCard() -> Element {
    let state = use_app_state();
    let mut shown = use_signal(|| PAGE);
    let log = use_resource(move || async move { state.audit_log(0, shown()).await });
    rsx! {
        Card { title: "Server log", icon: Icon::Server,
            Text { tone: TextTone::Muted, "Kept by the server: who changed what, and when. It never shows the content, which only members can decrypt." }
            match &*log.read() {
                None => rsx! { Text { tone: TextTone::Muted, "Loading…" } },
                Some(Err(e)) => rsx! { Alert { tone: Tone::Error, "The server log could not be read: {e}" } },
                Some(Ok(entries)) if entries.is_empty() => rsx! { Text { tone: TextTone::Muted, "Nothing recorded yet." } },
                Some(Ok(entries)) => rsx! {
                    Text { tone: TextTone::Small, "Lines older than a year are removed." }
                    ItemList {
                        items: entries.iter().map(|e| {
                            let (icon, title) = describe_audit(e);
                            let at = e.at.trim_start_matches("d'").trim_end_matches('\'').replace('T', " ");
                            ListEntry {
                                icon,
                                title,
                                subtitle: format!("@{} · {} UTC", e.actor.clone().unwrap_or_else(|| "server".into()), at.get(..16).unwrap_or(&at)),
                                meta: String::new(),
                            }
                        }).collect(),
                    }
                    if entries.len() as u32 >= shown() {
                        Cluster { gap: Gap::Sm,
                            ActionButton { action: Action::View, variant: ButtonVariant::Ghost, label: "Show more", onclick: move |_| shown += PAGE }
                        }
                    }
                },
            }
        }
    }
}

/// Lines of the server log per page.
const PAGE: u32 = 50;

/// The congregation's details: name, number, language, time zone and meeting times. Saved like
/// all its data (encrypted); a Cloud congregation's name is also renamed for this account.
#[component]
fn EditDetailsDialog(onclose: EventHandler<()>) -> Element {
    let mut state = use_app_state();
    let toaster = use_toaster();
    let c = state.active_congregation();
    let start = c.clone();
    let mut name = use_signal(|| start.as_ref().map(|c| c.name.clone()).unwrap_or_default());
    let mut number = use_signal(|| start.as_ref().map(|c| c.number.clone()).unwrap_or_default());
    let mut language = use_signal(|| start.as_ref().map(|c| c.language.clone()).unwrap_or_else(|| "en".into()));
    let mut tz = use_signal(|| start.as_ref().map(|c| c.time_zone.clone()).unwrap_or_else(|| "UTC".into()));
    let mut mid_day = use_signal(|| start.as_ref().map(|c| c.midweek.day.clone()).unwrap_or_else(|| "Thursday".into()));
    let mut mid_time = use_signal(|| start.as_ref().map(|c| c.midweek.time.clone()).unwrap_or_else(|| "19:00".into()));
    let mut wk_day = use_signal(|| start.as_ref().map(|c| c.weekend.day.clone()).unwrap_or_else(|| "Sunday".into()));
    let mut wk_time = use_signal(|| start.as_ref().map(|c| c.weekend.time.clone()).unwrap_or_else(|| "10:00".into()));
    let mut submitted = use_signal(|| false);
    let s = submitted();
    let name_err = (s && name().trim().is_empty()).then(|| "Enter the congregation name.".to_string());
    let time_ok = |t: &str| t.len() == 5 && t.as_bytes()[2] == b':' && t[..2].parse::<u8>().is_ok_and(|h| h < 24) && t[3..].parse::<u8>().is_ok_and(|m| m < 60);
    let mid_err = (s && !time_ok(&mid_time())).then(|| "Enter a time, for example 19:00.".to_string());
    let wk_err = (s && !time_ok(&wk_time())).then(|| "Enter a time, for example 10:00.".to_string());
    // A time zone not in the list (set elsewhere) is kept as a choice.
    let mut zones = super::wizard::time_zones();
    if !zones.iter().any(|o| o.value == tz()) {
        zones.insert(0, SelectOption::new(tz(), tz()));
    }
    let save = move |_| {
        submitted.set(true);
        if name().trim().is_empty() || !time_ok(&mid_time()) || !time_ok(&wk_time()) {
            return;
        }
        state.update_details(|c| {
            c.name = name().trim().to_string();
            c.number = number().trim().to_string();
            c.language = language();
            c.time_zone = tz();
            c.midweek = crate::state::MeetingSlot { day: mid_day(), time: mid_time() };
            c.weekend = crate::state::MeetingSlot { day: wk_day(), time: wk_time() };
        });
        toaster.success("Congregation details saved.");
        onclose.call(());
    };
    rsx! {
        Dialog {
            open: true,
            onclose: move |_| onclose.call(()),
            title: "Congregation details",
            description: "Everyone in the congregation sees these.",
            actions: rsx! {
                ActionButton { action: Action::Cancel, onclick: move |_| onclose.call(()) }
                ActionButton { action: Action::Save, onclick: save }
            },
            Stack {
                Field { label: "Congregation name", required: true, error: name_err.clone(),
                    TextInput { value: name(), oninput: move |v| name.set(v), invalid: name_err.is_some() }
                }
                Field { label: "Congregation number", help: "Optional. The number your congregation uses, if any.",
                    TextInput { value: number(), oninput: move |v| number.set(v) }
                }
                Grid { cols: 2,
                    Field { label: "Meeting language", required: true, help: "Used to import meeting content.",
                        Select { value: language(), options: super::wizard::languages(), onchange: move |v| language.set(v) }
                    }
                    Field { label: "Time zone", required: true,
                        Select { value: tz(), options: zones, onchange: move |v| tz.set(v) }
                    }
                    Field { label: "Midweek meeting", required: true,
                        Select { value: mid_day(), options: super::wizard::weekdays(), onchange: move |v| mid_day.set(v) }
                    }
                    Field { label: "Time", required: true, error: mid_err.clone(),
                        TextInput { value: mid_time(), oninput: move |v| mid_time.set(v), kind: InputKind::Time, icon: Icon::Clock, invalid: mid_err.is_some() }
                    }
                    Field { label: "Weekend meeting", required: true,
                        Select { value: wk_day(), options: super::wizard::weekend_days(), onchange: move |v| wk_day.set(v) }
                    }
                    Field { label: "Time", required: true, error: wk_err.clone(),
                        TextInput { value: wk_time(), oninput: move |v| wk_time.set(v), kind: InputKind::Time, icon: Icon::Clock, invalid: wk_err.is_some() }
                    }
                }
            }
        }
    }
}

/// Export, import, and where the congregation lives (D-51). For those who may manage its data:
/// the user of a Local congregation, the owner and full-access members of a Cloud one.
#[component]
fn DataCard() -> Element {
    let state = use_app_state();
    let toaster = use_toaster();
    let mut exporting = use_signal(|| false);
    let mut importing = use_signal(|| false);
    let mut moving = use_signal(|| false);
    // Only the owner moves a Cloud congregation to a device (it ends everyone else's access).
    let owner = use_resource(move || async move { state.owns_cloud_congregation().await });
    let Some(c) = state.active_congregation() else { return rsx! {} };
    let cloud = matches!(c.mode, StorageMode::Cloud { .. });
    let can_move = c.stored && if cloud { owner().unwrap_or(false) } else { crate::cloud::available() };
    let name = c.name.clone();
    rsx! {
        Card { title: "Export, import and storage", icon: Icon::Download,
            Text { tone: TextTone::Muted,
                "An export is one small file with all the data of {name}: people, meetings, duties, talks, reports, territories, permissions and settings. It is always encrypted, with a password you choose when exporting. Keep it in a safe place."
            }
            Cluster { gap: Gap::Sm,
                ActionButton { action: Action::CreateBackup, label: "Export", variant: ButtonVariant::Secondary, onclick: move |_| exporting.set(true) }
                ActionButton { action: Action::RestoreBackup, label: "Import", variant: ButtonVariant::Ghost, onclick: move |_| importing.set(true) }
            }
            if c.stored {
                Divider {}
                Text { tone: TextTone::Muted,
                    if cloud { "Stored in the Cloud: encrypted on the Theo Manager servers, for every member." } else { "Stored on this device only, encrypted." }
                }
                if can_move {
                    Cluster { gap: Gap::Sm,
                        ActionButton {
                            action: if cloud { Action::MoveToDevice } else { Action::MoveToCloud },
                            variant: ButtonVariant::Secondary,
                            onclick: move |_| moving.set(true),
                        }
                    }
                } else if cloud {
                    Text { tone: TextTone::Small, "Only the owner can move it to a device." }
                }
            }
        }
        if exporting() {
            ExportDialog { onclose: move |_| exporting.set(false) }
        }
        if importing() {
            RestoreBackupDialog {
                onclose: move |_| importing.set(false),
                ondone: move |people: usize| {
                    importing.set(false);
                    toaster.success(format!("Imported: {people} people. Everything else was replaced too."));
                },
            }
        }
        if moving() {
            MoveDialog { to_cloud: !cloud, onclose: move |_| moving.set(false) }
        }
    }
}

/// Exports the open congregation: always encrypted, with a generated passphrase (default) or a
/// strong password of your own. Not the congregation password: the file may be handed on.
#[component]
fn ExportDialog(onclose: EventHandler<()>) -> Element {
    let mut state = use_app_state();
    let toaster = use_toaster();
    let mut own = use_signal(|| "generated".to_string());
    let mut generated = use_signal(crate::password::generate);
    let mut saved = use_signal(|| false);
    let mut custom = use_signal(String::new);
    let mut custom2 = use_signal(String::new);
    let mut error = use_signal(|| None::<String>);
    let mut busy = use_signal(|| false);
    let name = state.active_congregation().map(|c| c.name).unwrap_or_default();
    let mine = own() == "own";
    let score = crate::password::strength(&custom(), &[name.as_str()]);
    let ready = if mine { score >= 3 && custom2() == custom() } else { saved() && !generated().is_empty() };
    let export = move |_| {
        if !ready {
            error.set(Some(if mine { "Use a strong password (at least \u{201c}Strong\u{201d}) and type it twice.".into() } else { "Confirm that you wrote the password down.".into() }));
            return;
        }
        let password = if mine { custom() } else { generated() };
        busy.set(true);
        spawn(async move {
            // Let the spinner appear before the (deliberately slow) key derivation.
            theo_ui::sleep(std::time::Duration::from_millis(60)).await;
            let Some(backup) = state.backup() else {
                busy.set(false);
                return;
            };
            match crate::backup::seal(&backup, &password, theo_crypto::KdfParams::DEFAULT) {
                Ok(bytes) => {
                    let file = format!("{}-{}.{}", crate::people::fold(&backup.details.name).replace(' ', "-"), &backup.created[..10], crate::backup::EXTENSION);
                    match crate::platform::save_file(&file, "application/octet-stream", &bytes) {
                        Ok(msg) => {
                            state.log("Exported the congregation.");
                            toaster.success(msg);
                            onclose.call(());
                        }
                        Err(msg) => error.set(Some(msg)),
                    }
                }
                Err(msg) => error.set(Some(msg)),
            }
            busy.set(false);
        });
    };
    let choices = vec![
        Segment { value: "generated".into(), label: "Generated password".into(), icon: Some(Icon::Magic) },
        Segment { value: "own".into(), label: "My own password".into(), icon: Some(Icon::Key) },
    ];
    rsx! {
        Dialog {
            open: true,
            onclose: move |_| onclose.call(()),
            title: "Export {name}",
            description: "One encrypted file with all the data. Import it later here, in another congregation, or after moving between this device and the Cloud.",
            actions: rsx! {
                ActionButton { action: Action::Cancel, onclick: move |_| onclose.call(()) }
                ActionButton { action: Action::CreateBackup, label: "Export", loading: busy(), onclick: export }
            },
            Stack {
                if let Some(e) = error() {
                    Alert { tone: Tone::Error, "{e}" }
                }
                Segmented { value: own(), options: choices, onchange: move |v| { own.set(v); error.set(None); } }
                if mine {
                    Field { label: "Password of the file", required: true,
                        PasswordInput { value: custom(), oninput: move |v| { custom.set(v); error.set(None); }, autocomplete: "new-password", strength: Some(score) }
                    }
                    Field { label: "Repeat the password", required: true,
                        PasswordInput { value: custom2(), oninput: move |v| { custom2.set(v); error.set(None); }, autocomplete: "new-password" }
                    }
                } else {
                    Field { label: "Password of the file",
                        Code { "{generated}" }
                    }
                    Cluster { gap: Gap::Sm,
                        super::CopyButton { text: generated() }
                        ActionButton { action: Action::Regenerate, variant: ButtonVariant::Secondary, onclick: move |_| { generated.set(crate::password::generate()); saved.set(false); } }
                    }
                    Switch { checked: saved(), onchange: move |v| { saved.set(v); error.set(None); }, label: "I have written this password down", description: "It is shown only now." }
                }
                Text { tone: TextTone::Small, "Without this password nobody can open the file, not even you. It is not the congregation password, so the file can be handed on without giving that away. Encrypting takes a few seconds on purpose." }
            }
        }
    }
}

/// Moves the open congregation between this device and the Cloud (D-51): explains what happens,
/// asks for the account to move it to, and for a Cloud congregation that everybody else loses
/// access.
#[component]
fn MoveDialog(to_cloud: bool, onclose: EventHandler<()>) -> Element {
    let mut state = use_app_state();
    let toaster = use_toaster();
    let mut mode = use_signal(|| "existing".to_string());
    let mut display = use_signal(String::new);
    let mut username = use_signal(String::new);
    let mut password = use_signal(String::new);
    let mut confirm = use_signal(String::new);
    let mut agreed = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let mut busy = use_signal(|| false);
    let name = state.active_congregation().map(|c| c.name).unwrap_or_default();
    let new = mode() == "new";
    let score = crate::password::strength(&password(), &[username().trim(), display().trim()]);
    let ready = username().trim().len() >= 3
        && !password().is_empty()
        && (!new || (!display().trim().is_empty() && score >= 3 && confirm() == password()))
        && (to_cloud || agreed());
    let start = move |_| {
        if !ready {
            error.set(Some(if new { "Fill in every field; the password must be strong and typed twice.".into() } else { "Fill in the username and the password.".into() }));
            return;
        }
        busy.set(true);
        error.set(None);
        spawn(async move {
            // Let the spinner appear before the (deliberately slow) key derivation.
            theo_ui::sleep(std::time::Duration::from_millis(60)).await;
            let new_name = new.then(|| display.read().trim().to_string());
            let result = if to_cloud {
                state.move_to_cloud(&username(), &password(), new_name.as_deref()).await
            } else {
                state.move_to_device(&username(), &password(), new_name.as_deref()).await
            };
            busy.set(false);
            match result {
                Ok(()) => {
                    toaster.success(if to_cloud { "Moved to the Cloud. Open it with your Cloud account, on any device." } else { "Moved to this device. Open it with the congregation password here." });
                    onclose.call(());
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };
    let accounts = vec![
        Segment { value: "existing".into(), label: if to_cloud { "I have a Cloud account".into() } else { "Account on this device".into() }, icon: Some(Icon::User) },
        Segment { value: "new".into(), label: "New account".into(), icon: Some(Icon::SignUp) },
    ];
    rsx! {
        Dialog {
            open: true,
            onclose: move |_| onclose.call(()),
            title: if to_cloud { "Move {name} to the Cloud" } else { "Move {name} to this device" },
            description: if to_cloud { "It moves to the Theo Manager Cloud and leaves this device." } else { "It moves to this device and leaves the Cloud." },
            actions: rsx! {
                ActionButton { action: Action::Cancel, onclick: move |_| onclose.call(()) }
                ActionButton {
                    action: if to_cloud { Action::MoveToCloud } else { Action::MoveToDevice },
                    label: "Move",
                    variant: if to_cloud { ButtonVariant::Primary } else { ButtonVariant::Destructive },
                    loading: busy(),
                    onclick: start,
                }
            },
            Stack {
                if to_cloud {
                    Alert { tone: Tone::Info, title: "What happens",
                        "1. You sign in to your Cloud account, or create one. 2. Everything is copied to the Cloud, encrypted on this device first, with the same congregation password. 3. Only then is the copy on this device deleted. Afterwards you open it with your Cloud account, on any device, and you can invite others."
                    }
                } else {
                    Alert { tone: Tone::Error, title: "Everybody else loses access",
                        "Only you keep it, on this device. The Cloud copy is deleted for everyone: the other members can no longer open it, open invites stop working, and this cannot be undone. Export it first if you want to keep a copy. Afterwards you open it with the congregation password on this device."
                    }
                }
                if let Some(e) = error() {
                    Alert { tone: Tone::Error, "{e}" }
                }
                Segmented { value: mode(), options: accounts, onchange: move |v| { mode.set(v); error.set(None); } }
                if new {
                    Field { label: "Your name", required: true,
                        TextInput { value: display(), oninput: move |v| display.set(v), autocomplete: "name" }
                    }
                }
                Field { label: "Username", required: true, help: if to_cloud && new { "Not your real name: the server can see usernames." } else { "" },
                    TextInput { value: username(), oninput: move |v| { username.set(v); error.set(None); }, autocomplete: "username", icon: Icon::User }
                }
                Field { label: if to_cloud { "Cloud account password" } else { "Password" }, required: true,
                    PasswordInput {
                        value: password(),
                        oninput: move |v| { password.set(v); error.set(None); },
                        autocomplete: if new { "new-password" } else { "current-password" },
                        strength: if new { Some(score) } else { None },
                    }
                }
                if new {
                    Field { label: "Repeat password", required: true,
                        PasswordInput { value: confirm(), oninput: move |v| confirm.set(v), autocomplete: "new-password" }
                    }
                }
                if !to_cloud {
                    Switch { checked: agreed(), onchange: move |v| agreed.set(v), label: "I understand that everybody else loses access", description: "The congregation will exist only on this device." }
                }
            }
        }
    }
}

#[component]
fn RestoreBackupDialog(onclose: EventHandler<()>, ondone: EventHandler<usize>) -> Element {
    let mut state = use_app_state();
    let name = state.active_congregation().map(|c| c.name).unwrap_or_default();
    let mut file = use_signal(|| None::<(String, Vec<u8>)>);
    let mut password = use_signal(String::new);
    let mut error = use_signal(|| None::<String>);
    let mut busy = use_signal(|| false);
    let restore = move |_| {
        let Some((_, bytes)) = file() else {
            error.set(Some("Choose a backup file.".into()));
            return;
        };
        busy.set(true);
        spawn(async move {
            theo_ui::sleep(std::time::Duration::from_millis(60)).await;
            match crate::backup::open(&bytes, &password()) {
                Ok(backup) => {
                    let people = backup.directory.people.len();
                    state.restore(backup);
                    ondone.call(people);
                }
                Err(msg) => error.set(Some(msg)),
            }
            busy.set(false);
        });
    };
    let chosen = file().map(|(n, b)| format!("{n} ({} KB)", b.len().div_ceil(1024))).unwrap_or_else(|| "No file chosen".into());
    rsx! {
        Dialog {
            open: true,
            onclose: move |_| onclose.call(()),
            title: "Import",
            description: "Choose an export file (.theobackup) and type its password.",
            actions: rsx! {
                ActionButton { action: Action::Cancel, onclick: move |_| onclose.call(()) }
                ActionButton { action: Action::RestoreBackup, variant: ButtonVariant::Destructive, label: "Import and replace", loading: busy(), onclick: restore }
            },
            Stack {
                Alert { tone: Tone::Warning, title: "This replaces all data of {name}",
                    "Everything that is not in the file is lost, for everyone who uses this congregation. The congregation password and where it is stored stay as they are."
                }
                Field { label: "Export file", required: true,
                    Cluster { gap: Gap::Sm,
                        FileButton { label: "Choose file", icon: Icon::Upload, accept: ".{crate::backup::EXTENSION}", on_file: move |f| { file.set(Some(f)); error.set(None); } }
                        Text { tone: TextTone::Small, "{chosen}" }
                    }
                }
                Field { label: "Password of the file", required: true, error: error(),
                    PasswordInput { value: password(), autocomplete: "current-password", invalid: error().is_some(), oninput: move |v| { password.set(v); error.set(None); } }
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
            Card { title: "My ministry", icon: Icon::Footprints,
                Cluster { justify: Justify::Between,
                    Text { tone: TextTone::Muted, "Log your time, credit hours and Bible studies each day, and send the month as your report." }
                    ActionButton { action: Action::View, label: "Open my ministry", onclick: move |_| { navigator.push(Route::MyMinistryPage {}); } }
                }
            }
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
                        if let (Some(label), Some(c)) = (biometric_label().filter(|_| congregation.as_ref().is_some_and(|c| state.passkey_available(&c.id))), congregation.clone()) {
                            Switch {
                                checked: state.has_passkey(&c.id),
                                onchange: move |v| {
                                    let id = c.id.clone();
                                    if v {
                                        // The browser asks for the fingerprint, face or PIN and makes a passkey.
                                        spawn(async move {
                                            match state.enable_passkey(&id).await {
                                                Ok(()) => toaster.success("Passkey unlock is on for this device."),
                                                Err(e) => toaster.error(format!("Passkey unlock is not on: {e}")),
                                            }
                                        });
                                    } else {
                                        state.disable_passkey(&id);
                                        toaster.success("Passkey unlock is off.");
                                    }
                                },
                                label: "Unlock {c.name} with {label}",
                                description: "The congregation's key is sealed by the passkey and kept only on this device; without the passkey's fingerprint, face or PIN it cannot be opened.",
                            }
                        } else if congregation.as_ref().is_some_and(|c| matches!(c.mode, StorageMode::Cloud { .. })) {
                            Text { tone: TextTone::Muted, "This Cloud congregation opens with your account; it needs no passkey of its own." }
                        } else {
                            Text { tone: TextTone::Muted, "Passkey unlock is not available here yet: it needs the browser's passkeys (the installed apps will use the device's own key store)." }
                        }
                        StayUnlockedField {}
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

/// *Stay unlocked* (this device): how long the congregation stays open after the app is closed or
/// reloaded, or lock at once.
#[component]
fn StayUnlockedField() -> Element {
    let state = use_app_state();
    let toaster = use_toaster();
    let mut minutes = use_signal(crate::state::stay_unlocked_minutes);
    let options: Vec<SelectOption> = crate::state::STAY_UNLOCKED.iter().map(|(m, l)| SelectOption::new(m.to_string(), *l)).collect();
    rsx! {
        Field {
            label: "Stay unlocked on this device",
            help: "After closing or reloading the app, it opens without the congregation password for this long after you last used it. Lock always asks again.",
            Select {
                value: minutes().to_string(),
                options,
                onchange: move |v: String| {
                    let m = v.parse().unwrap_or(0);
                    minutes.set(m);
                    state.set_stay_unlocked(m);
                    toaster.success(if m == 0 { "The app locks when it is closed or reloaded.".to_string() } else { "Saved for this device.".to_string() });
                },
            }
        }
    }
}
