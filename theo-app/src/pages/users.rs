//! *Users › All users*: the list with filters and export, and the person page (new and edit).
//! Behaviour: `docs/model/m01-people.md` in the meta repo.

use crate::export::{self, Format, Table};
use crate::people::{
    self, AccountLink, Activity, Appointment, Directory, Gender, GroupRole, Person, PersonDraft, PublisherType,
    RelationKind,
};
use crate::prefs::{self, Preferences};
use crate::state::{use_app_state, StorageMode};
use crate::{platform, Route};
use dioxus::prelude::*;
use theo_ui::*;
use theo_ui::Action; // explicit: dioxus::prelude also exports an `Action`

// ---------------------------------------------------------------------------------------------
// List
// ---------------------------------------------------------------------------------------------

/// Labels on a person's card.
fn tags(dir: &Directory, p: &Person) -> Vec<Tag> {
    let ty = p.publisher_type;
    let mut t = vec![Tag::new(ty.label(), if ty.is_pioneer() { Tone::Info } else { Tone::Neutral })];
    if p.moved_away.is_some() {
        t.push(Tag::new("Moved away", Tone::Neutral).with_icon(Icon::MovedAway));
        return t;
    }
    if let Some(a) = p.appointment {
        t.push(Tag::new(a.label(), Tone::Success).with_icon(Icon::Privilege));
    }
    if p.anointed {
        t.push(Tag::new("Anointed", Tone::Neutral).with_icon(Icon::Sparkles));
    }
    if p.infirm {
        t.push(Tag::new("Infirm", Tone::Neutral).with_icon(Icon::Infirm));
    }
    match dir.activity(p) {
        Some(Activity::Irregular) => t.push(Tag::new("Irregular", Tone::Warning).with_icon(Icon::Warning)),
        Some(Activity::Inactive) => t.push(Tag::new("Inactive", Tone::Error).with_icon(Icon::Error)),
        _ => {}
    }
    t.extend(label_tags(dir, p));
    t
}

/// The labels of a person as coloured tags.
fn label_tags(dir: &Directory, p: &Person) -> Vec<Tag> {
    p.labels.iter().filter_map(|id| dir.label(*id)).map(|l| Tag::new(l.name.clone(), l.color.tone()).with_icon(Icon::Tag)).collect()
}

/// "Age: 23.5 years" for a date field, or nothing when the date is empty or invalid.
fn years_text(iso: &str, label: &str) -> Option<String> {
    let years = people::parse_date(iso).and_then(crate::prefs::years_since)?;
    Some(format!("{label}: {years:.1} years"))
}

/// A stored `YYYY-MM-DD` date in the user's date format.
fn show_date(prefs: &Preferences, iso: &str) -> String {
    people::parse_date(iso).map(|d| prefs.format_date(d)).unwrap_or_else(|| iso.to_string())
}

/// "North · Overseer", "Family head"… under the name.
fn card_subtitle(dir: &Directory, p: &Person, prefs: &Preferences) -> Option<String> {
    if let Some(d) = &p.moved_away {
        return Some(format!("Moved {} · removed {}", show_date(prefs, d), show_date(prefs, &people::removal_date(d))));
    }
    let mut parts: Vec<String> = dir
        .groups_of(p.id)
        .into_iter()
        .map(|(g, role)| match role {
            GroupRole::Overseer => format!("{} · Overseer", g.name),
            GroupRole::Assistant => format!("{} · Assistant", g.name),
            GroupRole::Member => g.name.clone(),
        })
        .collect();
    if dir.family_of(p.id).is_some_and(|f| f.head == p.id) {
        parts.push("Family head".into());
    }
    (!parts.is_empty()).then(|| parts.join(" · "))
}

fn group_names(dir: &Directory, n: u32) -> String {
    dir.groups_of(n).iter().map(|(g, _)| g.name.clone()).collect::<Vec<_>>().join(", ")
}

/// The export table. CSV gets every field; DOCX and PDF a printable list.
fn export_table(dir: &Directory, list: &[Person], prefs: &Preferences, format: Format, filters: String, congregation: &str) -> Table {
    let s = |o: &Option<String>| o.clone().unwrap_or_default();
    let yes = |b: bool| if b { "Yes" } else { "No" }.to_string();
    let title = format!("{congregation} · All users");
    if format == Format::Csv {
        let columns = [
            "First name", "Last name", "Gender", "Birthday", "E-mail", "Telephone", "Mobile phone", "Baptism date",
            "Type", "Infirm", "Pioneer number", "Appointment", "Anointed", "Address line 1", "Address line 2",
            "Postal code", "City", "Consent date", "Notes", "Field service group", "Family head", "Activity", "Moved away", "Labels", "Emergency contacts",
        ];
        let rows = list
            .iter()
            .map(|p| {
                let head = dir.family_of(p.id).and_then(|f| dir.person(f.head)).map(|h| h.display_name(prefs)).unwrap_or_default();
                vec![
                    p.first_name.clone(),
                    p.last_name.clone(),
                    p.gender.label().into(),
                    s(&p.birthday),
                    s(&p.email),
                    s(&p.telephone),
                    s(&p.mobile_phone),
                    s(&p.baptism_date),
                    p.publisher_type.label().into(),
                    yes(p.infirm),
                    s(&p.pioneer_number),
                    p.appointment.map(|a| a.label().to_string()).unwrap_or_default(),
                    yes(p.anointed),
                    s(&p.address_line_1),
                    s(&p.address_line_2),
                    s(&p.postal_code),
                    s(&p.city),
                    s(&p.consent_date),
                    s(&p.notes),
                    group_names(dir, p.id),
                    head,
                    dir.activity(p).map(|a| a.label().to_string()).unwrap_or_default(),
                    s(&p.moved_away),
                    p.labels.iter().filter_map(|id| dir.label(*id)).map(|l| l.name.clone()).collect::<Vec<_>>().join(", "),
                    p.emergency_contacts
                        .iter()
                        .map(|c| {
                            let phone = c.mobile_phone.clone().or_else(|| c.telephone.clone()).unwrap_or_default();
                            if phone.is_empty() { c.name() } else { format!("{} ({phone})", c.name()) }
                        })
                        .collect::<Vec<_>>()
                        .join("; "),
                ]
            })
            .collect();
        return Table { title, subtitle: filters, columns: columns.iter().map(|c| c.to_string()).collect(), widths: vec![1.0; columns.len()], rows };
    }
    let rows = list
        .iter()
        .map(|p| {
            vec![
                p.display_name(prefs),
                p.publisher_type.label().into(),
                p.appointment.map(|a| a.label().to_string()).unwrap_or_default(),
                group_names(dir, p.id),
                s(&p.mobile_phone),
                s(&p.email),
            ]
        })
        .collect();
    let (y, m, d) = prefs::today();
    Table {
        title,
        subtitle: format!("{filters} · {} users · {}", list.len(), prefs.format_date((y, m, d))),
        columns: ["Name", "Type", "Appointment", "Group", "Mobile phone", "E-mail"].iter().map(|c| c.to_string()).collect(),
        widths: vec![2.2, 2.1, 1.5, 1.0, 1.7, 2.6],
        rows,
    }
}

/// The export menu of every list page (same entries, same warning: R-066).
#[component]
pub(super) fn ExportMenu(csv_description: String, onselect: EventHandler<String>) -> Element {
    rsx! {
        MenuButton {
            label: Action::Export.label(),
            icon: Icon::Download,
            note: "Exported files are not encrypted. Keep them safe and delete them when you no longer need them.",
            items: vec![
                MenuItem::new("pdf", "PDF", Icon::FilePdf).with_description("To print or share"),
                MenuItem::new("docx", "Word (DOCX)", Icon::FileDocx).with_description("To edit before printing"),
                MenuItem::new("csv", "CSV", Icon::FileCsv).with_description(csv_description),
            ],
            onselect: move |id| onselect.call(id),
        }
    }
}

/// Renders and saves an export, then tells the user where it went.
pub(super) fn save_export(table: &Table, format: Format, congregation: &str, what: &str, toaster: Toaster) {
    let bytes = export::render(table, format);
    let name = format!("{}-{what}.{}", people::fold(congregation).replace(' ', "-"), format.extension());
    match platform::save_file(&name, format.mime(), &bytes) {
        Ok(msg) => toaster.success(msg),
        Err(msg) => toaster.error(msg),
    }
}

/// The tabs of All users: people and families.
#[component]
pub(super) fn UsersTabs() -> Element {
    let mut state = use_app_state();
    rsx! {
        Segmented {
            value: (state.users_tab)(),
            options: vec![
                Segment { value: "people".into(), label: "People".into(), icon: Some(Icon::Users) },
                Segment { value: "families".into(), label: "Families".into(), icon: Some(Icon::Family) },
            ],
            onchange: move |v: String| state.users_tab.set(v),
        }
    }
}

/// `/app/users/all`: the People tab, or the Families tab.
#[component]
pub fn UserList() -> Element {
    let state = use_app_state();
    if (state.users_tab)() == "families" {
        rsx! { super::families::FamiliesTab {} }
    } else {
        rsx! { PeopleTab {} }
    }
}

#[component]
fn PeopleTab() -> Element {
    let mut state = use_app_state();
    let navigator = navigator();
    let toaster = use_toaster();
    let prefs = state.preferences();
    let dir = state.directory();
    let congregation = state.active_congregation().map(|c| c.name).unwrap_or_default();
    let query = (state.user_query)();
    let sel = (state.user_filters)();
    let list: Vec<Person> = people::filtered(&dir, &query, &sel, &prefs).into_iter().cloned().collect();
    let facets = people::facets(&dir, &query, &sel);
    let total = dir.people.len();
    let summary = format!("{} of {total} users", list.len());
    let cards: Vec<(u32, String, Option<String>, Vec<Tag>)> =
        list.iter().map(|p| (p.id, p.display_name(&prefs), card_subtitle(&dir, p, &prefs), tags(&dir, p))).collect();
    let mut managing = use_signal(|| false);

    let on_export = {
        let (dir, list, prefs, congregation) = (dir.clone(), list.clone(), prefs.clone(), congregation.clone());
        move |id: String| {
            let Some(format) = Format::parse(&id) else { return };
            if list.is_empty() {
                toaster.error("There is nothing to export. Change the filters first.");
                return;
            }
            let filters = people::describe(&dir, &(state.user_query)(), &(state.user_filters)());
            let table = export_table(&dir, &list, &prefs, format, filters, &congregation);
            save_export(&table, format, &congregation, "users", toaster);
        }
    };

    rsx! {
        Page {
            title: "All users",
            subtitle: "{congregation} · Publishers, Bible students and visitors",
            actions: rsx! {
                ActionButton { action: Action::ManageLabels, onclick: move |_| managing.set(true) }
                ExportMenu { csv_description: "Every field, for spreadsheets", onselect: on_export }
                ActionButton { action: Action::Add, label: "Add user", onclick: move |_| { navigator.push(Route::UserNew {}); } }
            },
            if total == 0 {
                StickyBar { UsersTabs {} }
                EmptyState {
                    icon: Icon::Users,
                    title: "No users yet",
                    description: "Add the publishers, Bible students and visitors of {congregation}.",
                    action: rsx! { ActionButton { action: Action::Add, label: "Add user", onclick: move |_| { navigator.push(Route::UserNew {}); } } },
                }
            } else {
                StickyBar {
                    UsersTabs {}
                    FilterBar {
                        query,
                        on_query: move |v| state.user_query.set(v),
                        search_placeholder: "Search by name",
                        facets,
                        selected: sel,
                        on_select: move |s| state.user_filters.set(s),
                        summary,
                        defaults: people::default_filters(),
                    }
                }
                if cards.is_empty() {
                    EmptyState {
                        icon: Icon::Search,
                        title: "No users match",
                        description: "Try another search, or remove some filters.",
                        action: rsx! {
                            ActionButton {
                                action: Action::ClearFilters,
                                onclick: move |_| {
                                    state.user_query.set(String::new());
                                    state.user_filters.set(people::default_filters());
                                },
                            }
                        },
                    }
                } else {
                    Grid { cols: 3, gap: Gap::Sm,
                        for (n, name, subtitle, tags) in cards {
                            RecordCard {
                                key: "{n}",
                                title: name.clone(),
                                avatar: name,
                                subtitle,
                                tags,
                                onclick: move |_| { navigator.push(Route::UserDetail { id: n }); },
                            }
                        }
                    }
                }
            }
        }
        if managing() {
            super::person_extras::ManageLabelsDialog { onclose: move |_| managing.set(false) }
        }
    }
}

/// `/app/users/moved-away`: people who moved away, kept for 1 year (m01-people.md).
#[component]
pub fn MovedAway() -> Element {
    let state = use_app_state();
    let navigator = navigator();
    let prefs = state.preferences();
    let dir = state.directory();
    let mut query = use_signal(String::new);
    let mut sel = use_signal(FilterSelection::new);
    let pool = people::moved_away(&dir);
    let total = pool.len();
    // The All users filters that still mean something after a move (no status, group or activity).
    let defs = people::standard_facets(
        &dir,
        &[
            people::FACET_LABELS,
            people::FACET_TYPE,
            people::FACET_APPOINTMENT,
            people::FACET_GENDER,
            people::FACET_ANOINTED,
            people::FACET_CONSENT,
            people::FACET_EMERGENCY,
        ],
    );
    let facets = people::facets_for(&dir, &pool, &query(), &sel(), defs, &[]);
    let mut list: Vec<&Person> = people::filter_pool(&dir, &pool, &query(), &sel(), &[], &prefs);
    list.sort_by(|a, b| b.moved_away.cmp(&a.moved_away));
    let cards: Vec<(u32, String, Option<String>, Vec<Tag>)> =
        list.iter().map(|p| (p.id, p.display_name(&prefs), card_subtitle(&dir, p, &prefs), tags(&dir, p))).collect();
    rsx! {
        Page {
            title: "Moved away",
            subtitle: "Kept for 1 year after they move away, then removed for good.",
            if total == 0 {
                EmptyState {
                    icon: Icon::MovedAway,
                    title: "Nobody has moved away",
                    description: "When someone moves to another congregation, open their page and choose Moved away. They stay here for 1 year.",
                }
            } else {
                StickyBar {
                    FilterBar {
                        query: query(),
                        on_query: move |v| query.set(v),
                        search_placeholder: "Search by name",
                        facets,
                        selected: sel(),
                        on_select: move |v| sel.set(v),
                        summary: format!("{} of {total} people", cards.len()),
                    }
                }
                Grid { cols: 3, gap: Gap::Sm,
                    for (n, name, subtitle, tags) in cards {
                        RecordCard {
                            key: "{n}",
                            title: name.clone(),
                            avatar: name,
                            subtitle,
                            tags,
                            onclick: move |_| { navigator.push(Route::UserDetail { id: n }); },
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Person page
// ---------------------------------------------------------------------------------------------

/// `/app/users/all/new`
#[component]
pub fn UserNew() -> Element {
    rsx! { PersonEditor { id: None } }
}

/// `/app/users/all/:id`
#[component]
pub fn UserDetail(id: u32) -> Element {
    rsx! { PersonEditor { key: "{id}", id: Some(id) } }
}

fn options(pairs: &[(&str, &str)]) -> Vec<SelectOption> {
    pairs.iter().map(|(v, l)| SelectOption::new(*v, *l)).collect()
}

/// What a save also changes (values that no longer apply, roles and privileges that are lost).
/// "overseer of North" and the like: the group roles of a person.
fn leads_of(dir: &Directory, id: Option<u32>) -> Vec<String> {
    id
        .map(|n| {
            dir.groups_of(n)
                .into_iter()
                .filter_map(|(g, r)| match r {
                    GroupRole::Overseer => Some(format!("overseer of {}", g.name)),
                    GroupRole::Assistant => Some(format!("assistant of {}", g.name)),
                    GroupRole::Member => None,
                })
                .collect()
        })
        .unwrap_or_default()
}

fn cleanup_for(dir: &Directory, id: Option<u32>, d: &PersonDraft) -> Vec<String> {
    let leads = leads_of(dir, id);
    let mut cleanup = d.cleanup_notes(&leads);
    if let (Some(n), Some(g)) = (id, people::Gender::parse(&d.gender)) {
        let lost = dir.privileges_lost(n, d.ty(), g);
        if !lost.is_empty() {
            let names: Vec<String> = lost.iter().map(|x| x.full_label()).collect();
            cleanup.push(format!("{} privileges will be removed: {}.", lost.len(), names.join(", ")));
        }
    }
    cleanup
}

#[component]
fn PersonEditor(id: Option<u32>) -> Element {
    let mut state = use_app_state();
    let navigator = navigator();
    let toaster = use_toaster();
    let prefs = state.preferences();
    let dir = state.directory();
    let cloud = state.active_congregation().is_some_and(|c| matches!(c.mode, StorageMode::Cloud { .. }));
    let existing = id.and_then(|n| dir.person(n).cloned());

    let mut draft = use_signal(|| existing.as_ref().map(|p| PersonDraft::from_person(p, &dir)).unwrap_or_default());
    let mut errors = use_signal(Vec::<(&'static str, String)>::new);
    let mut confirm_delete = use_signal(|| false);
    let mut invite_open = use_signal(|| false);
    let mut move_open = use_signal(|| false);
    let mut move_date = use_signal(people::today_iso);
    let mut move_family = use_signal(|| true);
    let mut move_error = use_signal(|| None::<String>);
    let moved = existing.as_ref().and_then(|p| p.moved_away.clone());

    let back = if moved.is_some() {
        ("Moved away".to_string(), EventHandler::new(move |_| { navigator.push(Route::MovedAway {}); }))
    } else {
        ("All users".to_string(), EventHandler::new(move |_| { navigator.push(Route::UserList {}); }))
    };
    if id.is_some() && existing.is_none() {
        return rsx! {
            Page { title: "User not found", back,
                EmptyState { icon: Icon::Search, title: "This user does not exist", description: "They may have been deleted." }
            }
        };
    }

    // Existing people save as you type (like the cards next to the form). A change that also
    // removes something (privileges, roles, values that no longer apply) waits for "Apply".
    let mut saved = use_signal(|| draft.peek().clone());
    use_effect(move || {
        let d = draft();
        let Some(n) = id else { return };
        if d == *saved.peek() {
            return;
        }
        // Check before writing: a write, even a failed save, would run this effect again.
        let invalid = d.validate();
        if !invalid.is_empty() {
            errors.set(invalid);
            return;
        }
        let dir = state.directory();
        if !cleanup_for(&dir, id, &d).is_empty() {
            return;
        }
        match state.update_directory(|dir| dir.save(Some(n), &d)) {
            Ok(_) => {
                errors.set(Vec::new());
                saved.set(d);
            }
            Err(e) => errors.set(e),
        }
    });

    let d = draft();
    let ty = d.ty();
    let err = move |f: &str| errors.read().iter().find(|(k, _)| *k == f).map(|(_, m)| m.clone());
    let cleanup = cleanup_for(&dir, id, &d);
    let leads = leads_of(&dir, id);
    let unsaved_errors = id.is_some() && !errors.read().is_empty();
    let name = existing.as_ref().map(|p| p.display_name(&prefs));
    let title = name.clone().unwrap_or_else(|| "New user".into());
    let subtitle = match &existing {
        Some(p) => p.publisher_type.label().to_string(),
        None => "Add a publisher, Bible student or visitor.".into(),
    };

    let type_options: Vec<SelectOption> = PublisherType::ALL.iter().map(|t| SelectOption::new(t.value(), t.label())).collect();
    let mut group_options = vec![SelectOption::new("", "No group")];
    group_options.extend(dir.groups.iter().map(|g| SelectOption::new(g.id.to_string(), g.name.clone())));

    let mut save = move |_: ()| {
        let d = draft();
        match state.update_directory(|dir| dir.save(id, &d)) {
            Ok(n) => {
                errors.set(Vec::new());
                if id.is_none() {
                    toaster.success("User added.");
                    navigator.replace(Route::UserDetail { id: n });
                } else {
                    toaster.success("Changes saved.");
                    let dir = state.directory();
                    if let Some(p) = dir.person(n) {
                        let fresh = PersonDraft::from_person(p, &dir);
                        saved.set(fresh.clone());
                        draft.set(fresh);
                    }
                }
            }
            Err(e) => {
                errors.set(e);
                toaster.error("Check the fields marked in red.");
            }
        }
    };

    let form_actions = if id.is_none() {
        rsx! {
            ActionButton { action: Action::Cancel, onclick: move |_| { navigator.push(Route::UserList {}); } }
            ActionButton { action: Action::Save, kind: ButtonKind::Submit }
        }
    } else {
        rsx! {
            Text { tone: TextTone::Small, "Changes are saved automatically." }
        }
    };
    let delete_consequences = id.map(|n| dir.delete_consequences(n, &prefs)).unwrap_or_default();
    let delete_name = name.clone().unwrap_or_default();
    let invite_name = name.clone().unwrap_or_default();
    let move_name = name.clone().unwrap_or_default();
    let back_name = name.clone().unwrap_or_default();
    let family_names: Vec<String> = id
        .map(|n| dir.family_to_move(n).into_iter().filter_map(|m| dir.person(m)).map(|p| p.display_name(&prefs)).collect())
        .unwrap_or_default();
    let move_consequences = id.map(|n| dir.move_away_consequences(n, move_family() && !family_names.is_empty(), &prefs)).unwrap_or_default();
    let moved_banner = moved.as_ref().map(|d| (show_date(&prefs, d), show_date(&prefs, &people::removal_date(d))));

    rsx! {
        Page {
            title,
            subtitle,
            back,
            actions: rsx! {
                if id.is_some() {
                    if moved.is_some() {
                        ActionButton {
                            action: Action::MoveBack,
                            onclick: move |_| {
                                if let Some(n) = id {
                                    state.update_directory(|d| d.move_back(n));
                                    toaster.success(format!("{back_name} is a current user again. Choose their field service group."));
                                }
                            },
                        }
                    } else {
                        ActionButton { action: Action::MoveAway, onclick: move |_| { move_error.set(None); move_open.set(true); } }
                    }
                    ActionButton { action: Action::Delete, onclick: move |_| confirm_delete.set(true) }
                }
            },
            if let Some((on, removed)) = moved_banner {
                Alert { tone: Tone::Info, title: "Moved away on {on}",
                    "Theo Manager removes this person and their history for good on {removed}. Until then you can still edit them, or move them back."
                }
            }
            Split {
                main: rsx! {
                    Form {
                        onsubmit: move |_| if id.is_none() { save(()) },
                        floating_save: id.is_none(),
                        actions: form_actions,
                        Card { title: "Personal", icon: Icon::User,
                            Grid { cols: 2,
                                Field { label: "First name", required: true, error: err("first_name"),
                                    TextInput { value: d.first_name.clone(), autocomplete: "off", invalid: err("first_name").is_some(), oninput: move |v| draft.write().first_name = v }
                                }
                                Field { label: "Last name", required: true, error: err("last_name"),
                                    TextInput { value: d.last_name.clone(), autocomplete: "off", invalid: err("last_name").is_some(), oninput: move |v| draft.write().last_name = v }
                                }
                                Field { label: "Gender", required: true, error: err("gender"),
                                    Select {
                                        value: d.gender.clone(),
                                        invalid: err("gender").is_some(),
                                        options: options(&[("", "Choose…"), (Gender::Male.value(), "Male"), (Gender::Female.value(), "Female")]),
                                        onchange: move |v| draft.write().gender = v,
                                    }
                                }
                                Field { label: "Birthday", error: err("birthday"), help: years_text(&d.birthday, "Age"),
                                    TextInput { kind: InputKind::Date, value: d.birthday.clone(), invalid: err("birthday").is_some(), oninput: move |v| draft.write().birthday = v }
                                }
                            }
                        }
                        Card { title: "Contact", icon: Icon::Phone,
                            Grid { cols: 2,
                                Field { label: "Mobile phone",
                                    TextInput { kind: InputKind::Tel, value: d.mobile_phone.clone(), autocomplete: "off", oninput: move |v| draft.write().mobile_phone = v }
                                }
                                Field { label: "Telephone",
                                    TextInput { kind: InputKind::Tel, value: d.telephone.clone(), autocomplete: "off", oninput: move |v| draft.write().telephone = v }
                                }
                                Field { label: "E-mail", error: err("email"),
                                    TextInput { kind: InputKind::Email, value: d.email.clone(), autocomplete: "off", invalid: err("email").is_some(), oninput: move |v| draft.write().email = v }
                                }
                            }
                        }
                        Card { title: "Address", icon: Icon::Address,
                            Grid { cols: 2,
                                Field { label: "Address",
                                    TextInput { value: d.address_line_1.clone(), autocomplete: "off", oninput: move |v| draft.write().address_line_1 = v }
                                }
                                Field { label: "Address line 2",
                                    TextInput { value: d.address_line_2.clone(), autocomplete: "off", oninput: move |v| draft.write().address_line_2 = v }
                                }
                                Field { label: "Postal code",
                                    TextInput { value: d.postal_code.clone(), autocomplete: "off", oninput: move |v| draft.write().postal_code = v }
                                }
                                Field { label: "City",
                                    TextInput { value: d.city.clone(), autocomplete: "off", oninput: move |v| draft.write().city = v }
                                }
                            }
                        }
                        Card { title: "Congregation", icon: Icon::Congregation,
                            Grid { cols: 2,
                                Field { label: "Type", required: true,
                                    Select { value: d.publisher_type.clone(), options: type_options, onchange: move |v| draft.write().publisher_type = v }
                                }
                                if ty.is_baptised() {
                                    Field { label: "Baptism date", error: err("baptism_date"), help: years_text(&d.baptism_date, "Baptised"),
                                        TextInput { kind: InputKind::Date, value: d.baptism_date.clone(), invalid: err("baptism_date").is_some(), oninput: move |v| draft.write().baptism_date = v }
                                    }
                                }
                                if ty.has_pioneer_number() {
                                    Field { label: "Pioneer number",
                                        TextInput { value: d.pioneer_number.clone(), autocomplete: "off", oninput: move |v| draft.write().pioneer_number = v }
                                    }
                                }
                                if d.may_be_appointed() {
                                    Field { label: "Appointment",
                                        Select {
                                            value: d.appointment.clone(),
                                            options: options(&[("", "None"), (Appointment::Elder.value(), "Elder"), (Appointment::MinisterialServant.value(), "Ministerial servant")]),
                                            onchange: move |v| draft.write().appointment = v,
                                        }
                                    }
                                }
                                if ty.is_publisher() && moved.is_none() {
                                    Field {
                                        label: "Field service group",
                                        help: if leads.is_empty() { None } else { Some(format!("Also {}, and so a member of that group.", leads.join(" and "))) },
                                        Select { value: d.group.clone(), options: group_options, onchange: move |v| draft.write().group = v }
                                    }
                                }
                            }
                            if !ty.is_publisher() {
                                Text { tone: TextTone::Small, "Only publishers can be in a field service group." }
                            }
                            if ty.is_baptised() {
                                Switch { checked: d.anointed, label: "Anointed", onchange: move |v| draft.write().anointed = v }
                            }
                            Switch {
                                checked: d.infirm,
                                label: "Infirm",
                                description: "Cannot go out in the ministry as usual, for example because of age or illness.",
                                onchange: move |v| draft.write().infirm = v,
                            }
                        }
                        Card { title: "Data protection and notes", icon: Icon::Shield,
                            Grid { cols: 2,
                                Field { label: "Personal consent date", error: err("consent_date"), help: "The date this person agreed that the congregation keeps their data.",
                                    TextInput { kind: InputKind::Date, value: d.consent_date.clone(), invalid: err("consent_date").is_some(), oninput: move |v| draft.write().consent_date = v }
                                }
                            }
                            Field { label: "Notes", error: err("notes"), help: "Only what the congregation needs. At most 2,000 characters.",
                                TextArea { value: d.notes.clone(), invalid: err("notes").is_some(), oninput: move |v| draft.write().notes = v }
                            }
                        }
                        if !cleanup.is_empty() {
                            Alert { tone: Tone::Warning, title: if id.is_some() { "Not saved yet: this change also" } else { "When you save" },
                                Stack { gap: Gap::Xs,
                                    for (i, c) in cleanup.iter().enumerate() {
                                        Text { key: "{i}", "{c}" }
                                    }
                                    if id.is_some() {
                                        Cluster { gap: Gap::Sm,
                                            ActionButton { action: Action::Apply, onclick: move |_| save(()) }
                                            ActionButton { action: Action::Undo, onclick: move |_| { errors.set(Vec::new()); draft.set(saved()); } }
                                        }
                                    }
                                }
                            }
                        }
                        if unsaved_errors {
                            Alert { tone: Tone::Error, title: "Not saved",
                                "Fix the fields marked in red to save your changes."
                            }
                        }
                    }
                },
                side: rsx! {
                    if let Some(p) = existing.clone() {
                        if p.publisher_type.is_publisher() && p.moved_away.is_none() {
                            super::reports::PersonReportsCard { person: p.id }
                        }
                        if p.moved_away.is_none() && p.publisher_type != PublisherType::Visitor {
                            super::privileges::PersonPrivilegesCard { person: p.id }
                        }
                        super::person_extras::LabelsCard { person: p.id }
                        FamilyCard { id: p.id }
                        super::person_extras::EmergencyContactsCard { person: p.id }
                        if cloud && p.moved_away.is_none() {
                            AccountCard { account: p.account.clone(), person: p.id, permissions: super::permissions::access_summary(&dir.access(p.id)), on_invite: move |_| invite_open.set(true) }
                        }
                    } else {
                        Card { title: "After saving", icon: Icon::Info,
                            Text { tone: TextTone::Muted,
                                if cloud { "You can add labels, family relationships and emergency contacts, and invite this person so they can use the app." } else { "You can add labels, family relationships and emergency contacts." }
                            }
                        }
                    }
                },
            }
        }
        Dialog {
            open: move_open(),
            onclose: move |_| move_open.set(false),
            title: "{move_name} moved away",
            description: "They leave the current users and are kept for 1 year, then removed for good with their history.",
            actions: rsx! {
                ActionButton { action: Action::Cancel, onclick: move |_| move_open.set(false) }
                ActionButton {
                    action: Action::MoveAway,
                    variant: ButtonVariant::Primary,
                    onclick: move |_| {
                        let date = move_date();
                        if people::parse_date(&date).is_none() || date > people::today_iso() {
                            move_error.set(Some("Enter a date that is not in the future.".into()));
                            return;
                        }
                        if let Some(n) = id {
                            let with_family = move_family();
                            state.update_directory(|d| d.move_away(n, &date, with_family));
                            move_open.set(false);
                            toaster.success("Marked as moved away.");
                        }
                    },
                }
            },
            Stack {
                Field { label: "Date they moved away", required: true, error: move_error(),
                    TextInput { kind: InputKind::Date, value: move_date(), oninput: move |v| move_date.set(v) }
                }
                if !family_names.is_empty() {
                    Switch {
                        checked: move_family(),
                        label: "The family moved too",
                        description: "{family_names.join(\", \")} move away as well, and stay a family.",
                        onchange: move |v| move_family.set(v),
                    }
                }
                if !move_consequences.is_empty() {
                    Alert { tone: Tone::Warning, title: "This also changes",
                        Stack { gap: Gap::Xs,
                            for (i, c) in move_consequences.iter().enumerate() {
                                Text { key: "{i}", "{c}" }
                            }
                        }
                    }
                }
            }
        }
        ConfirmDialog {
            open: confirm_delete(),
            title: "Delete {delete_name}?",
            message: "They disappear from lists, filters and exports. Past assignments and reports keep their name.",
            confirm: Action::Delete,
            consequences: delete_consequences,
            oncancel: move |_| confirm_delete.set(false),
            onconfirm: move |_| {
                if let Some(n) = id {
                    state.update_directory(|d| d.delete(n));
                    toaster.success(format!("{delete_name} was deleted."));
                    navigator.replace(Route::UserList {});
                }
            },
        }
        Dialog {
            open: invite_open(),
            onclose: move |_| invite_open.set(false),
            title: "Invite {invite_name}",
            description: "They get their own account, linked to this person.",
            actions: rsx! {
                ActionButton { action: Action::Cancel, onclick: move |_| invite_open.set(false) }
                ActionButton {
                    action: Action::Invite,
                    label: "Create invitation",
                    onclick: move |_| {
                        if let Some(n) = id {
                            let link = format!("https://app.example.org/join#t={}", crate::mock::token());
                            state.update_directory(|d| d.set_account(n, AccountLink::Invited { link }));
                            invite_open.set(false);
                            toaster.success("Invitation created. Send the link privately.");
                        }
                    },
                }
            },
            Stack {
                Field { label: "Role", help: "Roles and their permissions are set up in Users › Permissions.",
                    Select { value: "publisher", options: options(&[("publisher", "Publisher: own assignments, absences and reports")]), onchange: move |_| {} }
                }
                Text { tone: TextTone::Small, "The link works once and expires after 7 days. It does not contain the congregation password (R-252)." }
            }
        }
    }
}

/// Family: relatives, head, add or leave.
#[component]
fn FamilyCard(id: u32) -> Element {
    let mut state = use_app_state();
    let toaster = use_toaster();
    let prefs = state.preferences();
    let dir = state.directory();
    let mut relative = use_signal(String::new);
    let mut kind = use_signal(|| RelationKind::Spouse.value().to_string());
    let mut error = use_signal(|| None::<String>);

    let family = dir.family_of(id).cloned();
    let is_head = family.as_ref().is_some_and(|f| f.head == id);
    let head_name = family.as_ref().and_then(|f| dir.person(f.head)).map(|h| h.display_name(&prefs));
    let entries: Vec<ListEntry> = dir
        .relatives(id)
        .into_iter()
        .filter_map(|r| {
            let p = dir.person(r.id)?;
            Some(ListEntry {
                icon: if r.is_head { Icon::FamilyHead } else { Icon::User },
                title: p.display_name(&prefs),
                subtitle: {
                    let kind = r.kind.map(|k| k.label()).unwrap_or("Family member");
                    if r.is_head { format!("{kind} · Family head") } else { kind.to_string() }
                },
                meta: String::new(),
            })
        })
        .collect();
    let mut others: Vec<(String, String)> =
        dir.people.iter().filter(|p| p.id != id).map(|p| (p.id.to_string(), p.display_name(&prefs))).collect();
    others.sort_by_key(|(_, n)| people::fold(n));
    let mut person_options = vec![SelectOption::new("", "Choose a person…")];
    person_options.extend(others.into_iter().map(|(v, l)| SelectOption::new(v, l)));
    let kind_options: Vec<SelectOption> = RelationKind::ALL.iter().map(|k| SelectOption::new(k.value(), k.label())).collect();
    let new_head = dir.proposed_new_head(id).and_then(|h| dir.person(h)).map(|h| h.display_name(&prefs));

    let add = {
        let prefs = prefs.clone();
        move |_| {
            let (Ok(r), Some(k)) = (relative().parse::<u32>(), RelationKind::parse(&kind())) else {
                error.set(Some("Choose a person and a relationship.".into()));
                return;
            };
            match state.update_directory(|d| d.add_relationship(id, r, k, &prefs)) {
                Ok(()) => {
                    error.set(None);
                    relative.set(String::new());
                    toaster.success("Relative added.");
                }
                Err(e) => error.set(Some(e)),
            }
        }
    };

    rsx! {
        Card { title: "Family", icon: Icon::Family,
            if let Some(head) = head_name {
                Text { tone: TextTone::Muted, if is_head { "Head of the family." } else { "Family of {head}." } }
                ItemList { items: entries }
                Cluster { gap: Gap::Sm,
                    if !is_head {
                        ActionButton { action: Action::MakeFamilyHead, onclick: move |_| { state.update_directory(|d| d.make_head(id)); toaster.success("Family head changed."); } }
                    }
                    ActionButton {
                        action: Action::Remove,
                        label: "Leave family",
                        onclick: {
                            let new_head = new_head.clone();
                            move |_| {
                                state.update_directory(|d| d.remove_from_family(id));
                                match &new_head {
                                    Some(h) => toaster.success(format!("Removed from the family. {h} is now the family head.")),
                                    None => toaster.success("Removed from the family."),
                                }
                            }
                        },
                    }
                }
                Divider {}
            } else {
                Text { tone: TextTone::Muted, "Not in a family. Add a relative to start one." }
            }
            Field { label: "Relative", error: error(),
                Select { value: relative(), options: person_options, onchange: move |v| relative.set(v) }
            }
            Field { label: "This relative is their",
                Select { value: kind(), options: kind_options, onchange: move |v| kind.set(v) }
            }
            ActionButton { action: Action::Add, variant: ButtonVariant::Secondary, label: "Add relative", onclick: add }
        }
    }
}

/// Account and invitation (Cloud congregations only).
#[component]
fn AccountCard(account: AccountLink, person: u32, permissions: String, on_invite: EventHandler<()>) -> Element {
    let navigator = navigator();
    rsx! {
        Card { title: "Account", icon: Icon::Key,
            match account {
                AccountLink::None => rsx! {
                    Text { tone: TextTone::Muted, "No account. Invite them so they can see their assignments, add absences and submit their reports." }
                    ActionButton { action: Action::Invite, variant: ButtonVariant::Secondary, onclick: move |_| on_invite.call(()) }
                },
                AccountLink::Invited { link } => rsx! {
                    Cluster { gap: Gap::Sm, Badge { tone: Tone::Info, icon: Icon::Invitation, "Invited" } }
                    Text { tone: TextTone::Small, "Send this link privately, for example in person or in a private message. It works once and expires after 7 days." }
                    Code { "{link}" }
                },
                AccountLink::Linked { username } => rsx! {
                    Cluster { gap: Gap::Sm, Badge { tone: Tone::Success, icon: Icon::Check, "Linked" } }
                    Text { tone: TextTone::Muted, "Signs in as @{username}. Permissions: {permissions}." }
                    ActionButton { action: Action::Edit, variant: ButtonVariant::Secondary, label: "Edit permissions", onclick: move |_| { navigator.push(Route::PermissionsDetail { id: person }); } }
                },
            }
        }
    }
}
