use crate::state::{use_app_state, CongregationDraft, MeetingSlot, StorageMode};
use crate::{Redirect, Route};
use dioxus::prelude::*;
use theo_ui::*;
use theo_ui::Action; // explicit: dioxus::prelude also exports an `Action`

const STEPS: [&str; 5] = ["Congregation", "Meetings", "Storage", "Security", "Review"];

fn options(list: &[(&str, &str)]) -> Vec<SelectOption> {
    list.iter().map(|(v, l)| SelectOption::new(*v, *l)).collect()
}

fn languages() -> Vec<SelectOption> {
    options(&[
        ("en", "English"),
        ("es", "Español"),
        ("fr", "Français"),
        ("de", "Deutsch"),
        ("it", "Italiano"),
        ("pt", "Português"),
        ("nl", "Nederlands"),
        ("da", "Dansk"),
        ("pl", "Polski"),
        ("ro", "Română"),
    ])
}

fn time_zones() -> Vec<SelectOption> {
    options(&[
        ("Europe/London", "Europe/London"),
        ("Europe/Madrid", "Europe/Madrid"),
        ("Europe/Paris", "Europe/Paris"),
        ("Europe/Berlin", "Europe/Berlin"),
        ("Europe/Copenhagen", "Europe/Copenhagen"),
        ("America/New_York", "America/New_York"),
        ("America/Chicago", "America/Chicago"),
        ("America/Los_Angeles", "America/Los_Angeles"),
        ("America/Mexico_City", "America/Mexico_City"),
        ("America/Sao_Paulo", "America/Sao_Paulo"),
        ("UTC", "UTC"),
    ])
}

fn weekdays() -> Vec<SelectOption> {
    options(&[("Monday", "Monday"), ("Tuesday", "Tuesday"), ("Wednesday", "Wednesday"), ("Thursday", "Thursday"), ("Friday", "Friday")])
}

fn weekend_days() -> Vec<SelectOption> {
    options(&[("Saturday", "Saturday"), ("Sunday", "Sunday")])
}

/// Create a congregation: the minimum data, then Local or Cloud, then the congregation password.
#[component]
pub fn NewCongregation() -> Element {
    let mut state = use_app_state();
    let nav = navigator();
    let toaster = use_toaster();

    let mut step = use_signal(|| 0usize);
    let mut tried = use_signal(|| false);
    // Step 1
    let mut name = use_signal(String::new);
    let mut number = use_signal(String::new);
    let mut language = use_signal(|| "en".to_string());
    let mut tz = use_signal(|| "Europe/London".to_string());
    // About you: the creator becomes a person of the congregation (All users), its administrator.
    let account_name = state.account().map(|a| a.display_name).unwrap_or_default();
    let mut my_first = use_signal(|| account_name.split_whitespace().next().unwrap_or("").to_string());
    let mut my_last = use_signal(|| account_name.split_whitespace().skip(1).collect::<Vec<_>>().join(" "));
    let mut my_gender = use_signal(String::new);
    // Step 2
    let mut mid_day = use_signal(|| "Thursday".to_string());
    let mut mid_time = use_signal(|| "19:00".to_string());
    let mut wk_day = use_signal(|| "Sunday".to_string());
    let mut wk_time = use_signal(|| "10:00".to_string());
    // Step 3
    let mut mode = use_signal(|| "local".to_string());
    // A Cloud congregation lives on the signed-in Cloud account's server: no address to type.
    let cloud_server = state.cloud_server();
    let has_cloud = cloud_server.is_some();
    // Step 4
    let mut generated = use_signal(crate::password::generate);
    let mut own = use_signal(|| false);
    let mut custom = use_signal(String::new);
    let mut custom2 = use_signal(String::new);
    let mut saved = use_signal(|| false);
    let mut creating = use_signal(|| false);

    if state.session.read().is_none() {
        return rsx! { Redirect { to: Route::Login {} } };
    }

    let has_any = !state.my_congregations().is_empty();
    let score = crate::password::strength(&custom(), &[name().trim()]);
    let t = tried();
    let name_err = (t && step() == 0 && name().trim().is_empty()).then(|| "Enter the congregation name.".to_string());
    let first_err = (t && step() == 0 && my_first().trim().is_empty()).then(|| "Enter your first name.".to_string());
    let last_err = (t && step() == 0 && my_last().trim().is_empty()).then(|| "Enter your last name.".to_string());
    let gender_err = (t && step() == 0 && my_gender().is_empty()).then(|| "Choose one.".to_string());
    let custom_err = (t && step() == 3 && own() && !crate::password::meets_rules(&custom())).then(|| "The password does not meet all the rules below.".to_string());
    let custom2_err = (t && step() == 3 && own() && custom2() != custom()).then(|| "The passwords do not match.".to_string());
    let saved_err = (t && step() == 3 && !saved()).then(|| "Confirm that you saved the password.".to_string());

    let step_valid = move |s: usize| -> bool {
        match s {
            0 => !name().trim().is_empty() && !my_first().trim().is_empty() && !my_last().trim().is_empty() && !my_gender().is_empty(),
            2 => mode() == "local" || has_cloud,
            3 => saved() && if own() { crate::password::meets_rules(&custom()) && custom2() == custom() } else { !generated().is_empty() },
            _ => true,
        }
    };

    let storage = if mode() == "cloud" { StorageMode::Cloud { server: cloud_server.clone().unwrap_or_default() } } else { StorageMode::Local };
    let create_server = cloud_server.clone().unwrap_or_default();
    let final_password = if own() { custom() } else { generated() };
    let is_last = step() + 1 == STEPS.len();

    rsx! {
        WizardLayout {
            title: "Create a congregation",
            subtitle: "Only the basics now. You can add everything else later.",
            steps: STEPS.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
            current: step(),
            top: rsx! {
                if has_any {
                    TextLink { icon: Icon::Close, onclick: move |_| { nav.go_back(); }, "Cancel" }
                } else {
                    TextLink { icon: Icon::SignOut, onclick: move |_| { state.sign_out(); nav.replace(Route::Landing {}); }, "Sign out" }
                }
            },
            Form {
                onsubmit: move |_| {
                    tried.set(true);
                    if !step_valid(step()) {
                        return;
                    }
                    tried.set(false);
                    if step() + 1 < STEPS.len() {
                        step.set(step() + 1);
                        return;
                    }
                    let mode = if mode() == "cloud" { StorageMode::Cloud { server: create_server.clone() } } else { StorageMode::Local };
                    let password = if own() { custom() } else { generated() };
                    let draft = CongregationDraft {
                        name: name(),
                        number: number(),
                        language: language(),
                        time_zone: tz(),
                        midweek: MeetingSlot { day: mid_day(), time: mid_time() },
                        weekend: MeetingSlot { day: wk_day(), time: wk_time() },
                        mode,
                        password,
                        my_first_name: my_first(),
                        my_last_name: my_last(),
                        my_gender: my_gender(),
                    };
                    creating.set(true);
                    spawn(async move {
                        // Let the spinner appear before the (deliberately slow) key derivation.
                        theo_ui::sleep(std::time::Duration::from_millis(60)).await;
                        let result = state.create_congregation(draft).await;
                        creating.set(false);
                        match result {
                            Ok(_) => {
                                toaster.success(format!("{} is ready.", name().trim()));
                                nav.replace(Route::Dashboard {});
                            }
                            Err(e) => toaster.error(format!("The congregation could not be created: {e}")),
                        }
                    });
                },
                actions: rsx! {
                    if step() > 0 {
                        ActionButton { action: Action::Back, onclick: move |_| { tried.set(false); step.set(step() - 1); } }
                    }
                    ActionButton { action: if is_last { Action::Finish } else { Action::Next }, kind: ButtonKind::Submit, loading: creating() }
                },
                match step() {
                    0 => rsx! {
                        Heading { level: HeadingLevel::H3, "About the congregation" }
                        Field { label: "Congregation name", required: true, error: name_err.clone(),
                            TextInput { value: name(), oninput: move |v| name.set(v), placeholder: "Riverside", invalid: name_err.is_some(), autofocus: true }
                        }
                        Field { label: "Congregation number", help: "Optional. The number your congregation uses, if any.",
                            TextInput { value: number(), oninput: move |v| number.set(v), kind: InputKind::Number }
                        }
                        Grid { cols: 2,
                            Field { label: "Meeting language", required: true, help: "Used to import meeting content.",
                                Select { value: language(), options: languages(), onchange: move |v| language.set(v) }
                            }
                            Field { label: "Time zone", required: true,
                                Select { value: tz(), options: time_zones(), onchange: move |v| tz.set(v) }
                            }
                        }
                        Heading { level: HeadingLevel::H3, "About you" }
                        Text { tone: TextTone::Muted, "You are added to the congregation's users as its administrator. You can complete your details later." }
                        Grid { cols: 2,
                            Field { label: "Your first name", required: true, error: first_err.clone(),
                                TextInput { value: my_first(), oninput: move |v| my_first.set(v), invalid: first_err.is_some() }
                            }
                            Field { label: "Your last name", required: true, error: last_err.clone(),
                                TextInput { value: my_last(), oninput: move |v| my_last.set(v), invalid: last_err.is_some() }
                            }
                        }
                        Field { label: "Gender", required: true, error: gender_err.clone(),
                            Select {
                                value: my_gender(),
                                invalid: gender_err.is_some(),
                                options: vec![SelectOption::new("", "Choose…"), SelectOption::new("male", "Male"), SelectOption::new("female", "Female")],
                                onchange: move |v| my_gender.set(v),
                            }
                        }
                    },
                    1 => rsx! {
                        Heading { level: HeadingLevel::H3, "Meeting times" }
                        Text { tone: TextTone::Muted, "Schedules are created for these days. Special weeks can be changed later." }
                        Grid { cols: 2,
                            Field { label: "Midweek meeting", required: true,
                                Select { value: mid_day(), options: weekdays(), onchange: move |v| mid_day.set(v) }
                            }
                            Field { label: "Time", required: true,
                                TextInput { value: mid_time(), oninput: move |v| mid_time.set(v), kind: InputKind::Time, icon: Icon::Clock }
                            }
                            Field { label: "Weekend meeting", required: true,
                                Select { value: wk_day(), options: weekend_days(), onchange: move |v| wk_day.set(v) }
                            }
                            Field { label: "Time", required: true,
                                TextInput { value: wk_time(), oninput: move |v| wk_time.set(v), kind: InputKind::Time, icon: Icon::Clock }
                            }
                        }
                    },
                    2 => rsx! {
                        Heading { level: HeadingLevel::H3, "Where should the data live?" }
                        ChoiceCards {
                            value: mode(),
                            onchange: move |v| mode.set(v),
                            choices: vec![
                                Choice { value: "local".into(), title: "Local".into(), icon: Icon::Local,
                                    description: "Only on this device. Works offline. Best if you organise everything yourself.".into() },
                                Choice { value: "cloud".into(), title: "Cloud".into(), icon: Icon::Cloud,
                                    description: "On your congregation's server, encrypted. Invite others and use any device.".into() },
                            ],
                        }
                        if mode() == "cloud" {
                            if let Some(url) = cloud_server.clone() {
                                Alert { tone: Tone::Info, title: "On {url}",
                                    "The server of your account. It stores only encrypted data, and only the members of the congregation can reach it."
                                }
                            } else {
                                Alert { tone: Tone::Warning, title: "A Cloud account is needed",
                                    "Cloud congregations live on the server of your account. Sign up or log in with a Cloud account to create one."
                                }
                            }
                        }
                    },
                    3 => rsx! {
                        Heading { level: HeadingLevel::H3, "Congregation password" }
                        Alert { tone: Tone::Warning, title: "Nobody can recover this password",
                            "It encrypts all congregation data. If it is lost, the data cannot be opened by anyone, including us. Write it down and keep it safe."
                        }
                        if !own() {
                            Stack { gap: Gap::Sm,
                                Text { tone: TextTone::Strong, "Your generated password" }
                                Code { "{generated}" }
                                Cluster {
                                    ActionButton { action: Action::Regenerate, onclick: move |_| generated.set(crate::password::generate()) }
                                }
                            }
                        }
                        Switch {
                            checked: own(),
                            onchange: move |v| own.set(v),
                            label: "Choose my own password",
                            description: "A generated passphrase is stronger and easier to remember.",
                        }
                        if own() {
                            Field { label: "Password", required: true, error: custom_err.clone(),
                                PasswordInput { value: custom(), oninput: move |v| custom.set(v), autocomplete: "new-password", strength: Some(score), invalid: custom_err.is_some() }
                                RuleChecklist { rules: crate::password::rules(&custom()) }
                            }
                            Field { label: "Repeat password", required: true, error: custom2_err.clone(),
                                PasswordInput { value: custom2(), oninput: move |v| custom2.set(v), autocomplete: "new-password", invalid: custom2_err.is_some() }
                            }
                        }
                        Field { label: "Confirmation", error: saved_err.clone(),
                            Checkbox { checked: saved(), onchange: move |v| saved.set(v), label: "I have written down the congregation password. I understand that nobody can recover it." }
                        }
                    },
                    _ => rsx! {
                        Heading { level: HeadingLevel::H3, "Check and create" }
                        ItemList {
                            items: vec![
                                ListEntry { icon: Icon::Congregation, title: name().trim().to_string(), subtitle: if number().is_empty() { "No number".into() } else { format!("Number {}", number()) }, meta: language().to_uppercase() },
                                ListEntry { icon: Icon::Meetings, title: format!("Midweek: {} {}", mid_day(), mid_time()), subtitle: format!("Weekend: {} {}", wk_day(), wk_time()), meta: tz() },
                                ListEntry { icon: storage.icon(), title: storage.label(), subtitle: match &storage { StorageMode::Cloud { server } => server.clone(), StorageMode::Local => "Nothing leaves this device".into() }, meta: String::new() },
                                ListEntry { icon: Icon::Key, title: "Congregation password".into(), subtitle: if own() { "Your own password".into() } else { "Generated passphrase".into() }, meta: format!("{} characters", final_password.chars().count()) },
                            ],
                        }
                    },
                }
            }
        }
    }
}
