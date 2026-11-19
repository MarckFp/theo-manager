//! `/app/me/assignments`: everything assigned to me, as a timeline (from the home widget).
//! Rules: `docs/model/m15-field-service-meetings.md`, "Notices and My assignments".

use crate::assignments::{Assignment, NoticeKind};
use crate::people::parse_date;
use crate::reports::month_label;
use crate::service_meetings::{weekday_of, WEEKDAYS};
use crate::state::use_app_state;
use crate::Route;
use dioxus::prelude::*;
use theo_ui::*;
use theo_ui::Action; // explicit: dioxus::prelude also exports an `Action`

fn notice_tag(k: NoticeKind) -> Tag {
    match k {
        NoticeKind::New => Tag::new("New", Tone::Info).with_icon(Icon::Sparkles),
        NoticeKind::Changed => Tag::new("Changed", Tone::Warning).with_icon(Icon::Edit),
        NoticeKind::Cancelled => Tag::new("Cancelled", Tone::Error).with_icon(Icon::Close),
    }
}

/// `/app/me/assignments`
#[component]
pub fn MyAssignmentsPage() -> Element {
    let mut state = use_app_state();
    let navigator = navigator();
    let prefs = state.preferences();
    let congregation = state.active_congregation().map(|c| c.name).unwrap_or_default();
    let mut tab = use_signal(|| "upcoming".to_string());
    let back = ("Home".to_string(), EventHandler::new(move |_| { navigator.push(Route::Dashboard {}); }));
    let Some(me) = state.me() else {
        return rsx! {
            Page { title: "My assignments", back,
                EmptyState { icon: Icon::Assignment, title: "Your account is not linked to a person", description: "Ask an administrator to link your account to your person in Users." }
            }
        };
    };
    let dir = state.directory();
    let today = crate::people::today_iso();
    let notices = dir.my_notices(me);
    let all = dir.all_my_assignments(me);
    let (upcoming, mut past): (Vec<Assignment>, Vec<Assignment>) = all.into_iter().partition(|a| a.date.as_str() >= today.as_str());
    past.reverse();
    let (n_up, n_past) = (upcoming.len(), past.len());
    let showing_past = tab() == "past";
    let list = if showing_past { past } else { upcoming };
    let when = |a: &Assignment| {
        let time = a.time.as_ref().map(|t| prefs.format_hhmm(t));
        [time, Some(a.detail.clone())].into_iter().flatten().collect::<Vec<_>>().join(" · ")
    };
    let items: Vec<TimelineItem> = list
        .iter()
        .enumerate()
        .map(|(i, a)| {
            let (y, m, d) = parse_date(&a.date).unwrap_or((0, 0, 0));
            let weekday = weekday_of(&a.date).map(|w| WEEKDAYS[w as usize][..3].to_string()).unwrap_or_default();
            TimelineItem {
                id: format!("{}-{}", a.record, a.date),
                group: month_label((y, m)),
                day: d.to_string(),
                weekday,
                title: a.kind.label().into(),
                subtitle: when(a),
                icon: Icon::Footprints,
                tag: notices.iter().find(|n| n.record == a.record && n.kind != NoticeKind::Cancelled).map(|n| notice_tag(n.kind)),
                muted: showing_past,
                highlight: !showing_past && i == 0,
            }
        })
        .collect();
    // Cancelled assignments are no longer in the list: their notices say so.
    let cancelled: Vec<ListEntry> = notices
        .iter()
        .filter(|n| n.kind == NoticeKind::Cancelled)
        .map(|n| {
            let day = parse_date(&n.date).map(|d| prefs.format_date(d)).unwrap_or_else(|| n.date.clone());
            ListEntry {
                icon: Icon::Close,
                title: n.what.label().into(),
                subtitle: format!("{day}{} · {}", n.time.as_ref().map(|t| format!(" · {}", prefs.format_hhmm(t))).unwrap_or_default(), n.detail),
                meta: "Cancelled".into(),
            }
        })
        .collect();
    let has_notices = !notices.is_empty();

    rsx! {
        Page {
            title: "My assignments",
            subtitle: "{congregation} · Everything assigned to you",
            back,
            StickyBar {
                Segmented {
                    value: tab(),
                    options: vec![
                        Segment { value: "upcoming".into(), label: format!("Coming up ({n_up})"), icon: Some(Icon::Calendar) },
                        Segment { value: "past".into(), label: format!("Past ({n_past})"), icon: Some(Icon::History) },
                    ],
                    onchange: move |v: String| tab.set(v),
                }
            }
            if has_notices {
                Alert { tone: Tone::Info, title: "Changes for you",
                    Stack { gap: Gap::Sm,
                        Text { "New and changed assignments are marked below." }
                        if !cancelled.is_empty() {
                            ItemList { items: cancelled }
                        }
                        Cluster {
                            ActionButton { action: Action::Done, variant: ButtonVariant::Secondary, label: "Got it", onclick: move |_| state.update_directory(|d| d.mark_notices_seen(me)) }
                        }
                    }
                }
            }
            if items.is_empty() {
                EmptyState {
                    icon: Icon::Assignment,
                    title: if showing_past { "No past assignments" } else { "Nothing coming up" },
                    description: "Assignments appear here when they are sent to you.",
                }
            } else {
                Timeline { items }
            }
        }
    }
}
