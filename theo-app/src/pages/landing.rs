use crate::state::use_app_state;
use crate::{Redirect, Route};
use dioxus::prelude::*;
use theo_ui::*;
use theo_ui::Action; // explicit: dioxus::prelude also exports an `Action`

/// Public landing page, web only (R-063). Signed-in users go straight to `/app`;
/// installed apps (desktop, mobile, and the web app installed as a PWA) open on the log-in screen.
#[component]
pub fn Landing() -> Element {
    let mut state = use_app_state();
    let nav = navigator();
    if state.session.read().is_some() {
        return rsx! { Redirect { to: Route::Dashboard {} } };
    }
    if !crate::platform::IS_WEB || crate::platform::is_installed_web_app() {
        return rsx! { Redirect { to: Route::Login {} } };
    }

    rsx! {
        PublicLayout {
            brand: "Theo Manager",
            footer: "Theo Manager · Free · Source-available",
            actions: rsx! {
                ActionButton { action: Action::SignIn, variant: ButtonVariant::Ghost, onclick: move |_| { nav.push(Route::Login {}); } }
                ActionButton { action: Action::SignUp, onclick: move |_| { nav.push(Route::Signup {}); } }
            },
            Hero {
                eyebrow: "Private by design · Works on every device",
                title: "Your congregation, organised and private.",
                subtitle: "Meetings, duties, territories, public talks, field service groups and reports in one place. Everything is encrypted on your device before it is saved.",
                note: "No trackers. No ads. Lose the congregation password and nobody can read the data, not even us.",
                actions: rsx! {
                    ActionButton { action: Action::SignUp, label: "Get started", onclick: move |_| { nav.push(Route::Signup {}); } }
                    ActionButton {
                        action: Action::TryDemo,
                        onclick: move |_| {
                            state.start_demo();
                            nav.push(Route::Unlock {});
                        },
                    }
                },
                visual: rsx! { Illustration { kind: IllustrationKind::Devices } },
            }

            Section {
                eyebrow: "How it works",
                title: "Up and running in three steps",
                subtitle: "Start alone on one device, or invite the brothers who help you in the cloud.",
                Grid { cols: 3,
                    StepCard { number: 1, icon: Icon::SignUp, title: "Create your account", text: "An account on this device, or on your congregation's own server." }
                    StepCard { number: 2, icon: Icon::Congregation, title: "Create the congregation", text: "Name, meeting times and where the data lives. The app gives you a strong congregation password." }
                    StepCard { number: 3, icon: Icon::Assignment, title: "Plan and share", text: "Schedule meetings and duties, then invite publishers to see their own assignments." }
                }
            }

            Section {
                muted: true,
                eyebrow: "Everything in one place",
                title: "Built for the work a congregation really does",
                Grid { cols: 3,
                    FeatureCard { icon: Icon::Meetings, title: "Midweek and weekend meetings", text: "Parts, students and assistants, chairmen and readers, with fair suggestions and conflict warnings." }
                    FeatureCard { icon: Icon::Audio, title: "Duties", text: "Attendants, audio, video, stage, microphones, Zoom and cleaning. Define your own duty types." }
                    FeatureCard { icon: Icon::Map, title: "Territories", text: "Maps, boundaries, check-out and return, and coverage history." }
                    FeatureCard { icon: Icon::PublicTalk, title: "Public talks", text: "Incoming and outgoing speakers, talk catalogue and other congregations." }
                    FeatureCard { icon: Icon::Footprints, title: "Field service", text: "Groups, monthly reports and totals. Publishers can send their own report." }
                    FeatureCard { icon: Icon::Mobile, title: "Every device", text: "Web, Windows, macOS, Linux, iOS and Android, with the same look everywhere." }
                }
            }

            Section {
                eyebrow: "Privacy",
                title: "Encrypted before it leaves your device",
                subtitle: "Zero-knowledge, end-to-end encryption with post-quantum key exchange. A server only ever stores data it cannot read.",
                Grid { cols: 2,
                    Illustration { kind: IllustrationKind::Encryption }
                    CheckList {
                        items: vec![
                            "Only people with the congregation password, or a key their role allows, can read the data.".to_string(),
                            "Publishers get only what their role needs: schedules, their assignments, their own report.".to_string(),
                            "Unlock with your fingerprint, face or Windows Hello on devices that support it.".to_string(),
                            "Post-quantum hybrid keys (ML-KEM) protect shared keys against future quantum computers.".to_string(),
                            "No trackers, no ads, no analytics.".to_string(),
                        ],
                    }
                }
            }

            Section {
                muted: true,
                eyebrow: "Your choice",
                title: "Local or cloud, per congregation",
                Grid { cols: 2,
                    CompareCard {
                        icon: Icon::Local,
                        title: "Local",
                        tagline: "One person, one device",
                        points: vec![
                            "Works completely offline".to_string(),
                            "Nothing leaves this device".to_string(),
                            "Move to the cloud later if you need it".to_string(),
                        ],
                    }
                    CompareCard {
                        icon: Icon::Cloud,
                        title: "Cloud",
                        tagline: "The whole congregation, on any device",
                        highlighted: true,
                        points: vec![
                            "Invite brothers and publishers with a link".to_string(),
                            "Roles decide who sees and changes what".to_string(),
                            "Your own server; it only stores encrypted data".to_string(),
                        ],
                    }
                }
            }

            Section {
                eyebrow: "At a glance",
                title: "A dashboard you arrange yourself",
                subtitle: "Drag widgets to reorder them and resize them to fit what matters to you.",
                Grid { cols: 3,
                    Card { title: "Meeting attendance", icon: Icon::Chart,
                        BarChart { data: crate::mock::attendance(), highlight_last: true }
                    }
                    Card { title: "Active publishers", icon: Icon::Users,
                        StatTile { label: "This month", value: "84", icon: Icon::Users, delta: "+3", trend: Trend::Up }
                        Sparkline { values: crate::mock::publishers_trend() }
                    }
                    Card { title: "My assignments", icon: Icon::Assignment,
                        ItemList { items: crate::mock::my_assignments() }
                    }
                }
            }
        }
    }
}
