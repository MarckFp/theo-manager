use dioxus::prelude::*;
use dioxus_i18n::t;
use dioxus_primitives::tabs::{TabContent, TabList, TabTrigger, Tabs};
use surrealdb::types::RecordId;

use crate::database::{use_crypto, use_db, ls_get};
use crate::models::congregation::{Congregation, NameFormat};
use crate::models::territory::{
    Territory, TerritoryAddress, TerritoryAddressData, TerritoryAssignment,
    TerritoryAssignmentData, TerritoryData, TerritoryRequest, TerritoryRequestData,
};
use crate::models::user::{User, UserType};
use crate::pages::app::user::format_name;

// ── Platform helpers ──────────────────────────────────────────────────────────

#[cfg(target_arch = "wasm32")]
fn today_iso() -> String {
    let d = js_sys::Date::new_0();
    format!(
        "{:04}-{:02}-{:02}",
        d.get_full_year(),
        d.get_month() + 1,
        d.get_date()
    )
}

#[cfg(not(target_arch = "wasm32"))]
fn today_iso() -> String {
    "2026-06-11".to_string()
}

#[cfg(target_arch = "wasm32")]
fn days_ago_iso(days: u32) -> String {
    // Use a JS expression to compute date string
    let js = format!(
        "var d=new Date(Date.now()-{}*86400000);\
         d.getFullYear()+'-'+String(d.getMonth()+1).padStart(2,'0')+'-'+String(d.getDate()).padStart(2,'0')",
        days
    );
    if let Ok(val) = js_sys::eval(&js) {
        if let Some(s) = val.as_string() {
            return s;
        }
    }
    today_iso()
}

#[cfg(not(target_arch = "wasm32"))]
fn days_ago_iso(_days: u32) -> String {
    today_iso()
}

#[cfg(target_arch = "wasm32")]
fn current_year() -> i32 {
    js_sys::Date::new_0().get_full_year() as i32
}

#[cfg(not(target_arch = "wasm32"))]
fn current_year() -> i32 {
    2026
}

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

fn parse_rid(s: &str) -> Option<RecordId> {
    RecordId::parse_simple(s).ok()
}

fn months_since(date: &str) -> i32 {
    let today = today_iso();
    let ty: i32 = today.get(0..4).and_then(|s| s.parse().ok()).unwrap_or(2026);
    let tm: i32 = today.get(5..7).and_then(|s| s.parse().ok()).unwrap_or(1);
    let dy: i32 = date.get(0..4).and_then(|s| s.parse().ok()).unwrap_or(ty);
    let dm: i32 = date.get(5..7).and_then(|s| s.parse().ok()).unwrap_or(tm);
    (ty - dy) * 12 + (tm - dm)
}

fn is_assignable(u: &User) -> bool {
    !matches!(u.user_type, UserType::Student)
}

fn boundary_to_json(boundary: &[Vec<f64>]) -> String {
    if boundary.is_empty() {
        return "[]".to_string();
    }
    let pairs: Vec<String> = boundary
        .iter()
        .map(|p| {
            if p.len() >= 2 {
                format!("[{},{}]", p[0], p[1])
            } else {
                "[0,0]".to_string()
            }
        })
        .collect();
    format!("[{}]", pairs.join(","))
}

fn addresses_to_json(addresses: &[TerritoryAddress]) -> String {
    let items: Vec<String> = addresses
        .iter()
        .map(|a| {
            format!(
                r#"{{"lat":{},"lng":{},"description":{:?}}}"#,
                a.lat, a.lng, a.description
            )
        })
        .collect();
    format!("[{}]", items.join(","))
}

fn map_center(boundary: &[Vec<f64>]) -> (f64, f64) {
    if boundary.is_empty() {
        return (20.0, 0.0);
    }
    let lat_sum: f64 = boundary.iter().filter_map(|p| p.first().copied()).sum();
    let lng_sum: f64 = boundary.iter().filter_map(|p| p.get(1).copied()).sum();
    let n = boundary.len() as f64;
    (lat_sum / n, lng_sum / n)
}

// ── TerritoryModal ────────────────────────────────────────────────────────────

#[component]
fn TerritoryModal(title: String, on_close: Callback<()>, children: Element) -> Element {
    rsx! {
        div {
            class: "fixed inset-0 z-50 flex items-end sm:items-center justify-center sm:p-4 bg-black/40",
            onclick: move |_| on_close.call(()),
            div {
                class: "relative bg-white w-full rounded-t-2xl sm:rounded-2xl shadow-2xl \
                        flex flex-col max-h-[92vh] sm:max-h-[85vh] sm:max-w-lg overflow-hidden",
                onclick: move |e| e.stop_propagation(),
                div { class: "shrink-0 flex items-center justify-between px-5 py-4 border-b border-gray-100",
                    h2 { class: "font-semibold text-gray-900", "{title}" }
                    button {
                        class: "p-1 text-gray-400 hover:text-gray-600 rounded hover:bg-gray-100 transition-colors",
                        onclick: move |_| on_close.call(()),
                        "✕"
                    }
                }
                div { class: "flex-1 overflow-y-auto px-5 py-4", {children} }
            }
        }
    }
}

// ── LeafletMap ────────────────────────────────────────────────────────────────

#[component]
fn LeafletMap(
    map_id: String,
    center_lat: f64,
    center_lng: f64,
    zoom: u8,
    boundary_json: String,
    addresses_json: String,
    draw_mode: ReadOnlySignal<bool>,
    on_point: Callback<String>,
) -> Element {
    let id = map_id.clone();
    let bnd = boundary_json.clone();
    let addr = addresses_json.clone();

    use_effect(move || {
        // Read draw_mode inside the effect so Dioxus re-runs when it changes.
        let draw = draw_mode();
        let id = id.clone();
        let bnd = bnd.clone();
        let addr = addr.clone();
        spawn(async move {
            gloo_timers::future::TimeoutFuture::new(80).await;
            let js = format!(
                r#"(function(){{
var mid={id:?};
function initMap(){{
    var el=document.getElementById('leaflet-'+mid);
    if(!el)return;
    var mk='_theo_map_'+mid;
    if(window[mk]){{window[mk].remove();window[mk]=null;}}
    var map=L.map('leaflet-'+mid).setView([{clat},{clng}],{zoom});
    window[mk]=map;
    L.tileLayer('https://{{s}}.tile.openstreetmap.org/{{z}}/{{x}}/{{y}}.png',{{
        attribution:'&copy; OpenStreetMap contributors',maxZoom:19
    }}).addTo(map);
    var bnd={bnd};
    var pk='_theo_poly_'+mid;
    window[pk]=bnd.slice();
    if(bnd.length>2){{
        L.polygon(bnd,{{color:'#4f46e5',weight:2,fillOpacity:0.15}}).addTo(map);
        map.fitBounds(L.polygon(bnd).getBounds().pad(0.1));
    }}else if(bnd.length===0){{
        map.locate({{setView:true,maxZoom:14}});
    }}
    var addrs={addr};
    addrs.forEach(function(a){{
        var icon=L.divIcon({{className:'',html:'<div style="background:#4f46e5;width:10px;height:10px;border-radius:50%;border:2px solid white;box-shadow:0 1px 3px rgba(0,0,0,.4)"></div>',iconSize:[10,10],iconAnchor:[5,5]}});
        L.marker([a.lat,a.lng],{{icon:icon}}).bindTooltip(a.description).addTo(map);
    }});
    if({draw}){{
        map.off('click');
        map.on('click',function(e){{
            var lat=e.latlng.lat.toFixed(6);
            var lng=e.latlng.lng.toFixed(6);
            window[pk]=window[pk]||[];
            window[pk].push([parseFloat(lat),parseFloat(lng)]);
            if(window['_theo_pl_'+mid]){{map.removeLayer(window['_theo_pl_'+mid]);window['_theo_pl_'+mid]=null;}}
            (window['_theo_mks_'+mid]||[]).forEach(function(m){{map.removeLayer(m);}});
            window['_theo_mks_'+mid]=[];
            if(window[pk].length===1){{
                window['_theo_mks_'+mid]=[L.circleMarker(window[pk][0],{{radius:6,color:'#4f46e5',fillColor:'#4f46e5',fillOpacity:0.9,weight:2}}).addTo(map)];
            }}else if(window[pk].length===2){{
                window['_theo_pl_'+mid]=L.polyline(window[pk],{{color:'#4f46e5',weight:2,dashArray:'6,4'}}).addTo(map);
            }}else{{
                window['_theo_pl_'+mid]=L.polygon(window[pk],{{color:'#4f46e5',weight:2,fillOpacity:0.1,dashArray:'5'}}).addTo(map);
            }}
            dioxus.send(lat+','+lng);
        }});
    }}
}}
function ensureLeaflet(cb){{
    if(window.L){{cb();return;}}
    if(!document.querySelector('link[href*="leaflet"]')){{
        var lnk=document.createElement('link');
        lnk.rel='stylesheet';
        lnk.href='https://unpkg.com/leaflet@1.9.4/dist/leaflet.css';
        document.head.appendChild(lnk);
    }}
    if(!document.querySelector('script[src*="leaflet"]')){{
        var s=document.createElement('script');
        s.src='https://unpkg.com/leaflet@1.9.4/dist/leaflet.js';
        s.onload=cb;
        document.head.appendChild(s);
    }} else {{
        var t=setInterval(function(){{if(window.L){{clearInterval(t);cb();}}}},50);
    }}
}}
ensureLeaflet(initMap);
}})();"#,
                id = id,
                clat = center_lat,
                clng = center_lng,
                zoom = zoom,
                bnd = bnd,
                addr = addr,
                draw = if draw { "true" } else { "false" },
            );
            let mut ev = document::eval(&js);
            loop {
                match ev.recv::<String>().await {
                    Ok(coord) => on_point.call(coord),
                    Err(_) => break,
                }
            }
        });
    });

    rsx! {
        div { id: "leaflet-{map_id}", class: "w-full h-full rounded-lg" }
    }
}

// ── Form state types ──────────────────────────────────────────────────────────

#[derive(Clone, Default)]
struct TerritoryFormState {
    number: String,
    name: String,
    description: String,
    notes: String,
    boundary: Vec<Vec<f64>>,
    submitting: bool,
    error: Option<String>,
}

#[derive(Clone, Default)]
struct AddressFormState {
    description: String,
    lat: String,
    lng: String,
    submitting: bool,
    error: Option<String>,
}

#[derive(Clone, Default)]
struct AssignFormState {
    territory_id: String,
    user_id: String,
    assigned_date: String,
    submitting: bool,
    error: Option<String>,
}

// ── Main page ─────────────────────────────────────────────────────────────────

#[component]
pub fn AppTerritory() -> Element {
    let db_signal = use_db();
    let crypto_signal = use_crypto();
    let mut name_fmt: Signal<NameFormat> = use_signal(NameFormat::default);

    {
        let db_signal = db_signal.clone();
        let crypto_signal = crypto_signal.clone();
        use_effect(move || {
            let db_signal = db_signal.clone();
            let crypto_signal = crypto_signal.clone();
            spawn(async move {
                if let Some(db) = db_signal.read().db.clone() {
                    let crypto = crypto_signal.read().clone();
                    if let Ok(congregations) = Congregation::all(&db, &crypto).await {
                        if let Some(cong) = congregations.into_iter().next() {
                            *name_fmt.write() = cong.name_format.clone();
                        }
                    }
                }
            });
        });
    }

    let mut territories_res = use_resource(move || {
        let db_signal = db_signal.clone();
        async move {
            let db = db_signal.read().db.clone()?;
            Territory::all(&db).await.ok()
        }
    });

    let mut users_res = use_resource(move || {
        let db_signal = db_signal.clone();
        let crypto_signal = crypto_signal.clone();
        async move {
            let db = db_signal.read().db.clone()?;
            let crypto = crypto_signal.read().clone();
            User::all(&db, &crypto).await.ok()
        }
    });

    let mut active_assignments_res = use_resource(move || {
        let db_signal = db_signal.clone();
        async move {
            let db = db_signal.read().db.clone()?;
            TerritoryAssignment::active(&db).await.ok()
        }
    });

    let mut requests_res = use_resource(move || {
        let db_signal = db_signal.clone();
        async move {
            let db = db_signal.read().db.clone()?;
            let cutoff = days_ago_iso(30);
            TerritoryRequest::expire_and_get_pending(&db, &cutoff).await.ok()
        }
    });

    // Resolve current user id from stored session
    let mut current_user_id: Signal<Option<RecordId>> = use_signal(|| None);
    {
        let db_signal = db_signal.clone();
        let crypto_signal = crypto_signal.clone();
        use_effect(move || {
            let db_signal = db_signal.clone();
            let crypto_signal = crypto_signal.clone();
            spawn(async move {
                // Use the user-id stored in prefs if available, else first user
                let stored_uid = ls_get("theo_current_user_id").await;
                if let Some(db) = db_signal.read().db.clone() {
                    let crypto = crypto_signal.read().clone();
                    if let Ok(users) = User::all(&db, &crypto).await {
                        let found = if let Some(ref uid_str) = stored_uid {
                            users.iter().find(|u| {
                                u.id.as_ref().map(|id| rid_str(id) == *uid_str).unwrap_or(false)
                            }).and_then(|u| u.id.clone())
                        } else {
                            None
                        };
                        let id = found.or_else(|| users.into_iter().next().and_then(|u| u.id));
                        *current_user_id.write() = id;
                    }
                }
            });
        });
    }

    rsx! {
        div { class: "flex flex-col h-full gap-3",
            h1 { class: "text-2xl font-bold text-gray-900 shrink-0", {t!("page-territory")} }
            Tabs {
                default_value: "mine".to_string(),
                horizontal: true,
                class: "flex flex-col flex-1 min-h-0",
                TabList { class: "flex gap-1 p-1 bg-gray-100 rounded-xl shrink-0",
                    TabTrigger {
                        value: "mine".to_string(),
                        index: 0usize,
                        class: "flex-1 py-2 px-3 text-sm font-medium rounded-lg transition-colors text-gray-500 data-[state=active]:bg-white data-[state=active]:text-gray-900 data-[state=active]:shadow-sm",
                        {t!("terr-tab-mine")}
                    }
                    TabTrigger {
                        value: "list".to_string(),
                        index: 1usize,
                        class: "flex-1 py-2 px-3 text-sm font-medium rounded-lg transition-colors text-gray-500 data-[state=active]:bg-white data-[state=active]:text-gray-900 data-[state=active]:shadow-sm",
                        {t!("terr-tab-list")}
                    }
                    TabTrigger {
                        value: "assignments".to_string(),
                        index: 2usize,
                        class: "flex-1 py-2 px-3 text-sm font-medium rounded-lg transition-colors text-gray-500 data-[state=active]:bg-white data-[state=active]:text-gray-900 data-[state=active]:shadow-sm",
                        {t!("terr-tab-assignments")}
                    }
                }
                TabContent {
                    value: "mine".to_string(),
                    index: 0usize,
                    class: "flex-1 overflow-y-auto",
                    MyTerritoriesTab {
                        territories_res,
                        active_assignments_res,
                        requests_res,
                        current_user_id,
                        name_fmt,
                    }
                }
                TabContent {
                    value: "list".to_string(),
                    index: 1usize,
                    class: "flex-1 overflow-y-auto",
                    TerritoriesListTab {
                        territories_res,
                        active_assignments_res,
                        name_fmt,
                    }
                }
                TabContent {
                    value: "assignments".to_string(),
                    index: 2usize,
                    class: "flex-1 overflow-y-auto",
                    AssignmentsTab {
                        territories_res,
                        users_res,
                        requests_res,
                        name_fmt,
                    }
                }
            }
        }
    }
}

// ── My Territories Tab ────────────────────────────────────────────────────────

#[component]
fn MyTerritoriesTab(
    territories_res: Resource<Option<Vec<Territory>>>,
    active_assignments_res: Resource<Option<Vec<TerritoryAssignment>>>,
    requests_res: Resource<Option<Vec<TerritoryRequest>>>,
    current_user_id: Signal<Option<RecordId>>,
    name_fmt: Signal<NameFormat>,
) -> Element {
    let db_signal = use_db();
    let mut returning: Signal<Option<RecordId>> = use_signal(|| None);
    let mut request_open = use_signal(|| false);

    let territories: Vec<Territory> = territories_res
        .read().as_ref().and_then(|r| r.as_ref()).cloned().unwrap_or_default();

    let territory_map: std::collections::HashMap<String, Territory> = territories
        .iter()
        .filter_map(|t| t.id.as_ref().map(|id| (rid_str(id), t.clone())))
        .collect();

    let active_all: Vec<TerritoryAssignment> = active_assignments_res
        .read().as_ref().and_then(|r| r.as_ref()).cloned().unwrap_or_default();

    let my_assignments: Vec<&TerritoryAssignment> = {
        let cur = current_user_id.read().clone();
        match cur {
            Some(ref uid) => active_all.iter().filter(|a| rid_str(&a.user) == rid_str(uid)).collect(),
            None => vec![],
        }
    };

    let assigned_ids: std::collections::HashSet<String> = active_all
        .iter().map(|a| rid_str(&a.territory)).collect();

    let all_requests: Vec<TerritoryRequest> = requests_res
        .read().as_ref().and_then(|r| r.as_ref()).cloned().unwrap_or_default();
    let my_pending_requests: Vec<TerritoryRequest> = {
        let cur = current_user_id.read().clone();
        match cur {
            Some(ref uid) => all_requests.into_iter()
                .filter(|r| rid_str(&r.user) == rid_str(uid))
                .collect(),
            None => vec![],
        }
    };

    rsx! {
        div { class: "flex flex-col gap-4 pt-3",
            // Pending request banners for this user
            for req in my_pending_requests.iter() {
                {
                    let req = req.clone();
                    rsx! {
                        div { class: "bg-amber-50 border border-amber-200 rounded-xl p-4 flex items-start gap-3",
                            div { class: "flex-1 min-w-0",
                                p { class: "text-sm font-semibold text-amber-800", {t!("terr-request-pending")} }
                                p { class: "text-xs text-amber-600 mt-0.5", "{req.requested_date}" }
                                if let Some(ref n) = req.notes {
                                    p { class: "text-xs text-amber-700 italic mt-0.5 break-words", "{n}" }
                                }
                            }
                            span { class: "text-lg shrink-0", "⏳" }
                        }
                    }
                }
            }

            // Request Territory — full width, above the list
            button {
                class: "w-full px-4 py-3 bg-primary-600 text-white rounded-xl hover:bg-primary-700 text-sm font-semibold transition-colors",
                onclick: move |_| request_open.set(true),
                {t!("terr-request")}
            }

            if my_assignments.is_empty() {
                div { class: "bg-white rounded-xl border border-gray-200 px-6 py-12 text-center",
                    p { class: "text-4xl mb-3", "🗺️" }
                    p { class: "font-medium text-gray-600", {t!("terr-no-mine")} }
                }
            } else {
                div { class: "flex flex-col gap-3",
                    for a in my_assignments.iter() {
                        {
                            let a = (*a).clone();
                            let terr = territory_map.get(&rid_str(&a.territory)).cloned();
                            let a_id = a.id.clone();
                            let months = months_since(&a.assigned_date);
                            let overdue = months >= 4;
                            rsx! {
                                div { class: if overdue { "bg-red-50 border border-red-200 rounded-xl p-4 flex items-center justify-between gap-3" } else { "bg-white border border-gray-200 rounded-xl p-4 flex items-center justify-between gap-3" },
                                    div { class: "flex-1 min-w-0",
                                        div { class: "flex items-center gap-2",
                                            if overdue {
                                                span { class: "text-sm", "⚠️" }
                                            }
                                            p { class: "font-semibold text-gray-900 truncate",
                                                if let Some(ref t) = terr {
                                                    "#{t.number} – {t.name}"
                                                } else {
                                                    "—"
                                                }
                                            }
                                        }
                                        p { class: "text-xs text-gray-500 mt-0.5",
                                            "{t!(\"terr-assigned-date\")}: {a.assigned_date}"
                                            if overdue {
                                                span { class: "ml-2 text-red-600 font-medium", "({months} mo)" }
                                            }
                                        }
                                    }
                                    button {
                                        class: "shrink-0 px-3 py-1.5 text-sm bg-gray-100 hover:bg-gray-200 text-gray-700 rounded-lg transition-colors font-medium",
                                        onclick: move |_| returning.set(a_id.clone()),
                                        {t!("terr-return")}
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Return modal
            if let Some(ref aid) = returning.read().clone() {
                {
                    let aid = aid.clone();
                    rsx! {
                        TerritoryModal {
                            title: t!("terr-return").to_string(),
                            on_close: move |_| returning.set(None),
                            div { class: "flex flex-col gap-4",
                                p { class: "text-sm text-gray-600", "{t!(\"terr-returned-date\")}: {today_iso()}" }
                                div { class: "flex gap-2 justify-end",
                                    button {
                                        class: "px-4 py-2 text-sm rounded-lg bg-gray-100 hover:bg-gray-200 text-gray-700",
                                        onclick: move |_| returning.set(None),
                                        {t!("btn-cancel")}
                                    }
                                    button {
                                        class: "px-4 py-2 text-sm rounded-lg bg-primary-600 hover:bg-primary-700 text-white font-medium",
                                        onclick: {
                                            let db_signal = db_signal.clone();
                                            let aid = aid.clone();
                                            move |_| {
                                                let db_signal = db_signal.clone();
                                                let aid = aid.clone();
                                                let today = today_iso();
                                                returning.set(None);
                                                spawn(async move {
                                                    if let Some(db) = db_signal.read().db.clone() {
                                                        let _ = TerritoryAssignment::return_territory(&db, aid, today).await;
                                                    }
                                                    active_assignments_res.restart();
                                                });
                                            }
                                        },
                                        {t!("terr-return")}
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Request territory modal
            if *request_open.read() {
                RequestTerritoryModal {
                    current_user_id,
                    on_close: move |_| {
                        request_open.set(false);
                        requests_res.restart();
                    },
                }
            }
        }
    }
}

#[component]
fn RequestTerritoryModal(
    current_user_id: Signal<Option<RecordId>>,
    on_close: Callback<()>,
) -> Element {
    let db_signal = use_db();
    let mut notes = use_signal(String::new);
    let mut submitting = use_signal(|| false);
    let mut error: Signal<Option<String>> = use_signal(|| None);

    rsx! {
        TerritoryModal {
            title: t!("terr-request").to_string(),
            on_close: move |_| on_close.call(()),
            div { class: "flex flex-col gap-4",
                div {
                    label { class: "block text-xs font-medium text-gray-600 mb-1",
                        {t!("terr-request-notes")}
                    }
                    textarea {
                        class: "w-full px-3 py-2 rounded-lg border border-gray-200 text-sm focus:outline-none focus:ring-2 focus:ring-primary-300 resize-none",
                        rows: 3,
                        placeholder: t!("terr-request-notes-hint").to_string(),
                        value: "{notes}",
                        oninput: move |e| notes.set(e.value()),
                    }
                }
                if let Some(ref err) = error.read().clone() {
                    p { class: "text-sm text-red-600", "{err}" }
                }
                div { class: "flex gap-2 justify-end",
                    button {
                        class: "px-4 py-2 text-sm rounded-lg bg-gray-100 hover:bg-gray-200 text-gray-700",
                        onclick: move |_| on_close.call(()),
                        {t!("btn-cancel")}
                    }
                    button {
                        class: "px-4 py-2 text-sm rounded-lg bg-primary-600 hover:bg-primary-700 text-white font-medium disabled:opacity-50",
                        disabled: *submitting.read(),
                        onclick: {
                            let db_signal = db_signal.clone();
                            let on_close = on_close.clone();
                            move |_| {
                                let uid = current_user_id.read().clone();
                                let Some(uid) = uid else {
                                    error.set(Some("No user selected".into()));
                                    return;
                                };
                                let n = notes.read().trim().to_string();
                                submitting.set(true);
                                let db_signal = db_signal.clone();
                                let on_close = on_close.clone();
                                spawn(async move {
                                    if let Some(db) = db_signal.read().db.clone() {
                                        let _ = TerritoryRequest::create(
                                                &db,
                                                TerritoryRequestData {
                                                    user: uid,
                                                    notes: if n.is_empty() { None } else { Some(n) },
                                                    requested_date: today_iso(),
                                                    status: "pending".to_string(),
                                                },
                                            )
                                            .await;
                                    }
                                    submitting.set(false);
                                    on_close.call(());
                                });
                            }
                        },
                        {t!("terr-request-submit")}
                    }
                }
            }
        }
    }
}

// ── Territories List Tab ──────────────────────────────────────────────────────

#[component]
fn TerritoriesListTab(
    territories_res: Resource<Option<Vec<Territory>>>,
    active_assignments_res: Resource<Option<Vec<TerritoryAssignment>>>,
    name_fmt: Signal<NameFormat>,
) -> Element {
    let db_signal = use_db();
    let mut search = use_signal(String::new);
    let mut selected: Signal<Option<Territory>> = use_signal(|| None);
    let mut edit_mode = use_signal(|| false);
    let mut draw_mode = use_signal(|| false);
    let mut add_point_mode = use_signal(|| false);
    let mut form = use_signal(TerritoryFormState::default);
    let mut addr_form = use_signal(AddressFormState::default);
    let mut add_addr_open = use_signal(|| false);
    let mut confirm_delete: Signal<Option<RecordId>> = use_signal(|| None);
    let mut pending_addr_coords: Signal<Option<(f64, f64)>> = use_signal(|| None);

    let mut addresses_res = use_resource(move || {
        let db_signal = db_signal.clone();
        let sel = selected.read().clone();
        async move {
            let terr = sel?;
            let id = terr.id?;
            let db = db_signal.read().db.clone()?;
            TerritoryAddress::for_territory(&db, &id).await.ok()
        }
    });

    let territories: Vec<Territory> = territories_res
        .read().as_ref().and_then(|r| r.as_ref()).cloned().unwrap_or_default();

    let active_assigned: std::collections::HashSet<String> = active_assignments_res
        .read().as_ref().and_then(|r| r.as_ref())
        .map(|list| list.iter().map(|a| rid_str(&a.territory)).collect())
        .unwrap_or_default();

    let addresses: Vec<TerritoryAddress> = addresses_res
        .read().as_ref().and_then(|r| r.as_ref()).cloned().unwrap_or_default();

    let filtered: Vec<Territory> = territories.iter()
        .filter(|t| {
            let q = search.read().to_lowercase();
            q.is_empty() || t.name.to_lowercase().contains(&q) || t.number.contains(&q)
        })
        .cloned()
        .collect();

    // When map clicked in add-point mode, pre-fill address form
    let pending_clone = pending_addr_coords.read().clone();
    if let Some((lat, lng)) = pending_clone {
        addr_form.write().lat = format!("{:.6}", lat);
        addr_form.write().lng = format!("{:.6}", lng);
        add_addr_open.set(true);
        *pending_addr_coords.write() = None;
    }

    rsx! {
        div { class: "flex flex-col gap-4 pt-3",
            div { class: "flex gap-2",
                input {
                    class: "flex-1 px-3 py-2 rounded-lg border border-gray-200 bg-white text-sm focus:outline-none focus:ring-2 focus:ring-primary-300",
                    r#type: "text",
                    placeholder: t!("terr-search").to_string(),
                    value: "{search}",
                    oninput: move |e| search.set(e.value()),
                }
                button {
                    class: "px-4 py-2 bg-primary-600 text-white rounded-lg hover:bg-primary-700 text-sm font-medium transition-colors shrink-0",
                    onclick: move |_| {
                        *form.write() = TerritoryFormState::default();
                        selected.set(None);
                        edit_mode.set(true);
                        draw_mode.set(false);
                    },
                    {t!("btn-add-territory")}
                }
            }

            if territories.is_empty() && !*edit_mode.read() {
                div { class: "bg-white rounded-xl border border-gray-200 px-6 py-12 text-center",
                    p { class: "text-4xl mb-3", "🗺️" }
                    p { class: "font-medium text-gray-600", {t!("empty-territory-title")} }
                    p { class: "text-sm mt-1 text-gray-400", {t!("empty-territory-desc")} }
                }
            }

            // Territory list
            if !territories.is_empty() {
                div { class: "flex flex-col gap-2",
                    for terr in filtered.iter() {
                        {
                            let t = terr.clone();
                            let is_assigned = t
                                .id
                                .as_ref()
                                .map(|id| active_assigned.contains(&rid_str(id)))
                                .unwrap_or(false);
                            let is_selected = selected
                                .read()
                                .as_ref()
                                .and_then(|s| s.id.as_ref())
                                .zip(t.id.as_ref())
                                .map(|(a, b)| rid_str(a) == rid_str(b))
                                .unwrap_or(false);
                            rsx! {
                                div {
                                    class: if is_selected { "bg-primary-50 border border-primary-200 rounded-xl p-4 flex items-center gap-3 cursor-pointer" } else { "bg-white border border-gray-200 rounded-xl p-4 flex items-center gap-3 hover:border-gray-300 cursor-pointer" },
                                    onclick: {
                                        let t = t.clone();
                                        move |_| {
                                            selected.set(Some(t.clone()));
                                            edit_mode.set(false);
                                            draw_mode.set(false);
                                            add_point_mode.set(false);
                                        }
                                    },
                                    div { class: "flex items-center justify-center w-10 h-10 rounded-lg bg-primary-100 text-primary-700 font-bold text-sm shrink-0",
                                        "#{t.number}"
                                    }
                                    div { class: "flex-1 min-w-0",
                                        p { class: "font-semibold text-gray-900 truncate", "{t.name}" }
                                        p { class: "text-xs text-gray-500 mt-0.5",
                                            if t.boundary.is_empty() {
                                                {t!("terr-no-boundary")}
                                            } else {
                                                "{t.boundary.len()} pts"
                                            }
                                            if is_assigned {
                                                span { class: "ml-2 text-amber-600 font-medium", "● Assigned" }
                                            }
                                        }
                                    }
                                    span { class: "text-gray-300 text-lg shrink-0", "›" }
                                }
                            }
                        }
                    }
                }
            }

            // Detail view
            if let Some(ref terr) = selected.read().clone() {
                if !*edit_mode.read() {
                    {
                        let terr = terr.clone();
                        let (clat, clng) = map_center(&terr.boundary);
                        let zoom: u8 = if terr.boundary.is_empty() { 10 } else { 15 };
                        let bnd_json = boundary_to_json(&terr.boundary);
                        let addr_json = addresses_to_json(&addresses);
                        let map_id = terr
                            .id
                            .as_ref()
                            .map(|id| rid_str(id).replace(':', "_"))
                            .unwrap_or_else(|| "view".to_string());
                        let terr_id_del = terr.id.clone();
                        let terr_edit = terr.clone();
                        rsx! {
                            div { class: "bg-white border border-gray-200 rounded-xl overflow-hidden",
                                div { class: "p-4 border-b border-gray-100 flex items-center justify-between gap-2",
                                    div {
                                        p { class: "font-bold text-gray-900", "#{terr.number} – {terr.name}" }
                                        if let Some(ref d) = terr.description {
                                            p { class: "text-sm text-gray-500 mt-0.5", "{d}" }
                                        }
                                    }
                                    div { class: "flex gap-2",
                                        button {
                                            class: "px-3 py-1.5 text-sm bg-gray-100 hover:bg-gray-200 rounded-lg text-gray-700",
                                            onclick: move |_| {
                                                *form.write() = TerritoryFormState {
                                                    number: terr_edit.number.clone(),
                                                    name: terr_edit.name.clone(),
                                                    description: terr_edit.description.clone().unwrap_or_default(), // Map  Map // Map  Map
                                                    notes: terr_edit.notes.clone().unwrap_or_default(),
                                                    boundary: terr_edit.boundary.clone(),
                                                    ..Default::default()
                                                };
                                                edit_mode.set(true);
                                                draw_mode.set(false);
                                            },
                                            {t!("terr-edit")}
                                        }
                                        if let Some(ref tid) = terr_id_del {
                                            {
                                                let tid = tid.clone(); // Map // Map
                                                rsx! {
                                                    button {
                                                        class: "px-3 py-1.5 text-sm bg-red-50 hover:bg-red-100 rounded-lg text-red-600",
                                                        onclick: move |_| confirm_delete.set(Some(tid.clone())),
                                                        {t!("terr-delete")}
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                // Map
                                div { class: "h-64 sm:h-96 w-full",
                                    LeafletMap {
                                        map_id: "{map_id}_v",
                                        center_lat: clat,
                                        center_lng: clng,
                                        zoom,
                                        boundary_json: bnd_json,
                                        addresses_json: addr_json,
                                        draw_mode: ReadOnlySignal::new(add_point_mode),
                                        on_point: move |coord: String| {
                                            if *add_point_mode.read() {
                                                let parts: Vec<&str> = coord.splitn(2, ',').collect();
                                                if parts.len() == 2 {
                                                    let lat = parts[0].parse().unwrap_or(0.0);
                                                    let lng = parts[1].parse().unwrap_or(0.0);
                                                    *pending_addr_coords.write() = Some((lat, lng));
                                                    add_point_mode.set(false);
                                                }
                                            }
                                        },
                                    }
                                }
                                // Addresses
                                div { class: "p-4",
                                    div { class: "flex items-center justify-between mb-3",
                                        p { class: "font-semibold text-gray-800 text-sm", {t!("terr-addresses")} }
                                        button {
                                            class: if *add_point_mode.read() { "px-3 py-1 text-xs rounded-lg bg-primary-100 text-primary-700 border border-primary-300 font-medium" } else { "px-3 py-1 text-xs rounded-lg bg-gray-100 text-gray-600 hover:bg-gray-200 transition-colors" },
                                            onclick: move |_| {
                                                if !*add_point_mode.read() {
                                                    add_point_mode.set(true);
                                                } else {
                                                    *addr_form.write() = AddressFormState::default();
                                                    add_addr_open.set(true);
                                                    add_point_mode.set(false);
                                                }
                                            },
                                            if *add_point_mode.read() {
                                                "📍 Click map…"
                                            } else {
                                                {t!("terr-add-address")}
                                            }
                                        }
                                    }
                                    if addresses.is_empty() {
                                        p { class: "text-sm text-gray-400 text-center py-3", {t!("terr-map-click-hint")} }
                                    } else {
                                        div { class: "flex flex-col gap-0.5",
                                            for addr in addresses.iter() {
                                                {
                                                    let addr_id = addr.id.clone();
                                                    let desc = addr.description.clone();
                                                    rsx! {
                                                        div { class: "flex items-center gap-2 px-3 py-2 rounded-lg hover:bg-gray-50 group",
                                                            span { class: "text-primary-600 shrink-0", "📍" }
                                                            span { class: "flex-1 text-sm text-gray-700 truncate", "{desc}" }
                                                            span { class: "text-xs text-gray-400 shrink-0", "{addr.lat:.4}, {addr.lng:.4}" }
                                                            button {
                                                                class: "shrink-0 opacity-0 group-hover:opacity-100 text-gray-400 hover:text-red-500 transition-all text-sm",
                                                                onclick: {
                                                                    let db_signal = db_signal.clone();
                                                                    move |_| {
                                                                        if let Some(ref id) = addr_id {
                                                                            let id = id.clone();
                                                                            let db_signal = db_signal.clone();
                                                                            spawn(async move {
                                                                                if let Some(db) = db_signal.read().db.clone() {
                                                                                    let _ = TerritoryAddress::delete(&db, id).await;
                                                                                }
                                                                                addresses_res.restart();
                                                                            });
                                                                        }
                                                                    }
                                                                },
                                                                "✕"
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Edit / create form
            if *edit_mode.read() {
                {
                    let existing = selected.read().clone();
                    let bnd_display = form.read().boundary.clone();
                    let bnd_show = if bnd_display.is_empty() {
                        existing.as_ref().map(|t| t.boundary.clone()).unwrap_or_default()
                    } else {
                        bnd_display
                    };
                    let (clat, clng) = map_center(&bnd_show);
                    let zoom: u8 = if bnd_show.is_empty() { 10 } else { 15 };
                    let bnd_json = boundary_to_json(&bnd_show);
                    let map_id = existing
                        .as_ref()
                        .and_then(|t| t.id.as_ref())
                        .map(|id| rid_str(id).replace(':', "_"))
                        .unwrap_or_else(|| "new".to_string());
                    let is_new = existing.is_none();
                    rsx! {
                        div { class: "bg-white border border-primary-200 rounded-xl overflow-hidden",
                            div { class: "p-4 border-b border-gray-100",
                                p { class: "font-bold text-gray-900",
                                    if is_new {
                                        {t!("btn-add-territory")}
                                    } else {
                                        {t!("terr-edit")}
                                    }
                                }
                            }
                            div { class: "p-4 flex flex-col gap-3",
                                div { class: "grid grid-cols-2 gap-3",
                                    div {
                                        label { class: "block text-xs font-medium text-gray-600 mb-1", {t!("terr-number")} }
                                        input {
                                            class: "w-full px-3 py-2 rounded-lg border border-gray-200 text-sm focus:outline-none focus:ring-2 focus:ring-primary-300",
                                            r#type: "text",
                                            value: "{form.read().number}",
                                            oninput: move |e| form.write().number = e.value(),
                                        }
                                    }
                                    div {
                                        label { class: "block text-xs font-medium text-gray-600 mb-1", {t!("terr-name")} }
                                        input {
                                            class: "w-full px-3 py-2 rounded-lg border border-gray-200 text-sm focus:outline-none focus:ring-2 focus:ring-primary-300",
                                            r#type: "text",
                                            value: "{form.read().name}",
                                            oninput: move |e| form.write().name = e.value(),
                                        }
                                    }
                                }
                                div {
                                    label { class: "block text-xs font-medium text-gray-600 mb-1", {t!("terr-description")} }
                                    input {
                                        class: "w-full px-3 py-2 rounded-lg border border-gray-200 text-sm focus:outline-none focus:ring-2 focus:ring-primary-300",
                                        r#type: "text",
                                        value: "{form.read().description}",
                                        oninput: move |e| form.write().description = e.value(),
                                    }
                                }
                                div {
                                    label { class: "block text-xs font-medium text-gray-600 mb-1", {t!("terr-notes")} }
                                    textarea {
                                        class: "w-full px-3 py-2 rounded-lg border border-gray-200 text-sm focus:outline-none focus:ring-2 focus:ring-primary-300 resize-none",
                                        rows: 2,
                                        value: "{form.read().notes}",
                                        oninput: move |e| form.write().notes = e.value(),
                                    }
                                }
                                // Map + boundary drawing
                                div {
                                    div { class: "flex items-center justify-between mb-2",
                                        label { class: "text-xs font-medium text-gray-600", {t!("terr-boundary")} }
                                        div { class: "flex gap-2",
                                            button {
                                                class: if *draw_mode.read() { "px-3 py-1 text-xs rounded-lg bg-primary-100 text-primary-700 border border-primary-300 font-medium" } else { "px-3 py-1 text-xs rounded-lg bg-gray-100 text-gray-600 hover:bg-gray-200 transition-colors" },
                                                onclick: move |_| draw_mode.set(!draw_mode()),
                                                if *draw_mode.read() {
                                                    "✏️ Drawing…"
                                                } else {
                                                    "✏️ Draw"
                                                }
                                            }
                                            button {
                                                class: "px-3 py-1 text-xs rounded-lg bg-gray-100 text-gray-600 hover:bg-red-100 hover:text-red-600 transition-colors",
                                                onclick: move |_| form.write().boundary.clear(),
                                                {t!("terr-clear-boundary")}
                                            }
                                        }
                                    }
                                    if *draw_mode.read() {
                                        p { class: "text-xs text-primary-600 mb-2", {t!("terr-draw-hint")} }
                                    }
                                    div { class: "h-56 sm:h-80 border border-gray-200 rounded-lg overflow-hidden",
                                        LeafletMap {
                                            map_id: "{map_id}_e",
                                            center_lat: clat,
                                            center_lng: clng,
                                            zoom,
                                            boundary_json: bnd_json,
                                            addresses_json: "[]".to_string(),
                                            draw_mode: ReadOnlySignal::new(draw_mode),
                                            on_point: move |coord: String| {
                                                if *draw_mode.read() {
                                                    let parts: Vec<&str> = coord.splitn(2, ',').collect();
                                                    if parts.len() == 2 {
                                                        form.write()
                                                            .boundary
                                                            .push(
                                                                vec![
                                                                    parts[0].parse().unwrap_or(0.0),
                                                                    parts[1].parse().unwrap_or(0.0),
                                                                ],
                                                            );
                                                    }
                                                }
                                            },
                                        }
                                    }
                                    p { class: "text-xs text-gray-400 mt-1", "{form.read().boundary.len()} points" }
                                }
                                if let Some(ref err) = form.read().error.clone() {
                                    p { class: "text-sm text-red-600", "{err}" }
                                }
                                div { class: "flex gap-2 justify-end",
                                    button {
                                        class: "px-4 py-2 text-sm rounded-lg bg-gray-100 hover:bg-gray-200 text-gray-700",
                                        onclick: move |_| {
                                            edit_mode.set(false);
                                            draw_mode.set(false);
                                        },
                                        {t!("btn-cancel")}
                                    }
                                    button {
                                        class: "px-4 py-2 text-sm rounded-lg bg-primary-600 hover:bg-primary-700 text-white font-medium disabled:opacity-50",
                                        disabled: form.read().submitting,
                                        onclick: {
                                            let db_signal = db_signal.clone();
                                            let existing_id = existing.as_ref().and_then(|t| t.id.clone());
                                            move |_| {
                                                let f = form.read().clone();
                                                if f.name.trim().is_empty() || f.number.trim().is_empty() {
                                                    form.write().error = Some("Number and name required".into());
                                                    return;
                                                }
                                                form.write().submitting = true;
                                                let db_signal = db_signal.clone();
                                                let eid = existing_id.clone();
                                                spawn(async move {
                                                    if let Some(db) = db_signal.read().db.clone() {
                                                        let data = TerritoryData {
                                                            number: f.number.trim().to_string(),
                                                            name: f.name.trim().to_string(),
                                                            description: if f.description.trim().is_empty() {
                                                                None
                                                            } else {
                                                                Some(f.description.trim().to_string())
                                                            },
                                                            boundary: f.boundary.clone(),
                                                            notes: if f.notes.trim().is_empty() {
                                                                None
                                                            } else {
                                                                Some(f.notes.trim().to_string())
                                                            },
                                                        };
                                                        if let Some(id) = eid {
                                                            let _ = Territory::update(&db, id, data).await;
                                                        } else {
                                                            let _ = Territory::create(&db, data).await;
                                                        }
                                                    }
                                                    edit_mode.set(false);
                                                    draw_mode.set(false);
                                                    form.write().submitting = false;
                                                    territories_res.restart();
                                                });
                                            }
                                        },
                                        {t!("terr-save")}
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Add address modal
            if *add_addr_open.read() {
                if let Some(ref terr) = selected.read().clone() {
                    {
                        let terr_id = terr.id.clone();
                        rsx! {
                            TerritoryModal {
                                title: t!("terr-add-address").to_string(),
                                on_close: move |_| {
                                    add_addr_open.set(false);
                                    *addr_form.write() = AddressFormState::default();
                                },
                                div { class: "flex flex-col gap-3",
                                    div {
                                        label { class: "block text-xs font-medium text-gray-600 mb-1", {t!("terr-address-desc")} }
                                        input {
                                            class: "w-full px-3 py-2 rounded-lg border border-gray-200 text-sm focus:outline-none focus:ring-2 focus:ring-primary-300",
                                            r#type: "text",
                                            placeholder: "e.g. 12 Main St",
                                            value: "{addr_form.read().description}",
                                            oninput: move |e| addr_form.write().description = e.value(),
                                        }
                                    }
                                    div { class: "grid grid-cols-2 gap-3",
                                        div {
                                            label { class: "block text-xs font-medium text-gray-600 mb-1", {t!("terr-address-lat")} }
                                            input {
                                                class: "w-full px-3 py-2 rounded-lg border border-gray-200 text-sm focus:outline-none focus:ring-2 focus:ring-primary-300",
                                                r#type: "number",
                                                step: "0.000001",
                                                value: "{addr_form.read().lat}",
                                                oninput: move |e| addr_form.write().lat = e.value(),
                                            }
                                        }
                                        div {
                                            label { class: "block text-xs font-medium text-gray-600 mb-1", {t!("terr-address-lng")} }
                                            input {
                                                class: "w-full px-3 py-2 rounded-lg border border-gray-200 text-sm focus:outline-none focus:ring-2 focus:ring-primary-300",
                                                r#type: "number",
                                                step: "0.000001",
                                                value: "{addr_form.read().lng}",
                                                oninput: move |e| addr_form.write().lng = e.value(),
                                            }
                                        }
                                    }
                                    if let Some(ref err) = addr_form.read().error.clone() {
                                        p { class: "text-sm text-red-600", "{err}" }
                                    }
                                    div { class: "flex gap-2 justify-end",
                                        button {
                                            class: "px-4 py-2 text-sm rounded-lg bg-gray-100 hover:bg-gray-200 text-gray-700",
                                            onclick: move |_| {
                                                add_addr_open.set(false);
                                                *addr_form.write() = AddressFormState::default();
                                            },
                                            {t!("btn-cancel")}
                                        }
                                        button {
                                            class: "px-4 py-2 text-sm rounded-lg bg-primary-600 hover:bg-primary-700 text-white font-medium disabled:opacity-50",
                                            disabled: addr_form.read().submitting,
                                            onclick: {
                                                let db_signal = db_signal.clone();
                                                move |_| {
                                                    let f = addr_form.read().clone();
                                                    let lat: f64 = f.lat.parse().unwrap_or(0.0);
                                                    let lng: f64 = f.lng.parse().unwrap_or(0.0);
                                                    let desc = f.description.trim().to_string();
                                                    if desc.is_empty() {
                                                        addr_form.write().error = Some("Description required".into());
                                                        return;
                                                    }
                                                    let tid = terr_id.clone();
                                                    let db_signal = db_signal.clone();
                                                    addr_form.write().submitting = true;
                                                    spawn(async move {
                                                        if let (Some(db), Some(tid)) = (db_signal.read().db.clone(), tid) {
                                                            let _ = TerritoryAddress::create(
                                                                    &db,
                                                                    TerritoryAddressData {
                                                                        territory: tid,
                                                                        lat,
                                                                        lng,
                                                                        description: desc,
                                                                        notes: None,
                                                                    },
                                                                )
                                                                .await;
                                                        }
                                                        add_addr_open.set(false);
                                                        addr_form.write().submitting = false;
                                                        addresses_res.restart();
                                                    });
                                                }
                                            },
                                            {t!("terr-add-address")}
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Confirm delete territory
            if let Some(ref tid) = confirm_delete.read().clone() {
                {
                    let tid = tid.clone();
                    rsx! {
                        TerritoryModal {
                            title: t!("terr-delete").to_string(),
                            on_close: move |_| confirm_delete.set(None),
                            div { class: "flex flex-col gap-4",
                                p { class: "text-sm text-gray-600", {t!("terr-confirm-delete")} }
                                div { class: "flex gap-2 justify-end",
                                    button {
                                        class: "px-4 py-2 text-sm rounded-lg bg-gray-100 hover:bg-gray-200 text-gray-700",
                                        onclick: move |_| confirm_delete.set(None),
                                        {t!("btn-cancel")}
                                    }
                                    button {
                                        class: "px-4 py-2 text-sm rounded-lg bg-red-600 hover:bg-red-700 text-white font-medium",
                                        onclick: {
                                            let db_signal = db_signal.clone();
                                            let tid = tid.clone();
                                            move |_| {
                                                let db_signal = db_signal.clone();
                                                let tid = tid.clone();
                                                confirm_delete.set(None);
                                                selected.set(None);
                                                spawn(async move {
                                                    if let Some(db) = db_signal.read().db.clone() {
                                                        let _ = Territory::delete(&db, tid).await;
                                                    }
                                                    territories_res.restart();
                                                });
                                            }
                                        },
                                        {t!("terr-delete")}
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

// ── Assignments Tab ───────────────────────────────────────────────────────────

#[component]
fn AssignmentsTab(
    territories_res: Resource<Option<Vec<Territory>>>,
    users_res: Resource<Option<Vec<User>>>,
    requests_res: Resource<Option<Vec<TerritoryRequest>>>,
    name_fmt: Signal<NameFormat>,
) -> Element {
    let db_signal = use_db();
    let mut sel_year = use_signal(current_year);
    let mut assign_open = use_signal(|| false);
    let mut assign_form = use_signal(AssignFormState::default);

    let mut assignments_res = use_resource(move || {
        let db_signal = db_signal.clone();
        let year = sel_year();
        async move {
            let db = db_signal.read().db.clone()?;
            TerritoryAssignment::all_for_year(&db, year).await.ok()
        }
    });

    let mut fulfilling: Signal<Option<TerritoryRequest>> = use_signal(|| None);
    let mut fulfill_tid = use_signal(String::new);

    let territories: Vec<Territory> = territories_res
        .read().as_ref().and_then(|r| r.as_ref()).cloned().unwrap_or_default();
    let users: Vec<User> = users_res
        .read().as_ref().and_then(|r| r.as_ref()).cloned().unwrap_or_default();

    let territory_map: std::collections::HashMap<String, Territory> = territories.iter()
        .filter_map(|t| t.id.as_ref().map(|id| (rid_str(id), t.clone())))
        .collect();
    let user_map: std::collections::HashMap<String, String> = users.iter()
        .filter_map(|u| {
            let fmt = name_fmt.read().clone();
            u.id.as_ref().map(|id| (rid_str(id), format_name(&u.first_name, &u.last_name, &fmt)))
        })
        .collect();

    let pending_requests: Vec<TerritoryRequest> = requests_res
        .read().as_ref().and_then(|r| r.as_ref()).cloned().unwrap_or_default();

    let assignments: Vec<TerritoryAssignment> = assignments_res
        .read().as_ref().and_then(|r| r.as_ref()).cloned().unwrap_or_default();

    let (overdue, rest): (Vec<_>, Vec<_>) = assignments.iter()
        .partition(|a| a.returned_date.is_none() && months_since(&a.assigned_date) >= 4);

    let assignable_users: Vec<&User> = users.iter().filter(|u| is_assignable(u)).collect();

    rsx! {
        div { class: "flex flex-col gap-4 pt-3",
            // Pending territory requests from publishers
            if !pending_requests.is_empty() {
                div { class: "bg-amber-50 border border-amber-200 rounded-xl p-4 flex flex-col gap-2",
                    p { class: "text-sm font-bold text-amber-800",
                        "📥 {t!(\"terr-requests-title\")}"
                    }
                    for req in pending_requests.iter() {
                        {
                            let req_a = req.clone();
                            let req_b = req.clone();
                            let req_id = req.id.clone();
                            let uname = user_map
                                .get(&rid_str(&req.user))
                                .cloned()
                                .unwrap_or_else(|| "—".to_string());
                            rsx! {
                                div { class: "flex items-start gap-3 pt-2 border-t border-amber-100",
                                    div { class: "flex-1 min-w-0",
                                        p { class: "text-sm font-semibold text-amber-900", "{uname}" }
                                        p { class: "text-xs text-amber-600", "{req_a.requested_date}" }
                                        if let Some(ref n) = req_a.notes {
                                            p { class: "text-xs text-amber-700 italic mt-0.5 break-words", "{n}" }
                                        }
                                    }
                                    div { class: "flex gap-2 shrink-0",
                                        button {
                                            class: "px-3 py-1 text-xs bg-primary-600 text-white rounded-lg hover:bg-primary-700 transition-colors font-medium",
                                            onclick: move |_| {
                                                fulfill_tid.set(String::new());
                                                fulfilling.set(Some(req_b.clone()));
                                            },
                                            {t!("terr-request-assign")}
                                        }
                                        button {
                                            class: "px-2 py-1 text-xs text-gray-400 hover:text-red-500 rounded transition-colors",
                                            onclick: {
                                                let db_signal = db_signal.clone();
                                                move |_| {
                                                    if let Some(ref id) = req_id {
                                                        let id = id.clone();
                                                        let db_signal = db_signal.clone();
                                                        spawn(async move {
                                                            if let Some(db) = db_signal.read().db.clone() {
                                                                let _ = TerritoryRequest::delete(&db, id).await;
                                                            }
                                                            requests_res.restart();
                                                        });
                                                    }
                                                }
                                            },
                                            "✕"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Year navigation + new button
            div { class: "flex items-center gap-3",
                button {
                    class: "px-4 py-2 rounded-lg border border-gray-200 text-gray-600 hover:bg-gray-50 font-semibold select-none",
                    onclick: move |_| sel_year.set(sel_year() - 1),
                    "‹"
                }
                span { class: "flex-1 text-center font-semibold text-gray-900", "{sel_year()}" }
                button {
                    class: "px-4 py-2 rounded-lg border border-gray-200 text-gray-600 hover:bg-gray-50 font-semibold select-none",
                    onclick: move |_| sel_year.set(sel_year() + 1),
                    "›"
                }
                button {
                    class: "ml-auto px-4 py-2 bg-primary-600 text-white rounded-lg hover:bg-primary-700 text-sm font-medium transition-colors",
                    onclick: move |_| {
                        *assign_form.write() = AssignFormState {
                            assigned_date: today_iso(),
                            ..Default::default()
                        };
                        assign_open.set(true);
                    },
                    {t!("terr-assign-new")}
                }
            }

            // Overdue banner
            if !overdue.is_empty() {
                div { class: "bg-red-50 border border-red-200 rounded-xl p-4 flex flex-col gap-2",
                    p { class: "text-sm font-bold text-red-700",
                        "⚠️ {t!(\"terr-overdue-title\")}"
                    }
                    for a in overdue.iter() {
                        {
                            let terr_name = territory_map
                                .get(&rid_str(&a.territory))
                                .map(|t| format!("#{} – {}", t.number, t.name))
                                .unwrap_or_else(|| "—".to_string());
                            let uname = user_map
                                .get(&rid_str(&a.user))
                                .cloned()
                                .unwrap_or_else(|| "—".to_string());
                            let mo = months_since(&a.assigned_date);
                            let a_id = a.id.clone();
                            rsx! {
                                div { class: "flex items-center justify-between gap-2 py-1 border-t border-red-100",
                                    div { class: "flex-1 min-w-0",
                                        p { class: "text-sm font-semibold text-red-800 truncate", "{terr_name}" }
                                        p { class: "text-xs text-red-600", "{uname} · {a.assigned_date} ({mo} mo)" }
                                    }
                                    button {
                                        class: "shrink-0 px-2 py-1 text-xs bg-white border border-red-200 rounded-lg text-red-600 hover:bg-red-100 transition-colors",
                                        onclick: {
                                            let db_signal = db_signal.clone();
                                            move |_| {
                                                let db_signal = db_signal.clone();
                                                let aid = a_id.clone();
                                                let today = today_iso();
                                                spawn(async move {
                                                    if let (Some(db), Some(aid)) = (db_signal.read().db.clone(), aid) {
                                                        let _ = TerritoryAssignment::return_territory(&db, aid, today).await;
                                                    }
                                                    assignments_res.restart();
                                                });
                                            }
                                        },
                                        {t!("terr-return")}
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if assignments.is_empty() {
                div { class: "bg-white rounded-xl border border-gray-200 px-6 py-12 text-center",
                    p { class: "text-4xl mb-3", "📋" }
                    p { class: "font-medium text-gray-600", {t!("terr-no-assignments")} }
                }
            } else {
                div { class: "flex flex-col gap-2",
                    for a in rest.iter() {
                        {
                            let terr_name = territory_map
                                .get(&rid_str(&a.territory))
                                .map(|t| format!("#{} – {}", t.number, t.name))
                                .unwrap_or_else(|| "—".to_string());
                            let uname = user_map
                                .get(&rid_str(&a.user))
                                .cloned()
                                .unwrap_or_else(|| "—".to_string());
                            let still_out = a.returned_date.is_none();
                            rsx! {
                                div { class: "bg-white border border-gray-200 rounded-xl p-3 flex items-center gap-3",
                                    div { class: "flex-1 min-w-0",
                                        p { class: "text-sm font-semibold text-gray-900 truncate", "{terr_name}" }
                                        p { class: "text-xs text-gray-500 mt-0.5",
                                            "{uname} · {a.assigned_date}"
                                            if let Some(ref rd) = a.returned_date {
                                                " → {rd}"
                                            }
                                        }
                                    }
                                    if still_out {
                                        span { class: "shrink-0 text-xs px-2 py-0.5 bg-amber-100 text-amber-700 rounded-full font-medium",
                                            {t!("terr-still-out")}
                                        }
                                    } else {
                                        span { class: "shrink-0 text-xs px-2 py-0.5 bg-green-100 text-green-700 rounded-full font-medium",
                                            {t!("terr-returned")}
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // New assignment modal
            if *assign_open.read() {
                TerritoryModal {
                    title: t!("terr-assign-new").to_string(),
                    on_close: move |_| assign_open.set(false),
                    div { class: "flex flex-col gap-3",
                        div {
                            label { class: "block text-xs font-medium text-gray-600 mb-1",
                                {t!("terr-territory")}
                            }
                            select {
                                class: "w-full px-3 py-2 rounded-lg border border-gray-200 text-sm focus:outline-none focus:ring-2 focus:ring-primary-300 bg-white",
                                value: "{assign_form.read().territory_id}",
                                onchange: move |e| assign_form.write().territory_id = e.value(),
                                option { value: "", "— Select territory —" }
                                for t in territories.iter() {
                                    {
                                        let tid = t.id.as_ref().map(rid_str).unwrap_or_default();
                                        rsx! {
                                            option { value: "{tid}", "#{t.number} – {t.name}" }
                                        }
                                    }
                                }
                            }
                        }
                        div {
                            label { class: "block text-xs font-medium text-gray-600 mb-1",
                                {t!("terr-assigned-to")}
                            }
                            select {
                                class: "w-full px-3 py-2 rounded-lg border border-gray-200 text-sm focus:outline-none focus:ring-2 focus:ring-primary-300 bg-white",
                                value: "{assign_form.read().user_id}",
                                onchange: move |e| assign_form.write().user_id = e.value(),
                                option { value: "", "— Select person —" }
                                for u in assignable_users.iter() {
                                    {
                                        let uid = u.id.as_ref().map(rid_str).unwrap_or_default();
                                        let fmt = name_fmt.read().clone();
                                        let uname = format_name(&u.first_name, &u.last_name, &fmt);
                                        rsx! {
                                            option { value: "{uid}", "{uname}" }
                                        }
                                    }
                                }
                            }
                        }
                        div {
                            label { class: "block text-xs font-medium text-gray-600 mb-1",
                                {t!("terr-assigned-date")}
                            }
                            input {
                                class: "w-full px-3 py-2 rounded-lg border border-gray-200 text-sm focus:outline-none focus:ring-2 focus:ring-primary-300",
                                r#type: "date",
                                value: "{assign_form.read().assigned_date}",
                                oninput: move |e| assign_form.write().assigned_date = e.value(),
                            }
                        }
                        if let Some(ref err) = assign_form.read().error.clone() {
                            p { class: "text-sm text-red-600", "{err}" }
                        }
                        div { class: "flex gap-2 justify-end",
                            button {
                                class: "px-4 py-2 text-sm rounded-lg bg-gray-100 hover:bg-gray-200 text-gray-700",
                                onclick: move |_| assign_open.set(false),
                                {t!("btn-cancel")}
                            }
                            button {
                                class: "px-4 py-2 text-sm rounded-lg bg-primary-600 hover:bg-primary-700 text-white font-medium disabled:opacity-50",
                                disabled: assign_form.read().submitting,
                                onclick: {
                                    let db_signal = db_signal.clone();
                                    move |_| {
                                        let f = assign_form.read().clone();
                                        let tid = parse_rid(&f.territory_id);
                                        let uid = parse_rid(&f.user_id);
                                        if tid.is_none() {
                                            assign_form.write().error = Some("Select a territory".into());
                                            return;
                                        }
                                        if uid.is_none() {
                                            assign_form.write().error = Some("Select a person".into());
                                            return;
                                        }
                                        if f.assigned_date.is_empty() {
                                            assign_form.write().error = Some("Date required".into());
                                            return;
                                        }
                                        assign_form.write().submitting = true;
                                        let db_signal = db_signal.clone();
                                        spawn(async move {
                                            if let Some(db) = db_signal.read().db.clone() {
                                                let _ = TerritoryAssignment::create(
                                                        &db,
                                                        TerritoryAssignmentData {
                                                            territory: tid.unwrap(),
                                                            user: uid.unwrap(),
                                                            assigned_date: f.assigned_date.clone(),
                                                            returned_date: None,
                                                        },
                                                    )
                                                    .await;
                                            }
                                            assign_open.set(false);
                                            assign_form.write().submitting = false;
                                            assignments_res.restart();
                                        });
                                    }
                                },
                                {t!("terr-assign")}
                            }
                        }
                    }
                }
            }

            // Fulfill request modal — territory overseer assigns a territory
            if let Some(ref req) = fulfilling.read().clone() {
                {
                    let req = req.clone();
                    let uname = user_map
                        .get(&rid_str(&req.user))
                        .cloned()
                        .unwrap_or_else(|| "—".to_string());
                    let req_id_f = req.id.clone();
                    let req_user = req.user.clone();
                    rsx! {
                        TerritoryModal {
                            title: t!("terr-request-fulfill").to_string(),
                            on_close: move |_| fulfilling.set(None),
                            div { class: "flex flex-col gap-4",
                                // Request summary
                                div { class: "bg-amber-50 border border-amber-100 rounded-lg p-3",
                                    p { class: "text-sm font-semibold text-amber-900", "{uname}" }
                                    p { class: "text-xs text-amber-600", "{req.requested_date}" }
                                    if let Some(ref n) = req.notes {
                                        p { class: "text-xs text-amber-700 italic mt-0.5 break-words", "{n}" }
                                    }
                                }
                                // Territory picker
                                div {
                                    label { class: "block text-xs font-medium text-gray-600 mb-1", {t!("terr-territory")} }
                                    select {
                                        class: "w-full px-3 py-2 rounded-lg border border-gray-200 text-sm focus:outline-none focus:ring-2 focus:ring-primary-300 bg-white",
                                        value: "{fulfill_tid}",
                                        onchange: move |e| fulfill_tid.set(e.value()),
                                        option { value: "", "— Select territory —" }
                                        for t in territories.iter() {
                                            {
                                                let tid = t.id.as_ref().map(rid_str).unwrap_or_default();
                                                rsx! {
                                                    option { value: "{tid}", "#{t.number} – {t.name}" }
                                                }
                                            }
                                        }
                                    }
                                }
                                div { class: "flex gap-2 justify-end",
                                    button {
                                        class: "px-4 py-2 text-sm rounded-lg bg-gray-100 hover:bg-gray-200 text-gray-700",
                                        onclick: move |_| fulfilling.set(None),
                                        {t!("btn-cancel")}
                                    }
                                    button {
                                        class: "px-4 py-2 text-sm rounded-lg bg-primary-600 hover:bg-primary-700 text-white font-medium",
                                        onclick: {
                                            let db_signal = db_signal.clone();
                                            move |_| {
                                                let tid_str = fulfill_tid.read().clone();
                                                let Some(tid) = parse_rid(&tid_str) else {
                                                    return;
                                                };
                                                let db_signal = db_signal.clone();
                                                let req_id_inner = req_id_f.clone();
                                                let req_user_inner = req_user.clone();
                                                fulfilling.set(None);
                                                spawn(async move {
                                                    if let Some(db) = db_signal.read().db.clone() {
                                                        let _ = TerritoryAssignment::create(
                                                                &db,
                                                                TerritoryAssignmentData {
                                                                    territory: tid,
                                                                    user: req_user_inner,
                                                                    assigned_date: today_iso(),
                                                                    returned_date: None,
                                                                },
                                                            )
                                                            .await;
                                                        if let Some(rid) = req_id_inner {
                                                            let _ = TerritoryRequest::fulfill(&db, rid).await;
                                                        }
                                                    }
                                                    requests_res.restart();
                                                    assignments_res.restart();
                                                });
                                            }
                                        },
                                        {t!("terr-assign")}
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
