use crate::platform::{biometric_label, biometric_register, biometric_verify, store_biometric, stored_biometric, SIMULATED};
use crate::state::use_app_state;
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
    let mut remember = use_signal(|| false);
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
    let bio = biometric_label();
    // On, for this congregation on this device: chosen in this session, or remembered.
    let bio_enabled = current.as_ref().is_some_and(|c| c.biometric || stored_biometric(&c.id).is_some());
    let items: Vec<SwitcherItem> = congregations
        .iter()
        .map(|c| SwitcherItem { id: c.id.clone(), name: c.name.clone(), kind_icon: c.mode.icon(), kind_label: c.mode.label() })
        .collect();
    let name = state.account().map(|a| a.display_name).unwrap_or_default();

    rsx! {
        AuthLayout {
            title: "Unlock your congregation",
            subtitle: "Hello {name}. Your data is encrypted; the congregation password opens it on this device.",
            footer: rsx! {
                Cluster { justify: Justify::Center, gap: Gap::Xs,
                    "Not you?"
                    TextLink { onclick: move |_| { state.sign_out(); nav.replace(Route::Landing {}); }, "Sign out" }
                }
            },
            Form {
                onsubmit: move |_| {
                    let Some(id) = selected() else { return };
                    if password().is_empty() {
                        error.set(Some("Enter the congregation password.".into()));
                        return;
                    }
                    let pw = password();
                    if remember() && state.password_matches(&id, &pw) {
                        // Set up the biometric unlock first (the browser asks for the fingerprint or
                        // face), then unlock: unlocking leaves this page, which would stop the setup.
                        let account = state.account();
                        let username = account.as_ref().map(|a| a.username.clone()).unwrap_or_default();
                        let display = account.map(|a| a.display_name).unwrap_or_default();
                        checking.set(true);
                        spawn(async move {
                            match biometric_register(&username, &display).await {
                                Ok(credential) => {
                                    store_biometric(&id, Some(&credential));
                                    state.set_biometric(&id, true);
                                    toaster.success("Biometric unlock is on for this device.");
                                }
                                Err(e) => toaster.error(format!("Biometric unlock is not on: {e}")),
                            }
                            checking.set(false);
                            if state.unlock(&id, &pw, false).is_ok() {
                                nav.replace(Route::Dashboard {});
                            }
                        });
                        return;
                    }
                    match state.unlock(&id, &pw, false) {
                        Ok(()) => { nav.replace(Route::Dashboard {}); }
                        Err(e) => error.set(Some(e.to_string())),
                    }
                },
                actions: rsx! { ActionButton { action: Action::Unlock, kind: ButtonKind::Submit, full_width: true } },
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
                if bio_enabled {
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
                                    // The browser's passkey prompt on the web; simulated in the apps.
                                    let credential = stored_biometric(&id).unwrap_or_else(|| SIMULATED.into());
                                    let ok = biometric_verify(&credential).await;
                                    checking.set(false);
                                    if ok && state.unlock_with_biometrics(&id) {
                                        nav.replace(Route::Dashboard {});
                                    } else {
                                        toaster.error("Biometric unlock did not work. Use the password.");
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
                Field { label: "Congregation password", required: true,
                    PasswordInput { value: password(), oninput: move |v| { password.set(v); error.set(None); }, autocomplete: "off", invalid: error().is_some(), autofocus: true }
                }
                if !bio_enabled {
                    if let Some(label) = bio {
                        Switch {
                            checked: remember(),
                            onchange: move |v| remember.set(v),
                            label: "Use {label} next time",
                            description: "Unlock this congregation on this device without typing the password.",
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

