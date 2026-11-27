use crate::platform::biometric_label;
use crate::state::{use_app_state, Secret};
use crate::{Redirect, Route};
use dioxus::prelude::*;
use theo_ui::*;
use theo_ui::Action; // explicit: dioxus::prelude also exports an `Action`

/// Choose a congregation and unlock it with its password or biometrics.
#[component]
pub fn Unlock() -> Element {
    let mut state = use_app_state();
    let nav = navigator();
    let toaster = use_toaster();
    let congregations = state.my_congregations();
    let initial = state
        .preselect
        .read()
        .clone()
        .filter(|id| congregations.iter().any(|c| &c.id == id))
        .or_else(|| congregations.first().map(|c| c.id.clone()));
    let mut selected = use_signal(|| initial.clone());
    let mut password = use_signal(String::new);
    // Suggested: "use fingerprint next time" starts on where the device can do it.
    let mut remember = use_signal(|| true);
    let mut error = use_signal(|| None::<String>);
    let mut checking = use_signal(|| false);

    if state.session.read().is_none() {
        return rsx! { Redirect { to: Route::Login {} } };
    }
    if state.unlocked.read().is_some() {
        return rsx! { Redirect { to: Route::Dashboard {} } };
    }
    if congregations.is_empty() {
        return rsx! { Redirect { to: Route::NewCongregation {} } };
    }

    let current = congregations.iter().find(|c| Some(&c.id) == selected().as_ref()).cloned();
    // What opening it asks: its congregation password, the account password (a Cloud congregation
    // after a restart), or nothing (the signed-in account opens it).
    let secret = current.as_ref().map_or(Secret::Congregation, |c| state.secret_for(&c.id));
    let needs_password = secret != Secret::None;
    // Passkey unlock (6.5): where this device can keep the key sealed by a passkey.
    let bio = biometric_label().filter(|_| current.as_ref().is_some_and(|c| state.passkey_available(&c.id)));
    let bio_enabled = current.as_ref().is_some_and(|c| state.has_passkey(&c.id));
    let (password_label, password_help) = match secret {
        Secret::Account => ("Your account password", "This Cloud congregation opens with your account."),
        _ => ("Congregation password", "Shared in person by your congregation's administrator."),
    };
    let items: Vec<SwitcherItem> = congregations
        .iter()
        .map(|c| SwitcherItem { id: c.id.clone(), name: c.name.clone(), kind_icon: c.mode.icon(), kind_label: c.mode.label() })
        .collect();
    let name = state.account().map(|a| a.display_name).unwrap_or_default();

    rsx! {
        AuthLayout {
            title: "Unlock your congregation",
            subtitle: "Hello {name}. Your data is encrypted; choose the congregation to open.",
            footer: rsx! {
                Cluster { justify: Justify::Center, gap: Gap::Xs,
                    "Not you?"
                    TextLink { onclick: move |_| { state.sign_out(); nav.replace(Route::Landing {}); }, "Sign out" }
                }
            },
            Form {
                onsubmit: move |_| {
                    let Some(id) = selected() else { return };
                    if !state.needs_password(&id) {
                        let remember = remember() && bio.is_some() && !bio_enabled;
                        checking.set(true);
                        spawn(async move {
                            let result = state.unlock(&id, "", remember).await;
                            checking.set(false);
                            let notice = state.notice.write().take();
                            if let Some(m) = notice {
                                if m.starts_with("Passkey unlock is on") { toaster.success(m) } else { toaster.error(m) }
                            }
                            match result {
                                Ok(()) => { nav.replace(Route::Dashboard {}); }
                                Err(e) => error.set(Some(e.to_string())),
                            }
                        });
                        return;
                    }
                    if password().is_empty() {
                        error.set(Some(format!("Enter {}.", password_label.to_lowercase().replace("congregation", "the congregation"))));
                        return;
                    }
                    let pw = password();
                    // With "use passkey next time", the passkey is made right after the password
                    // opened the congregation, before this page closes.
                    let remember = remember() && bio.is_some() && !bio_enabled;
                    checking.set(true);
                    spawn(async move {
                        // Let the spinner appear before the (deliberately slow) key derivation.
                        theo_ui::sleep(std::time::Duration::from_millis(60)).await;
                        let result = state.unlock(&id, &pw, remember).await;
                        checking.set(false);
                        let notice = state.notice.write().take();
                        if let Some(m) = notice {
                            if m.starts_with("Passkey unlock is on") { toaster.success(m) } else { toaster.error(m) }
                        }
                        match result {
                            Ok(()) => { nav.replace(Route::Dashboard {}); }
                            Err(e) => error.set(Some(e.to_string())),
                        }
                    });
                },
                full_width_actions: true,
                actions: rsx! { ActionButton { action: Action::Unlock, kind: ButtonKind::Submit, full_width: true, loading: checking() } },
                Field { label: "Congregation",
                    if items.len() > 1 {
                        WorkspaceSwitcher {
                            items: items.clone(),
                            active: selected(),
                            onselect: move |id| { selected.set(Some(id)); error.set(None); password.set(String::new()); },
                            create_label: "Create congregation",
                            oncreate: move |_| { nav.push(Route::NewCongregation {}); },
                        }
                    } else if let Some(c) = current.clone() {
                        ItemList { items: vec![ListEntry { icon: c.mode.icon(), title: c.name.clone(), subtitle: c.mode.label(), meta: String::new() }] }
                    }
                }
                if bio_enabled && needs_password {
                    if let Some(label) = bio {
                        Button {
                            variant: ButtonVariant::Secondary,
                            icon: Icon::Fingerprint,
                            loading: checking(),
                            full_width: true,
                            onclick: move |_| {
                                let Some(id) = selected() else { return };
                                checking.set(true);
                                spawn(async move {
                                    // The browser asks for the fingerprint, face or PIN; the passkey's
                                    // answer opens the sealed key.
                                    let result = state.unlock_with_passkey(&id).await;
                                    checking.set(false);
                                    match result {
                                        Ok(()) => { nav.replace(Route::Dashboard {}); }
                                        Err(e) => toaster.error(e.to_string()),
                                    }
                                });
                            },
                            "Unlock with {label}"
                        }
                        Divider { label: "or use the password" }
                    }
                }
                if let Some(e) = error() {
                    Alert { tone: Tone::Error, "{e}" }
                }
                if needs_password {
                    Field { label: password_label, required: true, help: password_help,
                        PasswordInput {
                            value: password(),
                            oninput: move |v| { password.set(v); error.set(None); },
                            autocomplete: if secret == Secret::Account { "current-password" } else { "off" },
                            invalid: error().is_some(),
                            autofocus: !bio_enabled,
                        }
                    }
                } else {
                    Text { tone: TextTone::Muted, "Your account opens this congregation. No password is needed." }
                }
                if !bio_enabled {
                    if let Some(label) = bio {
                        Switch {
                            checked: remember(),
                            onchange: move |v| remember.set(v),
                            label: "Use {label} next time",
                            description: if secret == Secret::Congregation { "Open this congregation on this device with your fingerprint, face or PIN instead of its password." } else { "Open your Cloud congregations on this device with your fingerprint, face or PIN instead of your account password." },
                        }
                    }
                }
                if current.as_ref().is_some_and(|c| c.owner == crate::mock::DEMO_USER) {
                    Alert { tone: Tone::Neutral, title: "Demo",
                        "The demo congregations use the password "
                        Code { "{crate::mock::DEMO_CONGREGATION_PASSWORD}" }
                        "."
                    }
                }
            }
        }
    }
}

