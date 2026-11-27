//! Join a Cloud congregation with an invite link (REQUIREMENTS.md 5.6).
//!
//! On the web the link opens this page, and its fragment (`#s=...&t=...`) is read once and
//! removed from the address bar. The installed apps have no web address: the link is pasted here.

use crate::state::use_app_state;
use crate::Route;
use dioxus::prelude::*;
use theo_core::invites::InviteLink;
use theo_ui::*;
use theo_ui::Action; // explicit: dioxus::prelude also exports an `Action`

fn account_segments() -> Vec<Segment> {
    vec![
        Segment { value: "existing".into(), label: "I have an account".into(), icon: Some(Icon::User) },
        Segment { value: "new".into(), label: "New account".into(), icon: Some(Icon::SignUp) },
    ]
}

/// `/join`
#[component]
pub fn Join() -> Element {
    let mut state = use_app_state();
    let nav = navigator();
    let toaster = use_toaster();
    let mut text = use_signal(|| crate::platform::take_location_fragment().unwrap_or_default());
    let mut mode = use_signal(|| "existing".to_string());
    let mut name = use_signal(String::new);
    let mut username = use_signal(String::new);
    let mut password = use_signal(String::new);
    let mut confirm = use_signal(String::new);
    let mut error = use_signal(|| None::<String>);
    let mut submitted = use_signal(|| false);
    let mut busy = use_signal(|| false);

    let link = InviteLink::parse(&text());
    let signed_in_there = link.is_some() && state.signed_in_to_cloud();
    let me = state.account().map(|a| a.username).unwrap_or_default();
    let new = mode() == "new";
    let score = crate::password::strength(&password(), &[username().trim(), name().trim()]);
    let s = submitted() && !signed_in_there;
    let link_err = (submitted() && link.is_none()).then(|| "Paste the whole invite link you were sent.".to_string());
    let name_err = (s && new && name().trim().is_empty()).then(|| "Enter your name.".to_string());
    let user_err = (s && username().trim().len() < 3).then(|| "Use at least 3 characters.".to_string());
    let pass_err = (s && if new { score < 3 } else { password().is_empty() }).then(|| if new { "Use a stronger password (at least “Strong”).".to_string() } else { "Enter your password.".to_string() });
    let confirm_err = (s && new && confirm() != password()).then(|| "The passwords do not match.".to_string());
    let invalid = link_err.is_some() || name_err.is_some() || user_err.is_some() || pass_err.is_some() || confirm_err.is_some();

    rsx! {
        AuthLayout {
            title: "Join a congregation",
            subtitle: "You were invited to use Theo Manager with your congregation.",
            top: if crate::platform::IS_WEB {
                Some(rsx! { TextLink { icon: Icon::Home, onclick: move |_| { nav.push(Route::Landing {}); }, "Home" } })
            } else {
                None
            },
            footer: rsx! {
                Cluster { justify: Justify::Center, gap: Gap::Xs,
                    "No invite?"
                    TextLink { onclick: move |_| { nav.push(Route::Login {}); }, "Log in" }
                }
            },
            Form {
                onsubmit: move |_| {
                    submitted.set(true);
                    // The errors above are from the last render; check again with this click's values.
                    let Some(link) = InviteLink::parse(&text()) else { return };
                    let there = state.signed_in_to_cloud();
                    let new = mode() == "new";
                    let bad = !there
                        && (username().trim().len() < 3
                            || (new && (name().trim().is_empty() || crate::password::strength(&password(), &[username().trim(), name().trim()]) < 3 || confirm() != password()))
                            || (!new && password().is_empty()));
                    if bad {
                        return;
                    }
                    busy.set(true);
                    error.set(None);
                    spawn(async move {
                        // Let the spinner appear before the (deliberately slow) key derivation.
                        theo_ui::sleep(std::time::Duration::from_millis(60)).await;
                        let new_name = (new && !there).then(|| name.read().clone());
                        let result = state.join(&link, &username(), &password(), new_name.as_deref()).await;
                        busy.set(false);
                        match result {
                            Ok(_) => {
                                toaster.success("You joined the congregation.");
                                nav.replace(Route::Unlock {});
                            }
                            Err(e) => error.set(Some(e.to_string())),
                        }
                    });
                },
                full_width_actions: true,
                actions: rsx! { ActionButton { action: Action::Invite, label: "Join", kind: ButtonKind::Submit, full_width: true, loading: busy(), disabled: invalid && submitted() } },
                if let Some(e) = error() {
                    Alert { tone: Tone::Error, "{e}" }
                }
                Field { label: "Invite link", required: true, error: link_err.clone(),
                    help: "Sent to you privately by an administrator. It works once.",
                    TextInput { value: text(), oninput: move |v| { text.set(v); error.set(None); }, icon: Icon::Invitation, placeholder: "https://…/join#t=…", invalid: link_err.is_some() }
                }
                if !crate::cloud::available() {
                    Alert { tone: Tone::Warning, "This version of the app has no Cloud service, so it cannot join Cloud congregations." }
                }
                if link.is_some() {
                    if signed_in_there {
                        Alert { tone: Tone::Info, "You join as @{me}." }
                    } else {
                        Segmented { value: mode(), options: account_segments(), onchange: move |v| { mode.set(v); submitted.set(false); } }
                        if new {
                            Field { label: "Your name", required: true, error: name_err.clone(),
                                TextInput { value: name(), oninput: move |v| name.set(v), autocomplete: "name", invalid: name_err.is_some() }
                            }
                        }
                        Field { label: "Username", required: true, error: user_err.clone(), help: if new { "Not your real name: the server can see usernames." } else { "" },
                            TextInput { value: username(), oninput: move |v| username.set(v), autocomplete: "username", icon: Icon::User, invalid: user_err.is_some() }
                        }
                        Field { label: "Password", required: true, error: pass_err.clone(), help: if new { "For your account only. You never need the congregation password with this invite, unless you get full access." } else { "" },
                            PasswordInput {
                                value: password(),
                                oninput: move |v| password.set(v),
                                autocomplete: if new { "new-password" } else { "current-password" },
                                strength: if new { Some(score) } else { None },
                                invalid: pass_err.is_some(),
                            }
                        }
                        if new {
                            Field { label: "Repeat password", required: true, error: confirm_err.clone(),
                                PasswordInput { value: confirm(), oninput: move |v| confirm.set(v), autocomplete: "new-password", invalid: confirm_err.is_some() }
                            }
                        }
                    }
                }
            }
        }
    }
}
