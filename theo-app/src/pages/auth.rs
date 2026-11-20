use crate::state::{use_app_state, AccountKind};
use crate::{next_after_sign_in, Redirect, Route};
use dioxus::prelude::*;
use theo_ui::*;
use theo_ui::Action; // explicit: dioxus::prelude also exports an `Action`

fn location_segments() -> Vec<Segment> {
    vec![
        Segment { value: "device".into(), label: "This device".into(), icon: Some(Icon::Local) },
        Segment { value: "cloud".into(), label: "Cloud server".into(), icon: Some(Icon::Cloud) },
    ]
}

/// Log in.
#[component]
pub fn Login() -> Element {
    let mut state = use_app_state();
    let nav = navigator();
    let mut username = use_signal(String::new);
    let mut password = use_signal(String::new);
    let mut location = use_signal(|| "device".to_string());
    let mut server = use_signal(String::new);
    let mut error = use_signal(|| None::<String>);
    let mut submitted = use_signal(|| false);

    if state.session.read().is_some() {
        return rsx! { Redirect { to: next_after_sign_in(&state) } };
    }

    let user_err = (submitted() && username().trim().is_empty()).then(|| "Enter your username.".to_string());
    let pass_err = (submitted() && password().is_empty()).then(|| "Enter your password.".to_string());

    rsx! {
        AuthLayout {
            title: "Welcome back",
            subtitle: "Log in to open your congregations.",
            // "Home" only exists in the browser; installed apps have no landing page (R-063).
            top: if crate::platform::IS_WEB {
                Some(rsx! { TextLink { icon: Icon::Home, onclick: move |_| { nav.push(Route::Landing {}); }, "Home" } })
            } else {
                None
            },
            footer: rsx! {
                Stack { gap: Gap::Sm, align: Align::Center,
                    Cluster { justify: Justify::Center, gap: Gap::Xs,
                        "No account yet?"
                        TextLink { onclick: move |_| { nav.push(Route::Signup {}); }, "Sign up" }
                    }
                    ActionButton {
                        action: Action::TryDemo,
                        variant: ButtonVariant::Ghost,
                        onclick: move |_| {
                            state.start_demo();
                            nav.push(Route::Unlock {});
                        },
                    }
                }
            },
            Form {
                onsubmit: move |_| {
                    submitted.set(true);
                    if username().trim().is_empty() || password().is_empty() {
                        return;
                    }
                    match state.sign_in(&username(), &password()) {
                        Ok(()) => { nav.replace(next_after_sign_in(&state)); }
                        Err(e) => error.set(Some(e.to_string())),
                    }
                },
                full_width_actions: true,
                actions: rsx! { ActionButton { action: Action::SignIn, kind: ButtonKind::Submit, full_width: true } },
                if let Some(e) = error() {
                    Alert { tone: Tone::Error, "{e}" }
                }
                Segmented { value: location(), options: location_segments(), onchange: move |v| location.set(v) }
                if location() == "cloud" {
                    Field { label: "Server address", help: "The address your congregation's admin gave you.",
                        TextInput { value: server(), oninput: move |v| server.set(v), kind: InputKind::Url, icon: Icon::Server, placeholder: "https://db.example.org" }
                    }
                }
                Field { label: "Username", required: true, error: user_err.clone(),
                    TextInput { value: username(), oninput: move |v| username.set(v), autocomplete: "username", icon: Icon::User, invalid: user_err.is_some(), autofocus: true }
                }
                Field { label: "Password", required: true, error: pass_err.clone(),
                    PasswordInput { value: password(), oninput: move |v| password.set(v), autocomplete: "current-password", invalid: pass_err.is_some() }
                }
            }
        }
    }
}

/// Sign up.
#[component]
pub fn Signup() -> Element {
    let mut state = use_app_state();
    let nav = navigator();
    let mut name = use_signal(String::new);
    let mut username = use_signal(String::new);
    let mut password = use_signal(String::new);
    let mut confirm = use_signal(String::new);
    let mut location = use_signal(|| "device".to_string());
    let mut server = use_signal(String::new);
    let mut error = use_signal(|| None::<String>);
    let mut submitted = use_signal(|| false);

    if state.session.read().is_some() {
        return rsx! { Redirect { to: next_after_sign_in(&state) } };
    }

    let score = crate::mock::strength(&password());
    let s = submitted();
    let name_err = (s && name().trim().is_empty()).then(|| "Enter your name.".to_string());
    let user_err = (s && username().trim().len() < 3).then(|| "Use at least 3 characters.".to_string());
    let pass_err = (s && score < 3).then(|| "Use a stronger password (at least “Strong”).".to_string());
    let confirm_err = (s && confirm() != password()).then(|| "The passwords do not match.".to_string());
    let server_err = (s && location() == "cloud" && !server().starts_with("https://")).then(|| "Enter an https:// address.".to_string());

    rsx! {
        AuthLayout {
            title: "Create your account",
            subtitle: "Then you create your congregation, or join one.",
            // "Home" only exists in the browser; installed apps have no landing page (R-063).
            top: if crate::platform::IS_WEB {
                Some(rsx! { TextLink { icon: Icon::Home, onclick: move |_| { nav.push(Route::Landing {}); }, "Home" } })
            } else {
                None
            },
            footer: rsx! {
                Cluster { justify: Justify::Center, gap: Gap::Xs,
                    "Already have an account?"
                    TextLink { onclick: move |_| { nav.push(Route::Login {}); }, "Log in" }
                }
            },
            Form {
                onsubmit: move |_| {
                    submitted.set(true);
                    let invalid = name().trim().is_empty()
                        || username().trim().len() < 3
                        || crate::mock::strength(&password()) < 3
                        || confirm() != password()
                        || (location() == "cloud" && !server().starts_with("https://"));
                    if invalid {
                        return;
                    }
                    let kind = if location() == "cloud" { AccountKind::Cloud { server: server() } } else { AccountKind::Device };
                    match state.sign_up(&username(), &name(), &password(), kind) {
                        Ok(()) => { nav.replace(Route::NewCongregation {}); }
                        Err(e) => error.set(Some(e.to_string())),
                    }
                },
                full_width_actions: true,
                actions: rsx! { ActionButton { action: Action::SignUp, kind: ButtonKind::Submit, full_width: true } },
                if let Some(e) = error() {
                    Alert { tone: Tone::Error, "{e}" }
                }
                Field { label: "Where should your account live?",
                    Segmented { value: location(), options: location_segments(), onchange: move |v| location.set(v) }
                }
                if location() == "cloud" {
                    Field { label: "Server address", required: true, error: server_err.clone(), help: "Your congregation's SurrealDB server.",
                        TextInput { value: server(), oninput: move |v| server.set(v), kind: InputKind::Url, icon: Icon::Server, placeholder: "https://db.example.org", invalid: server_err.is_some() }
                    }
                }
                Field { label: "Your name", required: true, error: name_err.clone(),
                    TextInput { value: name(), oninput: move |v| name.set(v), autocomplete: "name", invalid: name_err.is_some(), autofocus: true }
                }
                Field { label: "Username", required: true, error: user_err.clone(), help: "Not your real name: the server can see usernames.",
                    TextInput { value: username(), oninput: move |v| username.set(v), autocomplete: "username", icon: Icon::User, invalid: user_err.is_some() }
                }
                Field { label: "Password", required: true, error: pass_err.clone(), help: "For your account only. The congregation gets its own password.",
                    PasswordInput { value: password(), oninput: move |v| password.set(v), autocomplete: "new-password", strength: Some(score), invalid: pass_err.is_some() }
                }
                Field { label: "Repeat password", required: true, error: confirm_err.clone(),
                    PasswordInput { value: confirm(), oninput: move |v| confirm.set(v), autocomplete: "new-password", invalid: confirm_err.is_some() }
                }
            }
        }
    }
}
