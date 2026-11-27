//! Person page cards: emergency contacts and labels; and the dialog that manages the labels of
//! the congregation. Rules: `docs/model/m01-people.md`, "Emergency contacts" and "Labels".

use crate::people::LabelTone as _;
use crate::people::{EmergencyContact, LabelColor};
use crate::state::use_app_state;
use dioxus::prelude::*;
use theo_ui::*;
use theo_ui::Action; // explicit: dioxus::prelude also exports an `Action`

/// The emergency contacts of a person: open one to change it, or add a new one.
#[component]
pub(super) fn EmergencyContactsCard(person: u32) -> Element {
    let state = use_app_state();
    let dir = state.directory();
    let mut editing = use_signal(|| None::<Option<usize>>);
    let contacts = dir.person(person).map(|p| p.emergency_contacts.clone()).unwrap_or_default();
    let entries: Vec<ListEntry> = contacts
        .iter()
        .map(|c| ListEntry {
            icon: Icon::Phone,
            title: c.name(),
            subtitle: [c.mobile_phone.clone(), c.telephone.clone(), c.notes.clone()].into_iter().flatten().collect::<Vec<_>>().join(" · "),
            meta: String::new(),
        })
        .collect();
    rsx! {
        Card { title: "Emergency contacts", icon: Icon::Phone, actions: rsx! { Badge { tone: Tone::Neutral, "{contacts.len()}" } },
            if contacts.is_empty() {
                Text { tone: TextTone::Muted, "No emergency contact yet." }
            } else {
                ItemList { items: entries, on_select: move |i: usize| editing.set(Some(Some(i))) }
            }
            ActionButton { action: Action::Add, variant: ButtonVariant::Secondary, label: "Add emergency contact", onclick: move |_| editing.set(Some(None)) }
        }
        if let Some(index) = editing() {
            EmergencyContactDialog { key: "{index:?}", person, index, onclose: move |_| editing.set(None) }
        }
    }
}

#[component]
fn EmergencyContactDialog(person: u32, index: Option<usize>, onclose: EventHandler<()>) -> Element {
    let mut state = use_app_state();
    let toaster = use_toaster();
    let existing = index.and_then(|i| state.directory().person(person).and_then(|p| p.emergency_contacts.get(i).cloned()));
    let mut c = use_signal(|| existing.clone().unwrap_or_default());
    let mut error = use_signal(|| None::<String>);
    let mut confirm = use_signal(|| false);
    let text = |o: &Option<String>| o.clone().unwrap_or_default();
    let opt = |v: String| Some(v);
    let save = move |_| match state.update_directory(|d| d.save_emergency_contact(person, index, c())) {
        Ok(()) => {
            toaster.success("Emergency contact saved.");
            onclose.call(());
        }
        Err(e) => error.set(Some(e)),
    };
    let v = c();
    // Suggest family members (a new contact only); anyone else can still be typed in.
    let prefs = state.preferences();
    let dir = state.directory();
    let family: Vec<(u32, String)> = if index.is_none() {
        dir.family_others(person).into_iter().filter_map(|m| dir.person(m).map(|p| (m, p.display_name(&prefs)))).collect()
    } else {
        Vec::new()
    };
    let mut suggested = use_signal(String::new);
    let mut family_options = vec![SelectOption::new("", "Someone else (type below)")];
    family_options.extend(family.iter().map(|(id, name)| SelectOption::new(id.to_string(), name.clone())));
    let fill = move |v: String| {
        suggested.set(v.clone());
        let Some(p) = v.parse::<u32>().ok().and_then(|id| state.directory().person(id).cloned()) else { return };
        let relation = state.directory().relation_label(person, p.id);
        c.set(EmergencyContact {
            first_name: p.first_name,
            last_name: p.last_name,
            telephone: p.telephone,
            mobile_phone: p.mobile_phone,
            address_line_1: p.address_line_1,
            address_line_2: p.address_line_2,
            postal_code: p.postal_code,
            city: p.city,
            notes: relation,
        });
        error.set(None);
    };
    rsx! {
        Dialog {
            open: true,
            onclose: move |_| onclose.call(()),
            title: if existing.is_some() { "Emergency contact" } else { "Add emergency contact" },
            description: "Someone to call when this person cannot be reached or needs help.",
            actions: rsx! {
                EditActions {
                    on_save: save,
                    on_cancel: move |_| onclose.call(()),
                    on_delete: existing.is_some().then_some(EventHandler::new(move |_| confirm.set(true))),
                }
            },
            Stack {
                if !family.is_empty() {
                    Field { label: "From the family", help: "Fills in their details. Most emergency contacts are outside the congregation: then type them below.",
                        Select { value: suggested(), options: family_options, onchange: fill }
                    }
                }
                Grid { cols: 2,
                    Field { label: "First name", required: true, error: error(),
                        TextInput { value: v.first_name.clone(), oninput: move |x| c.write().first_name = x }
                    }
                    Field { label: "Last name", required: true,
                        TextInput { value: v.last_name.clone(), oninput: move |x| c.write().last_name = x }
                    }
                    Field { label: "Mobile phone",
                        TextInput { kind: InputKind::Tel, value: text(&v.mobile_phone), oninput: move |x| c.write().mobile_phone = opt(x) }
                    }
                    Field { label: "Telephone",
                        TextInput { kind: InputKind::Tel, value: text(&v.telephone), oninput: move |x| c.write().telephone = opt(x) }
                    }
                    Field { label: "Address",
                        TextInput { value: text(&v.address_line_1), oninput: move |x| c.write().address_line_1 = opt(x) }
                    }
                    Field { label: "Address line 2",
                        TextInput { value: text(&v.address_line_2), oninput: move |x| c.write().address_line_2 = opt(x) }
                    }
                    Field { label: "Postal code",
                        TextInput { value: text(&v.postal_code), oninput: move |x| c.write().postal_code = opt(x) }
                    }
                    Field { label: "City",
                        TextInput { value: text(&v.city), oninput: move |x| c.write().city = opt(x) }
                    }
                }
                Field { label: "Notes", help: "For example how they are related.",
                    TextArea { value: text(&v.notes), rows: 2, oninput: move |x| c.write().notes = opt(x) }
                }
            }
        }
        ConfirmDialog {
            open: confirm(),
            title: "Delete this emergency contact?",
            message: "The contact is removed from this person.",
            confirm: Action::Delete,
            oncancel: move |_| confirm.set(false),
            onconfirm: move |_| {
                if let Some(i) = index {
                    state.update_directory(|d| d.remove_emergency_contact(person, i));
                }
                toaster.success("Emergency contact deleted.");
                onclose.call(());
            },
        }
    }
}

/// The labels of a person: tap to put on or take off. Changes apply at once.
#[component]
pub(super) fn LabelsCard(person: u32) -> Element {
    let mut state = use_app_state();
    let dir = state.directory();
    let mut managing = use_signal(|| false);
    let mine = dir.person(person).map(|p| p.labels.clone()).unwrap_or_default();
    rsx! {
        Card { title: "Labels", icon: Icon::Tag,
            if dir.labels.is_empty() {
                Text { tone: TextTone::Muted, "The congregation has no labels yet." }
            } else {
                Cluster { gap: Gap::Sm,
                    for l in dir.labels.iter().cloned() {
                        TagToggle {
                            key: "{l.id}",
                            label: l.name.clone(),
                            tone: l.color.tone(),
                            selected: mine.contains(&l.id),
                            onclick: move |_| {
                                let on = !state.directory().person(person).is_some_and(|p| p.labels.contains(&l.id));
                                state.update_directory(|d| d.set_label(person, l.id, on));
                            },
                        }
                    }
                }
            }
            ActionButton { action: Action::ManageLabels, variant: ButtonVariant::Ghost, onclick: move |_| managing.set(true) }
        }
        if managing() {
            ManageLabelsDialog { onclose: move |_| managing.set(false) }
        }
    }
}

/// Add, rename, recolour and delete the labels of the congregation.
#[component]
pub(super) fn ManageLabelsDialog(onclose: EventHandler<()>) -> Element {
    let mut state = use_app_state();
    let toaster = use_toaster();
    let dir = state.directory();
    let mut editing = use_signal(|| None::<u32>);
    let mut name = use_signal(String::new);
    let mut color = use_signal(|| LabelColor::Blue.value().to_string());
    let mut error = use_signal(|| None::<String>);
    let mut confirm = use_signal(|| false);
    let labels = dir.labels.clone();
    let entries: Vec<ListEntry> = labels
        .iter()
        .map(|l| {
            let n = dir.people.iter().filter(|p| p.labels.contains(&l.id)).count();
            ListEntry { icon: Icon::Tag, title: l.name.clone(), subtitle: format!("{} · {n} people", l.color.label()), meta: String::new() }
        })
        .collect();
    let ids: Vec<u32> = labels.iter().map(|l| l.id).collect();
    let reset = move || {
        let (mut e, mut n, mut c, mut err) = (editing, name, color, error);
        e.set(None);
        n.set(String::new());
        c.set(LabelColor::Blue.value().to_string());
        err.set(None);
    };
    let save = move |_| {
        let id = editing();
        match state.update_directory(|d| d.save_label(id, &name(), LabelColor::parse(&color()))) {
            Ok(_) => {
                toaster.success(if id.is_some() { "Label saved." } else { "Label added." });
                reset();
            }
            Err(e) => error.set(Some(e)),
        }
    };
    let color_options: Vec<(String, String, Tone)> = LabelColor::ALL.iter().map(|c| (c.value().to_string(), c.label().to_string(), c.tone())).collect();
    let preview = LabelColor::parse(&color());
    let preview_text = if name().trim().is_empty() { "Preview".to_string() } else { name() };
    rsx! {
        Dialog {
            open: true,
            onclose: move |_| onclose.call(()),
            title: "Labels",
            description: "Labels of the congregation. Put them on people, and filter by them in All users.",
            actions: rsx! { ActionButton { action: Action::Done, onclick: move |_| onclose.call(()) } },
            Stack { gap: Gap::Lg,
                if entries.is_empty() {
                    Text { tone: TextTone::Muted, "No labels yet." }
                } else {
                    ItemList {
                        items: entries,
                        on_select: move |i: usize| {
                            if let Some(l) = state.directory().labels.get(i).cloned() {
                                editing.set(Some(l.id));
                                name.set(l.name);
                                color.set(l.color.value().to_string());
                                error.set(None);
                            }
                        },
                    }
                }
                Card { title: if editing().is_some() { "Change label" } else { "New label" }, icon: Icon::Tag,
                    Grid { cols: 2,
                        Field { label: "Name", required: true, error: error(),
                            TextInput { value: name(), placeholder: "For example: Needs a ride", oninput: move |v| { name.set(v); error.set(None); } }
                        }
                        Field { label: "Colour",
                            ColorPicker { value: color(), options: color_options, onchange: move |v| color.set(v) }
                        }
                    }
                    Cluster { gap: Gap::Sm,
                        Badge { tone: preview.tone(), icon: Icon::Tag, "{preview_text}" }
                    }
                    Cluster { gap: Gap::Sm,
                        ActionButton { action: if editing().is_some() { Action::Save } else { Action::Add }, variant: ButtonVariant::Secondary, label: if editing().is_some() { "Save label" } else { "Add label" }, onclick: save }
                        if editing().is_some() {
                            ActionButton { action: Action::Cancel, onclick: move |_| reset() }
                            ActionButton { action: Action::Delete, onclick: move |_| confirm.set(true) }
                        }
                    }
                }
                Text { tone: TextTone::Small, "{ids.len()} labels." }
            }
        }
        ConfirmDialog {
            open: confirm(),
            title: "Delete this label?",
            message: "The label is taken off everyone who has it.",
            confirm: Action::Delete,
            oncancel: move |_| confirm.set(false),
            onconfirm: move |_| {
                if let Some(id) = editing() {
                    state.update_directory(|d| d.delete_label(id));
                }
                confirm.set(false);
                reset();
                toaster.success("Label deleted.");
            },
        }
    }
}
