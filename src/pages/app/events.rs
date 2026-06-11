use dioxus::prelude::*;
use dioxus_i18n::t;
use surrealdb::types::RecordId;

use crate::database::use_db;
use crate::models::event::{CongregationEvent, CongregationEventData, EventType, today_str};

// ── Helpers ───────────────────────────────────────────────────────────────────

fn rid_str(id: &RecordId) -> String {
    format!(
        "{}:{}",
        id.table,
        match &id.key {
            surrealdb::types::RecordIdKey::String(k) => k.clone(),
            surrealdb::types::RecordIdKey::Number(n) => n.to_string(),
            _ => String::new(),
        }
    )
}

pub fn event_type_label(et: &EventType) -> String {
    match et {
        EventType::CircuitAssembly => t!("event-type-circuit-assembly"),
        EventType::Memorial => t!("event-type-memorial"),
        EventType::CircuitOverseerVisit => t!("event-type-circuit-overseer"),
        EventType::RegionalConvention => t!("event-type-regional-convention"),
        EventType::Other => t!("event-type-other"),
    }
}

pub fn event_display_title(e: &CongregationEvent) -> String {
    match &e.event_type {
        EventType::Other => e
            .title
            .clone()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| t!("event-type-other")),
        other => event_type_label(other),
    }
}

fn event_badge_cls(et: &EventType) -> &'static str {
    match et {
        EventType::CircuitAssembly => "bg-blue-100 text-blue-700",
        EventType::Memorial => "bg-purple-100 text-purple-700",
        EventType::CircuitOverseerVisit => "bg-green-100 text-green-700",
        EventType::RegionalConvention => "bg-orange-100 text-orange-700",
        EventType::Other => "bg-gray-100 text-gray-600",
    }
}

// ── Form state ────────────────────────────────────────────────────────────────

#[derive(Clone, Default)]
struct EventFormData {
    start_date: String,
    end_date: String,
    event_type: EventType,
    title: String,
    description: String,
}

impl EventFormData {
    fn from_event(e: &CongregationEvent) -> Self {
        Self {
            start_date: e.start_date.clone(),
            end_date: e.end_date.clone(),
            event_type: e.event_type.clone(),
            title: e.title.clone().unwrap_or_default(),
            description: e.description.clone().unwrap_or_default(),
        }
    }

    fn into_model_data(self) -> CongregationEventData {
        CongregationEventData {
            start_date: self.start_date,
            end_date: self.end_date,
            title: if matches!(self.event_type, EventType::Other) {
                Some(self.title).filter(|s| !s.is_empty())
            } else {
                None
            },
            description: Some(self.description).filter(|s| !s.is_empty()),
            event_type: self.event_type,
        }
    }

    fn is_valid(&self) -> bool {
        !self.start_date.is_empty()
            && !self.end_date.is_empty()
            && self.end_date >= self.start_date
    }
}

// ── Main page ─────────────────────────────────────────────────────────────────

#[component]
pub fn AppEvents() -> Element {
    let db_signal = use_db();

    let mut events_res = use_resource(move || async move {
        let Some(db) = db_signal.read().db.clone() else {
            return vec![];
        };
        CongregationEvent::all_prune(&db).await.unwrap_or_default()
    });

    let mut bootstrapped = use_signal(|| false);
    use_effect(move || {
        if *bootstrapped.peek() {
            return;
        }
        bootstrapped.set(true);
        events_res.restart();
    });

    let mut modal_open = use_signal(|| false);
    let mut edit_target: Signal<Option<CongregationEvent>> = use_signal(|| None);
    let mut pending_delete: Signal<Option<RecordId>> = use_signal(|| None);

    let is_loading = events_res.read().is_none();
    let events: Vec<CongregationEvent> = events_res().unwrap_or_default();

    rsx! {
        div { class: "space-y-5 w-full pb-10",

            // ── Header ────────────────────────────────────────────────────
            h1 { class: "text-2xl font-bold text-gray-900", {t!("page-events")} }

            if is_loading {
                div { class: "flex justify-center py-20 text-gray-400",
                    span { class: "text-sm animate-pulse", {t!("priv-loading")} }
                }
            } else if events.is_empty() {
                div { class: "bg-white rounded-xl border border-gray-200 px-6 py-14 text-center",
                    p { class: "text-4xl mb-3", "📅" }
                    p { class: "font-medium text-gray-600", {t!("empty-events-title")} }
                    p { class: "text-sm text-gray-400 mt-1", {t!("empty-events-desc")} }
                }
            } else {
                div { class: "space-y-2",
                    for event in events.clone() {
                        {
                            let e_id = event.id.clone();
                            let e_edit = event.clone();
                            let confirming = pending_delete
                                .read()
                                .as_ref()
                                .and_then(|pd| e_id.as_ref().map(|eid| rid_str(pd) == rid_str(eid)))
                                .unwrap_or(false);
                            rsx! {
                                EventCard {
                                    event,
                                    confirming_delete: confirming,
                                    on_edit: move |_| {
                                        edit_target.set(Some(e_edit.clone()));
                                        modal_open.set(true);
                                    },
                                    on_delete_request: move |_| {
                                        pending_delete.set(e_id.clone());
                                    },
                                    on_delete_confirm: move |_| {
                                        let rid = pending_delete.read().clone();
                                        pending_delete.set(None);
                                        if let Some(rid) = rid {
                                            spawn(async move {
                                                if let Some(db) = db_signal.read().db.clone() {
                                                    let _ = CongregationEvent::delete(&db, rid).await;
                                                }
                                                events_res.restart();
                                            });
                                        }
                                    },
                                    on_delete_cancel: move |_| pending_delete.set(None),
                                }
                            }
                        }
                    }
                }
            }
        }

        // ── Floating add button ───────────────────────────────────────────
        button {
            class: "fixed bottom-20 right-6 md:bottom-6 w-14 h-14 bg-primary-600 text-white rounded-full shadow-xl hover:bg-primary-700 active:scale-95 transition-all flex items-center justify-center text-2xl z-20 select-none",
            onclick: move |_| {
                edit_target.set(None);
                modal_open.set(true);
            },
            "＋"
        }

        // ── Modal ─────────────────────────────────────────────────────────
        if *modal_open.read() {
            EventModal {
                event: edit_target.read().clone(),
                on_close: move |_| modal_open.set(false),
                on_saved: move |_| {
                    events_res.restart();
                    modal_open.set(false);
                },
            }
        }
    }
}

// ── EventCard ─────────────────────────────────────────────────────────────────

#[component]
fn EventCard(
    event: CongregationEvent,
    confirming_delete: bool,
    on_edit: Callback<()>,
    on_delete_request: Callback<()>,
    on_delete_confirm: Callback<()>,
    on_delete_cancel: Callback<()>,
) -> Element {
    let badge_cls = format!(
        "inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium {}",
        event_badge_cls(&event.event_type)
    );
    let type_label = event_type_label(&event.event_type);
    let display_title = event_display_title(&event);
    let date_range = if event.start_date == event.end_date {
        event.start_date.clone()
    } else {
        format!("{} – {}", event.start_date, event.end_date)
    };

    rsx! {
        div { class: "bg-white rounded-xl border border-gray-200 px-4 py-3",
            div { class: "flex items-start justify-between gap-3",
                div { class: "flex-1 min-w-0",
                    div { class: "flex flex-wrap items-center gap-2 mb-1",
                        span { class: "{badge_cls}", "{type_label}" }
                        span { class: "text-sm font-semibold text-gray-900", "{display_title}" }
                    }
                    p { class: "text-xs text-gray-500 tabular-nums", "{date_range}" }
                    if let Some(ref desc) = event.description {
                        if !desc.is_empty() {
                            p { class: "text-sm text-gray-500 mt-1 line-clamp-2",
                                "{desc}"
                            }
                        }
                    }
                }
                div { class: "flex items-center gap-1 shrink-0",
                    if confirming_delete {
                        span { class: "text-xs text-red-600 font-medium mr-1",
                            {t!("event-delete-confirm")}
                        }
                        button {
                            class: "px-2.5 py-1 text-xs bg-red-600 text-white rounded-lg hover:bg-red-700 transition-colors font-medium",
                            onclick: move |_| on_delete_confirm.call(()),
                            {t!("btn-delete")}
                        }
                        button {
                            class: "px-2.5 py-1 text-xs border border-gray-200 rounded-lg text-gray-600 hover:bg-gray-50 transition-colors",
                            onclick: move |_| on_delete_cancel.call(()),
                            {t!("btn-cancel")}
                        }
                    } else {
                        button {
                            class: "p-1.5 text-gray-400 hover:text-primary-600 hover:bg-primary-50 rounded-lg transition-colors",
                            onclick: move |_| on_edit.call(()),
                            "✏️"
                        }
                        button {
                            class: "p-1.5 text-gray-400 hover:text-red-600 hover:bg-red-50 rounded-lg transition-colors",
                            onclick: move |_| on_delete_request.call(()),
                            "🗑️"
                        }
                    }
                }
            }
        }
    }
}

// ── EventModal ────────────────────────────────────────────────────────────────

#[component]
fn EventModal(
    event: Option<CongregationEvent>,
    on_close: Callback<()>,
    on_saved: Callback<()>,
) -> Element {
    let db_signal = use_db();
    let is_edit = event.is_some();
    let existing_id = event.as_ref().and_then(|e| e.id.clone());
    let today = today_str();
    let event_for_init = event.clone();

    let mut form = use_signal(move || match &event_for_init {
        Some(e) => EventFormData::from_event(e),
        None => EventFormData {
            start_date: today.clone(),
            end_date: today.clone(),
            ..Default::default()
        },
    });

    let mut submitting = use_signal(|| false);
    let mut save_error: Signal<Option<String>> = use_signal(|| None);

    let f = form.read().clone();
    let is_other = matches!(f.event_type, EventType::Other);
    let valid = f.is_valid();
    let sub = *submitting.read();

    let on_submit = use_callback(move |_: Event<MouseData>| {
        let fd = form.read().clone();
        if !fd.is_valid() {
            return;
        }
        let data = fd.into_model_data();
        let eid = existing_id.clone();
        submitting.set(true);
        save_error.set(None);
        spawn(async move {
            let Some(db) = db_signal.read().db.clone() else {
                submitting.set(false);
                return;
            };
            let result = if let Some(rid) = eid {
                CongregationEvent::update(&db, rid, data).await.map(|_| ())
            } else {
                CongregationEvent::create(&db, data).await.map(|_| ())
            };
            submitting.set(false);
            match result {
                Ok(_) => on_saved.call(()),
                Err(e) => save_error.set(Some(e.to_string())),
            }
        });
    });

    rsx! {
        div {
            class: "fixed inset-0 z-50 flex items-end lg:items-center justify-center lg:p-4 bg-black/40",
            onclick: move |_| on_close.call(()),
            div {
                class: "relative bg-white w-full rounded-t-2xl lg:rounded-2xl shadow-2xl flex flex-col max-h-[92vh] lg:max-h-[85vh] lg:max-w-lg overflow-hidden",
                onclick: move |e| e.stop_propagation(),

                // Drag handle (mobile)
                div { class: "lg:hidden shrink-0 flex justify-center pt-3 pb-2",
                    div { class: "w-10 h-1 bg-gray-300 rounded-full" }
                }

                // Header
                div { class: "shrink-0 flex items-center justify-between px-4 lg:px-6 pb-3 lg:pt-5 lg:pb-4 border-b border-gray-100",
                    h2 { class: "text-base lg:text-lg font-semibold text-gray-900",
                        {if is_edit { t!("event-edit-title") } else { t!("event-new-title") }}
                    }
                    button {
                        class: "ml-4 p-1.5 text-gray-400 hover:text-gray-600 rounded hover:bg-gray-100 transition-colors",
                        onclick: move |_| on_close.call(()),
                        "✕"
                    }
                }

                // Body
                div { class: "flex-1 overflow-y-auto px-4 lg:px-6 py-4 space-y-4",
                    if let Some(err) = save_error.read().clone() {
                        div { class: "bg-red-50 border border-red-200 text-red-700 text-sm px-3 py-2 rounded-lg",
                            "{err}"
                        }
                    }

                    // Event type
                    div { class: "space-y-1",
                        label { class: "block text-xs font-semibold text-gray-500 uppercase tracking-wide",
                            {t!("event-form-type")}
                        }
                        select {
                            class: "w-full px-3 py-2 text-sm border border-gray-200 rounded-lg bg-white focus:outline-none focus:ring-2 focus:ring-primary-500",
                            onchange: move |e| {
                                let new_type = match e.value().as_str() {
                                    "circuit_assembly" => EventType::CircuitAssembly,
                                    "memorial" => EventType::Memorial,
                                    "circuit_overseer" => EventType::CircuitOverseerVisit,
                                    "regional_convention" => EventType::RegionalConvention,
                                    _ => EventType::Other,
                                };
                                let was_other = matches!(form.read().event_type, EventType::Other);
                                form.write().event_type = new_type.clone();
                                // Clear custom title when switching away from Other
                                if was_other && !matches!(new_type, EventType::Other) {
                                    form.write().title = String::new();
                                }
                            },
                            option {
                                value: "circuit_assembly",
                                selected: matches!(f.event_type, EventType::CircuitAssembly),
                                {t!("event-type-circuit-assembly")}
                            }
                            option {
                                value: "memorial",
                                selected: matches!(f.event_type, EventType::Memorial),
                                {t!("event-type-memorial")}
                            }
                            option {
                                value: "circuit_overseer",
                                selected: matches!(f.event_type, EventType::CircuitOverseerVisit),
                                {t!("event-type-circuit-overseer")}
                            }
                            option {
                                value: "regional_convention",
                                selected: matches!(f.event_type, EventType::RegionalConvention),
                                {t!("event-type-regional-convention")}
                            }
                            option {
                                value: "other",
                                selected: matches!(f.event_type, EventType::Other),
                                {t!("event-type-other")}
                            }
                        }
                    }

                    // Custom title — only editable when type is Other
                    if is_other {
                        div { class: "space-y-1",
                            label { class: "block text-xs font-semibold text-gray-500 uppercase tracking-wide",
                                {t!("event-form-title")}
                            }
                            input {
                                r#type: "text",
                                class: "w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500",
                                value: f.title.clone(),
                                oninput: move |e| form.write().title = e.value(),
                            }
                        }
                    }

                    // Dates
                    div { class: "grid grid-cols-2 gap-3",
                        div { class: "space-y-1",
                            label { class: "block text-xs font-semibold text-gray-500 uppercase tracking-wide",
                                {t!("event-form-start-date")}
                            }
                            input {
                                r#type: "date",
                                class: "w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500",
                                value: f.start_date.clone(),
                                onchange: move |e| form.write().start_date = e.value(),
                            }
                        }
                        div { class: "space-y-1",
                            label { class: "block text-xs font-semibold text-gray-500 uppercase tracking-wide",
                                {t!("event-form-end-date")}
                            }
                            input {
                                r#type: "date",
                                class: "w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500",
                                value: f.end_date.clone(),
                                onchange: move |e| form.write().end_date = e.value(),
                            }
                        }
                    }

                    // Description
                    div { class: "space-y-1",
                        label { class: "block text-xs font-semibold text-gray-500 uppercase tracking-wide",
                            {t!("event-form-description")}
                        }
                        textarea {
                            class: "w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500 resize-none",
                            rows: "3",
                            value: f.description.clone(),
                            oninput: move |e| form.write().description = e.value(),
                        }
                    }
                }

                // Footer
                div { class: "shrink-0 px-4 lg:px-6 py-3 lg:py-4 border-t border-gray-100 flex gap-2 lg:gap-3 lg:justify-end",
                    button {
                        class: "flex-1 lg:flex-none px-4 lg:px-5 py-2.5 lg:py-2 text-sm border border-gray-200 rounded-xl text-gray-700 hover:bg-gray-50 transition-colors",
                        disabled: sub,
                        onclick: move |_| on_close.call(()),
                        {t!("btn-cancel")}
                    }
                    button {
                        class: if valid { "flex-1 lg:flex-none px-4 lg:px-5 py-2.5 lg:py-2 text-sm bg-primary-600 text-white rounded-xl hover:bg-primary-700 disabled:opacity-50 transition-colors font-medium" } else { "flex-1 lg:flex-none px-4 lg:px-5 py-2.5 lg:py-2 text-sm bg-gray-200 text-gray-400 rounded-xl cursor-not-allowed font-medium" },
                        disabled: sub || !valid,
                        onclick: move |e| on_submit.call(e),
                        {if sub { t!("btn-connecting") } else { t!("btn-save") }}
                    }
                }
            }
        }
    }
}

