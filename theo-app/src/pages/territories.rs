//! *Ministry › Territories*: assigning territories (available, assigned with the late ones, and
//! the registry), the territories with their maps, statistics, and the settings (late limit,
//! map provider, categories and labels). A territory has its own page (map, do-not-calls,
//! history) and an editor (map drawing). Publishers see the territories they have.
//! Rules: `docs/model/m07-territories.md`.

use crate::people::{parse_date, today_iso, Directory, Label, LabelColor};
use crate::prefs::Preferences;
use crate::state::use_app_state;
use crate::territories::{
    days_between, duration_label, from_camera, from_map_shapes, import_shapes, months_text, to_camera, to_map_shapes, DoNotCall, MapProvider, ShapeKind, TagKind, Territory, TerritoryAssignment,
    TerritoryDraft,
};
use crate::Route;
use dioxus::prelude::*;
use std::collections::BTreeSet;
use theo_ui::*;
use theo_ui::Action; // explicit: dioxus::prelude also exports an `Action`

pub(crate) const PAGE: &str = "ministry/territories";

fn date(prefs: &Preferences, iso: &str) -> String {
    parse_date(iso).map(|d| prefs.format_date(d)).unwrap_or_else(|| iso.to_string())
}

fn plural(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

fn name(dir: &Directory, prefs: &Preferences, person: u32) -> String {
    dir.person(person).map(|p| p.display_name(prefs)).unwrap_or_else(|| "Unknown person".into())
}

fn tag_of(l: &Label, icon: Icon) -> Tag {
    Tag::new(l.name.clone(), l.color.tone()).with_icon(icon)
}

/// Category and label tags of a territory.
fn territory_tags(dir: &Directory, t: &Territory) -> Vec<Tag> {
    let mut tags = Vec::new();
    if let Some(c) = t.category.and_then(|c| dir.territory_tag(TagKind::Category, c)) {
        tags.push(tag_of(c, Icon::Map));
    }
    for l in t.labels.iter().filter_map(|l| dir.territory_tag(TagKind::Label, *l)) {
        tags.push(tag_of(l, Icon::Tag));
    }
    tags
}

/// The labels of an assignment as tags.
fn assignment_tags(dir: &Directory, a: &TerritoryAssignment) -> Vec<Tag> {
    a.labels.iter().filter_map(|l| dir.territory_tag(TagKind::Assignment, *l)).map(|l| tag_of(l, Icon::Tag)).collect()
}

/// "Assigned for more than 120 days (about 4 months)."
fn late_rule(dir: &Directory) -> String {
    let d = dir.territory_settings.late_days;
    format!("Assigned for more than {d} days (about {}).", months_text(d))
}

/// Tap to put assignment labels on or off.
#[component]
fn AssignmentLabelsField(selected: BTreeSet<u32>, onchange: EventHandler<BTreeSet<u32>>) -> Element {
    let state = use_app_state();
    let labels = state.directory().assignment_labels.clone();
    if labels.is_empty() {
        return rsx! {};
    }
    rsx! {
        Field { label: "Labels", help: "For example when it was given for a campaign.",
            Cluster { gap: Gap::Sm,
                for l in labels {
                    {
                        let on = selected.contains(&l.id);
                        let sel = selected.clone();
                        rsx! {
                            TagToggle {
                                key: "{l.id}",
                                label: l.name.clone(),
                                tone: l.color.tone(),
                                selected: on,
                                onclick: move |_| {
                                    let mut next = sel.clone();
                                    if !next.remove(&l.id) {
                                        next.insert(l.id);
                                    }
                                    onchange.call(next);
                                },
                            }
                        }
                    }
                }
            }
        }
    }
}

fn dnc_tag(t: &Territory) -> Option<Tag> {
    match t.do_not_calls.len() {
        0 => None,
        n => Some(Tag::new(format!("{n} do not call"), Tone::Error).with_icon(Icon::DoNotCall)),
    }
}

/// "available", "assigned" or "late".
fn status_of(dir: &Directory, t: &Territory, today: &str) -> &'static str {
    match dir.current_assignment(t.id) {
        None => "available",
        Some(a) if dir.is_late(a, today) => "late",
        Some(_) => "assigned",
    }
}

const FACET_CATEGORY: &str = "category";
const FACET_LABEL: &str = "label";
const FACET_STATUS: &str = "status";

fn matches_query(dir: &Directory, prefs: &Preferences, t: &Territory, q: &str) -> bool {
    let q = crate::people::fold(q.trim());
    if q.is_empty() {
        return true;
    }
    let mut hay = vec![t.title.clone(), t.description.clone().unwrap_or_default()];
    hay.extend(t.do_not_calls.iter().map(|d| d.address.clone()));
    hay.extend(t.shapes.iter().filter_map(|s| s.title.clone()));
    if let Some(a) = dir.current_assignment(t.id) {
        hay.push(name(dir, prefs, a.person));
    }
    hay.iter().any(|h| crate::people::fold(h).contains(&q))
}

fn matches_facet(dir: &Directory, t: &Territory, today: &str, facet: &str, chosen: &BTreeSet<String>) -> bool {
    if chosen.is_empty() {
        return true;
    }
    match facet {
        FACET_CATEGORY => chosen.contains(&t.category.map(|c| c.to_string()).unwrap_or_else(|| "none".into())),
        FACET_LABEL => t.labels.iter().any(|l| chosen.contains(&l.to_string())),
        FACET_STATUS => {
            let s = status_of(dir, t, today);
            chosen.contains(s) || (s == "late" && chosen.contains("assigned"))
        }
        _ => true,
    }
}

/// The territories that pass the search and filters (all but `except`).
fn filtered<'a>(dir: &'a Directory, prefs: &Preferences, query: &str, sel: &FilterSelection, except: Option<&str>) -> Vec<&'a Territory> {
    let today = today_iso();
    dir.territories_sorted()
        .into_iter()
        .filter(|t| matches_query(dir, prefs, t, query))
        .filter(|t| sel.iter().filter(|(f, _)| Some(f.as_str()) != except).all(|(f, chosen)| matches_facet(dir, t, &today, f, chosen)))
        .collect()
}

fn facets(dir: &Directory, prefs: &Preferences, query: &str, sel: &FilterSelection) -> Vec<Facet> {
    let today = today_iso();
    let count = |facet: &str, value: &str| {
        let one: BTreeSet<String> = std::iter::once(value.to_string()).collect();
        filtered(dir, prefs, query, sel, Some(facet)).iter().filter(|t| matches_facet(dir, t, &today, facet, &one)).count()
    };
    let mut out = Vec::new();
    out.push(Facet {
        id: FACET_STATUS.into(),
        label: "Status".into(),
        icon: Icon::Assignment,
        options: [("available", "Available"), ("assigned", "Assigned"), ("late", "Late")]
            .iter()
            .map(|(v, l)| FacetOption { value: v.to_string(), label: l.to_string(), count: count(FACET_STATUS, v) })
            .collect(),
    });
    if !dir.territory_categories.is_empty() {
        let mut options: Vec<FacetOption> = dir.territory_categories.iter().map(|c| FacetOption { value: c.id.to_string(), label: c.name.clone(), count: count(FACET_CATEGORY, &c.id.to_string()) }).collect();
        options.push(FacetOption { value: "none".into(), label: "No category".into(), count: count(FACET_CATEGORY, "none") });
        out.push(Facet { id: FACET_CATEGORY.into(), label: "Category".into(), icon: Icon::Map, options });
    }
    if !dir.territory_labels.is_empty() {
        out.push(Facet {
            id: FACET_LABEL.into(),
            label: "Label".into(),
            icon: Icon::Tag,
            options: dir.territory_labels.iter().map(|c| FacetOption { value: c.id.to_string(), label: c.name.clone(), count: count(FACET_LABEL, &c.id.to_string()) }).collect(),
        });
    }
    out
}

// ---------------------------------------------------------------------------------------------
// Cards (PDF)
// ---------------------------------------------------------------------------------------------

const CARD_W: f64 = 1000.0;
const CARD_H: f64 = 680.0;

/// Builds the card of a territory: reads the map tiles into an image when maps are on.
async fn card_of(dir: &Directory, prefs: &Preferences, congregation: &str, t: &Territory) -> crate::territory_card::Card {
    let tiles = dir.territory_settings.tiles();
    let pts: Vec<GeoPoint> = t.shapes.iter().flat_map(|s| s.points.iter().map(|p| GeoPoint::new(p.lat, p.lon))).collect();
    let max = tiles.as_ref().map(|t| t.max_zoom).unwrap_or(19);
    let cam = t.view.map(to_camera).or_else(|| fit_camera(&pts, CARD_W, CARD_H, max)).or_else(|| dir.territory_settings.home.map(to_camera));
    let (jpeg, shapes) = match cam {
        Some(cam) => {
            let jpeg = match &tiles {
                Some(src) => crate::platform::render_tiles(&tile_grid(src, cam, CARD_W, CARD_H), CARD_W as u32, CARD_H as u32).await,
                None => None,
            };
            let shapes = t
                .shapes
                .iter()
                .map(|s| (s.kind, s.points.iter().map(|p| to_pixels(cam, CARD_W, CARD_H, GeoPoint::new(p.lat, p.lon))).collect(), s.title.clone(), crate::territory_card::marker_rgb(s.color)))
                .collect();
            (jpeg, shapes)
        }
        None => (None, Vec::new()),
    };
    let labels: Vec<String> = t.labels.iter().filter_map(|l| dir.territory_tag(TagKind::Label, *l)).map(|l| l.name.clone()).collect();
    crate::territory_card::Card {
        title: t.title.clone(),
        congregation: congregation.into(),
        category: t.category.and_then(|c| dir.territory_tag(TagKind::Category, c)).map(|c| c.name.clone()).unwrap_or_default(),
        labels: labels.join(" · "),
        description: t.description.clone(),
        do_not_calls: t
            .do_not_calls
            .iter()
            .map(|d| {
                let mut parts = vec![d.address.clone()];
                parts.extend(d.note.clone());
                parts.extend(d.date.as_deref().map(|x| date(prefs, x)));
                parts.join(" · ")
            })
            .collect(),
        url: t.url.clone(),
        printed: format!("Printed {}", date(prefs, &today_iso())),
        map: crate::territory_card::CardMap { jpeg, width: CARD_W, height: CARD_H, shapes, attribution: tiles.map(|t| t.attribution).unwrap_or_default() },
    }
}

/// Saves the cards of `ids` as one PDF.
fn export_cards(ids: Vec<u32>, file: String, toaster: Toaster) {
    let state = use_app_state();
    let dir = state.directory();
    let prefs = state.preferences();
    let congregation = state.active_congregation().map(|c| c.name).unwrap_or_default();
    if ids.is_empty() {
        toaster.error("There are no territories to print.");
        return;
    }
    toaster.show(Tone::Info, if ids.len() == 1 { "Preparing the card…".to_string() } else { format!("Preparing {} cards…", ids.len()) });
    spawn(async move {
        let mut cards = Vec::new();
        for id in ids {
            if let Some(t) = dir.territory(id) {
                cards.push(card_of(&dir, &prefs, &congregation, t).await);
            }
        }
        let bytes = crate::territory_card::cards_pdf(&cards);
        let name = format!("{}-{file}.pdf", crate::people::fold(&congregation).replace(' ', "-"));
        match crate::platform::save_file(&name, "application/pdf", &bytes) {
            Ok(m) => toaster.success(m),
            Err(m) => toaster.error(m),
        }
    });
}

fn registry_table(dir: &Directory, prefs: &Preferences, congregation: &str, list: &[&Territory]) -> crate::export::Table {
    let mut rows = Vec::new();
    for t in list {
        let history = dir.territory_history(t.id);
        if history.is_empty() {
            rows.push(vec![t.title.clone(), "Never assigned".into(), String::new(), String::new(), String::new(), String::new()]);
        }
        for a in history {
            let days = days_between(&a.assigned, a.completed.as_deref().unwrap_or(&today_iso())).unwrap_or(0);
            rows.push(vec![
                t.title.clone(),
                name(dir, prefs, a.person),
                date(prefs, &a.assigned),
                a.completed.as_deref().map(|c| date(prefs, c)).unwrap_or_else(|| "Still assigned".into()),
                days.to_string(),
                assignment_tags(dir, a).into_iter().map(|t| t.label).collect::<Vec<_>>().join(", "),
            ]);
        }
    }
    crate::export::Table {
        title: format!("{congregation} · Territory assignment record"),
        subtitle: format!("Newest first per territory · {}", date(prefs, &today_iso())),
        columns: ["Territory", "Publisher", "Assigned", "Completed", "Days", "Labels"].iter().map(|s| s.to_string()).collect(),
        widths: vec![1.0, 2.0, 1.3, 1.3, 0.6, 1.8],
        rows,
    }
}

// ---------------------------------------------------------------------------------------------
// The page
// ---------------------------------------------------------------------------------------------

/// `/app/ministry/territories`
#[component]
pub fn TerritoriesPage() -> Element {
    let mut state = use_app_state();
    let toaster = use_toaster();
    let navigator = navigator();
    let congregation = state.active_congregation().map(|c| c.name).unwrap_or_default();
    if !state.can_read(PAGE) {
        return rsx! {
            Page { title: "Territories",
                EmptyState { icon: Icon::Lock, title: "Only for the territory servant and helpers", description: "Your own territories are in the My territories widget on the home page. Ask an administrator if you need access." }
            }
        };
    }
    let writer = state.can_write(PAGE);
    let prefs = state.preferences();
    let dir = state.directory();
    let today = today_iso();
    let tab = (state.terr_tab)();
    let part = (state.terr_part)();
    let query = (state.terr_query)();
    let sel = (state.terr_filters)();
    let list = filtered(&dir, &prefs, &query, &sel, None);
    let available = list.iter().filter(|t| dir.current_assignment(t.id).is_none()).count();
    let assigned = list.len() - available;
    let late = list.iter().filter(|t| status_of(&dir, t, &today) == "late").count();
    let ids: Vec<u32> = list.iter().map(|t| t.id).collect();
    let summary = format!("{} of {} territories", list.len(), dir.territories.len());
    let facets = facets(&dir, &prefs, &query, &sel);

    let on_export = {
        let dir = dir.clone();
        let prefs = prefs.clone();
        let congregation = congregation.clone();
        let ids = ids.clone();
        move |id: String| {
            if id == "cards" {
                export_cards(ids.clone(), "territory-cards".into(), toaster);
                return;
            }
            let Some(format) = id.strip_prefix("registry-").and_then(crate::export::Format::parse) else { return };
            let list: Vec<&Territory> = ids.iter().filter_map(|i| dir.territory(*i)).collect();
            let table = registry_table(&dir, &prefs, &congregation, &list);
            super::users::save_export(&table, format, &congregation, "territory-registry", toaster);
        }
    };
    let export = rsx! {
        MenuButton {
            label: "Export",
            icon: Icon::Download,
            items: vec![
                MenuItem::new("cards", "Territory cards", Icon::FilePdf).with_description("PDF, one page per territory shown, with its map"),
                MenuItem::new("registry-pdf", "Registry as PDF", Icon::FilePdf).with_description("Who had each territory, newest first"),
                MenuItem::new("registry-docx", "Registry as Word", Icon::FileDocx).with_description("The same, to edit"),
                MenuItem::new("registry-csv", "Registry as CSV", Icon::FileCsv).with_description("For spreadsheets"),
            ],
            onselect: on_export,
        }
    };
    let actions = rsx! {
        if writer {
            ActionButton { action: Action::Settings, variant: ButtonVariant::Secondary, label: "Settings", onclick: move |_| { navigator.push(Route::TerritorySettingsPage {}); } }
        }
        {export}
        if writer {
            ActionButton { action: Action::Add, label: "Add territory", onclick: move |_| { navigator.push(Route::TerritoryNew {}); } }
        }
    };
    let late_text = late_rule(&dir);

    rsx! {
        Page {
            title: "Territories",
            subtitle: "{congregation} · Who has which territory, the maps and the registry",
            actions,
            StickyBar {
                Segmented {
                    value: tab.clone(),
                    options: vec![
                        Segment { value: "assignments".into(), label: "Assignments".into(), icon: Some(Icon::Assignment) },
                        Segment { value: "territories".into(), label: "Territories".into(), icon: Some(Icon::Map) },
                        Segment { value: "statistics".into(), label: "Statistics".into(), icon: Some(Icon::Chart) },
                    ],
                    onchange: move |v: String| state.terr_tab.set(v),
                }
                if tab == "assignments" {
                    Segmented {
                        value: part.clone(),
                        options: vec![
                            Segment { value: "assigned".into(), label: if late > 0 { format!("Assigned ({assigned} · {late} late)") } else { format!("Assigned ({assigned})") }, icon: Some(Icon::User) },
                            Segment { value: "available".into(), label: format!("Available ({available})"), icon: Some(Icon::Check) },
                            Segment { value: "registry".into(), label: "Registry".into(), icon: Some(Icon::History) },
                        ],
                        onchange: move |v: String| state.terr_part.set(v),
                    }
                }
            }
            if dir.territories.is_empty() {
                EmptyState {
                    icon: Icon::Map,
                    title: "No territories yet",
                    description: if writer { "Add the first territory: a title, its area on the map and the do-not-call addresses." } else { "They appear here when the territory servant adds them." },
                }
            } else if tab == "statistics" {
                StatisticsTab {}
            } else {
                FilterBar {
                    query: query.clone(),
                    on_query: move |q| state.terr_query.set(q),
                    search_placeholder: "Search title, address, publisher…",
                    facets,
                    selected: sel.clone(),
                    on_select: move |s| state.terr_filters.set(s),
                    summary,
                }
                if tab == "territories" {
                    TerritoriesTab { ids: ids.clone() }
                } else {
                    match part.as_str() {
                        "available" => rsx! { AvailablePart { ids: ids.clone(), writer } },
                        "registry" => rsx! { RegistryPart { ids: ids.clone(), writer } },
                        _ => rsx! {
                            if late > 0 {
                                Alert { tone: Tone::Warning, title: if late == 1 { "1 territory is late".to_string() } else { format!("{late} territories are late") },
                                    "{late_text} They are shown first."
                                }
                            }
                            AssignedPart { ids: ids.clone(), writer }
                        },
                    }
                }
            }
        }
    }
}

#[component]
fn AvailablePart(ids: Vec<u32>, writer: bool) -> Element {
    let state = use_app_state();
    let navigator = navigator();
    let prefs = state.preferences();
    let dir = state.directory();
    let today = today_iso();
    let mut assigning = use_signal(|| None::<u32>);
    // Longest since it was worked first; never worked before everything.
    let mut list: Vec<(&Territory, Option<String>)> = ids.iter().filter_map(|i| dir.territory(*i)).filter(|t| dir.current_assignment(t.id).is_none()).map(|t| (t, dir.last_completed(t.id))).collect();
    list.sort_by(|a, b| a.1.cmp(&b.1));
    if list.is_empty() {
        return rsx! { EmptyState { icon: Icon::Check, title: "Every territory is assigned", description: "Territories that come back appear here." } };
    }
    rsx! {
        Text { tone: TextTone::Small, "Longest since it was last completed first." }
        Grid { cols: 3, gap: Gap::Sm,
            for (t, last) in list {
                {
                    let id = t.id;
                    let mut tags = territory_tags(&dir, t);
                    if let Some(d) = dnc_tag(t) {
                        tags.push(d);
                    }
                    let subtitle = match &last {
                        Some(l) => format!("Last completed {} · {} ago", date(&prefs, l), duration_label(days_between(l, &today).unwrap_or(0))),
                        None => "Never completed".to_string(),
                    };
                    rsx! {
                        RecordCard {
                            key: "{id}",
                            title: format!("Territory {}", t.title),
                            avatar: t.title.clone(),
                            subtitle,
                            tags,
                            onclick: move |_| if writer { assigning.set(Some(id)) } else { navigator.push(Route::TerritoryDetail { id }); },
                        }
                    }
                }
            }
        }
        if let Some(t) = assigning() {
            AssignDialog { territory: t, onclose: move |_| assigning.set(None) }
        }
    }
}

#[component]
fn AssignedPart(ids: Vec<u32>, writer: bool) -> Element {
    let state = use_app_state();
    let navigator = navigator();
    let prefs = state.preferences();
    let dir = state.directory();
    let today = today_iso();
    let mut returning = use_signal(|| None::<u32>);
    let mut list: Vec<(&Territory, &crate::territories::TerritoryAssignment)> = ids.iter().filter_map(|i| dir.territory(*i)).filter_map(|t| dir.current_assignment(t.id).map(|a| (t, a))).collect();
    // Late first, then the oldest.
    list.sort_by(|a, b| (!dir.is_late(a.1, &today), &a.1.assigned).cmp(&(!dir.is_late(b.1, &today), &b.1.assigned)));
    if list.is_empty() {
        return rsx! { EmptyState { icon: Icon::Map, title: "No territory is assigned", description: "Assign one from Available." } };
    }
    rsx! {
        Grid { cols: 3, gap: Gap::Sm,
            for (t, a) in list {
                {
                    let id = t.id;
                    let aid = a.id;
                    let days = days_between(&a.assigned, &today).unwrap_or(0);
                    let mut tags = Vec::new();
                    if dir.is_late(a, &today) {
                        tags.push(Tag::new(format!("Late · {}", duration_label(days)), Tone::Error).with_icon(Icon::Warning));
                    } else {
                        tags.push(Tag::new(duration_label(days), Tone::Neutral).with_icon(Icon::Clock));
                    }
                    tags.extend(assignment_tags(&dir, a));
                    tags.extend(territory_tags(&dir, t));
                    if let Some(d) = dnc_tag(t) {
                        tags.push(d);
                    }
                    rsx! {
                        RecordCard {
                            key: "{id}",
                            title: format!("Territory {}", t.title),
                            avatar: t.title.clone(),
                            subtitle: format!("{} · since {}", name(&dir, &prefs, a.person), date(&prefs, &a.assigned)),
                            tags,
                            onclick: move |_| if writer { returning.set(Some(aid)) } else { navigator.push(Route::TerritoryDetail { id }); },
                        }
                    }
                }
            }
        }
        if let Some(a) = returning() {
            ReturnDialog { assignment: a, onclose: move |_| returning.set(None) }
        }
    }
}

#[component]
fn RegistryPart(ids: Vec<u32>, writer: bool) -> Element {
    let state = use_app_state();
    let prefs = state.preferences();
    let dir = state.directory();
    let today = today_iso();
    let mut editing = use_signal(|| None::<Option<u32>>);
    let list: Vec<&Territory> = ids.iter().filter_map(|i| dir.territory(*i)).collect();
    let columns: Vec<(String, String)> = list
        .iter()
        .map(|t| {
            let sub = match dir.last_completed(t.id) {
                Some(l) => format!("Last completed {}", date(&prefs, &l)),
                None => "Never completed".to_string(),
            };
            (format!("Territory {}", t.title), sub)
        })
        .collect();
    let history: Vec<Vec<crate::territories::TerritoryAssignment>> = list.iter().map(|t| dir.territory_history(t.id).into_iter().cloned().collect()).collect();
    let cells: Vec<Vec<MatrixCell>> = history
        .iter()
        .map(|h| {
            h.iter()
                .map(|a| MatrixCell {
                    title: name(&dir, &prefs, a.person),
                    detail: match &a.completed {
                        Some(c) => format!("{} – {}", date(&prefs, &a.assigned), date(&prefs, c)),
                        None => format!("Since {}", date(&prefs, &a.assigned)),
                    },
                    tone: match &a.completed {
                        Some(_) => Tone::Neutral,
                        None if dir.is_late(a, &today) => Tone::Error,
                        None => Tone::Info,
                    },
                    tags: assignment_tags(&dir, a),
                })
                .collect()
        })
        .collect();
    rsx! {
        Cluster { justify: Justify::Between,
            Text { tone: TextTone::Small, "One column per territory, newest first. Blue: assigned now. Red: late." }
            if writer {
                ActionButton { action: Action::Add, variant: ButtonVariant::Secondary, label: "Add a past assignment", onclick: move |_| editing.set(Some(None)) }
            }
        }
        MatrixTable {
            columns,
            cells,
            empty: "Never assigned",
            caption: "Territory assignment record",
            on_select: if writer {
                Some(EventHandler::new(move |(c, r): (usize, usize)| {
                    if let Some(a) = history.get(c).and_then(|h| h.get(r)) {
                        editing.set(Some(Some(a.id)));
                    }
                }))
            } else {
                None
            },
        }
        if let Some(id) = editing() {
            AssignmentDialog { key: "{id:?}", id, onclose: move |_| editing.set(None) }
        }
    }
}

#[component]
fn TerritoriesTab(ids: Vec<u32>) -> Element {
    let mut state = use_app_state();
    let navigator = navigator();
    let prefs = state.preferences();
    let dir = state.directory();
    let today = today_iso();
    let view = (state.terr_view)();
    let list: Vec<&Territory> = ids.iter().filter_map(|i| dir.territory(*i)).collect();
    let shapes: Vec<MapShape> = list
        .iter()
        .flat_map(|t| {
            let tone = match status_of(&dir, t, &today) {
                "available" => MapTone::Success,
                "late" => MapTone::Error,
                _ => MapTone::Info,
            };
            let id = t.id.to_string();
            let title = t.title.clone();
            t.shapes.iter().filter(|s| s.kind == ShapeKind::Area).enumerate().map(move |(i, s)| MapShape {
                id: id.clone(),
                kind: theo_ui::ShapeKind::Area,
                points: s.points.iter().map(|p| GeoPoint::new(p.lat, p.lon)).collect(),
                label: if i == 0 { title.clone() } else { String::new() },
                tone,
            })
        })
        .collect();
    rsx! {
        Segmented {
            value: view.clone(),
            options: vec![
                Segment { value: "list".into(), label: "List".into(), icon: Some(Icon::Report) },
                Segment { value: "map".into(), label: "Map".into(), icon: Some(Icon::Map) },
            ],
            onchange: move |v: String| state.terr_view.set(v),
        }
        if list.is_empty() {
            EmptyState { icon: Icon::Search, title: "No territory matches", description: "Change the search or the filters." }
        } else if view == "map" {
            MapView {
                tiles: dir.territory_settings.tiles(),
                shapes,
                fallback: dir.territory_settings.home.map(to_camera),
                size: MapSize::Lg,
                on_select: move |id: String| {
                    if let Ok(id) = id.parse::<u32>() {
                        navigator.push(Route::TerritoryDetail { id });
                    }
                },
            }
            Text { tone: TextTone::Small, "Green: available. Blue: assigned. Red: late. Tap a territory to open it." }
        } else {
            Grid { cols: 3, gap: Gap::Sm,
                for t in list {
                    {
                        let id = t.id;
                        let mut tags = territory_tags(&dir, t);
                        if let Some(d) = dnc_tag(t) {
                            tags.push(d);
                        }
                        if t.areas() == 0 {
                            tags.push(Tag::new("No area on the map", Tone::Warning).with_icon(Icon::Map));
                        }
                        let status = match dir.current_assignment(id) {
                            Some(a) => format!("{}{}", name(&dir, &prefs, a.person), if dir.is_late(a, &today) { " · late" } else { "" }),
                            None => "Available".to_string(),
                        };
                        rsx! {
                            RecordCard {
                                key: "{id}",
                                title: format!("Territory {}", t.title),
                                avatar: t.title.clone(),
                                subtitle: status,
                                tags,
                                onclick: move |_| { navigator.push(Route::TerritoryDetail { id }); },
                            }
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Dialogs: assign, return, correct the registry
// ---------------------------------------------------------------------------------------------

fn people_options(dir: &Directory, prefs: &Preferences) -> Vec<SelectOption> {
    let mut v = vec![SelectOption::new("", "Choose a publisher")];
    v.extend(dir.territory_people().iter().map(|p| {
        let n = dir.my_territories(p.id).len();
        let label = match n {
            0 => p.display_name(prefs),
            1 => format!("{} · has 1", p.display_name(prefs)),
            n => format!("{} · has {n}", p.display_name(prefs)),
        };
        SelectOption::new(p.id.to_string(), label)
    }));
    v
}

#[component]
fn AssignDialog(territory: u32, onclose: EventHandler<()>) -> Element {
    let mut state = use_app_state();
    let toaster = use_toaster();
    let navigator = navigator();
    let prefs = state.preferences();
    let dir = state.directory();
    let mut person = use_signal(String::new);
    let mut when = use_signal(today_iso);
    let mut labels = use_signal(BTreeSet::<u32>::new);
    let mut error = use_signal(|| None::<String>);
    let Some(t) = dir.territory(territory).cloned() else { return rsx! {} };
    let last = dir.last_completed(territory);
    let save = move |_| {
        let Ok(p) = person().parse::<u32>() else {
            error.set(Some("Choose a publisher.".into()));
            return;
        };
        let chosen = labels();
        match state.update_directory(|d| d.assign_territory(territory, p, &when()).inspect(|id| d.set_assignment_labels(*id, &chosen))) {
            Ok(_) => {
                toaster.success("Territory assigned.");
                onclose.call(());
            }
            Err(e) => error.set(Some(e)),
        }
    };
    rsx! {
        Dialog {
            open: true,
            onclose: move |_| onclose.call(()),
            title: format!("Assign territory {}", t.title),
            description: match &last {
                Some(l) => format!("Last completed {}.", date(&prefs, l)),
                None => "Never completed.".to_string(),
            },
            actions: rsx! {
                ActionButton { action: Action::Cancel, onclick: move |_| onclose.call(()) }
                ActionButton { action: Action::Save, label: "Assign", onclick: save }
            },
            Stack {
                if !t.do_not_calls.is_empty() {
                    Alert { tone: Tone::Error, title: format!("{} do-not-call", t.do_not_calls.len()), "Remind the publisher: they are on the territory page and the card." }
                }
                Field { label: "Publisher", required: true, error: error(),
                    Select { value: person(), options: people_options(&dir, &prefs), onchange: move |v| { person.set(v); error.set(None); } }
                }
                Field { label: "Assigned on",
                    TextInput { kind: InputKind::Date, value: when(), oninput: move |v| when.set(v) }
                }
                AssignmentLabelsField { selected: labels(), onchange: move |l| labels.set(l) }
                TextLink { icon: Icon::ArrowRight, onclick: move |_| { navigator.push(Route::TerritoryDetail { id: territory }); }, "Open the territory" }
            }
        }
    }
}

#[component]
fn ReturnDialog(assignment: u32, onclose: EventHandler<()>) -> Element {
    let mut state = use_app_state();
    let toaster = use_toaster();
    let navigator = navigator();
    let prefs = state.preferences();
    let dir = state.directory();
    let mut when = use_signal(today_iso);
    let mut error = use_signal(|| None::<String>);
    let mut correcting = use_signal(|| false);
    let mut labels = use_signal(|| dir.territory_assignment(assignment).map(|a| a.labels.clone()).unwrap_or_default());
    let Some(a) = dir.territory_assignment(assignment).cloned() else { return rsx! {} };
    let Some(t) = dir.territory(a.territory).cloned() else { return rsx! {} };
    let today = today_iso();
    let days = days_between(&a.assigned, &today).unwrap_or(0);
    let late = dir.is_late(&a, &today);
    let save = move |_| match state.update_directory(|d| {
        d.set_assignment_labels(assignment, &labels());
        d.complete_territory(assignment, &when())
    }) {
        Ok(()) => {
            toaster.success("Territory completed. It is available again.");
            onclose.call(());
        }
        Err(e) => error.set(Some(e)),
    };
    if correcting() {
        return rsx! { AssignmentDialog { id: Some(assignment), onclose: move |_| onclose.call(()) } };
    }
    rsx! {
        Dialog {
            open: true,
            onclose: move |_| onclose.call(()),
            title: format!("Territory {}", t.title),
            description: format!("{} has it since {} ({}).", name(&dir, &prefs, a.person), date(&prefs, &a.assigned), duration_label(days)),
            actions: rsx! {
                ActionButton { action: Action::Cancel, onclick: move |_| onclose.call(()) }
                ActionButton { action: Action::Done, label: "Completed", onclick: save }
            },
            Stack {
                if late {
                    Alert { tone: Tone::Error, title: "Late", "{late_rule(&dir)}" }
                }
                Field { label: "Completed on", error: error(), help: "When the publisher gave it back worked. Then it is available again.",
                    TextInput { kind: InputKind::Date, value: when(), oninput: move |v| { when.set(v); error.set(None); } }
                }
                AssignmentLabelsField { selected: labels(), onchange: move |l| labels.set(l) }
                Cluster { gap: Gap::Md,
                    TextLink { icon: Icon::Edit, onclick: move |_| correcting.set(true), "Correct this assignment" }
                    TextLink { icon: Icon::ArrowRight, onclick: move |_| { navigator.push(Route::TerritoryDetail { id: t.id }); }, "Open the territory" }
                }
            }
        }
    }
}

/// Adds a past assignment (`id` None) or corrects a line of the registry.
#[component]
fn AssignmentDialog(id: Option<u32>, onclose: EventHandler<()>) -> Element {
    let mut state = use_app_state();
    let toaster = use_toaster();
    let prefs = state.preferences();
    let dir = state.directory();
    let existing = id.and_then(|i| dir.territory_assignment(i).cloned());
    let mut territory = use_signal(|| existing.as_ref().map(|a| a.territory.to_string()).unwrap_or_default());
    let mut person = use_signal(|| existing.as_ref().map(|a| a.person.to_string()).unwrap_or_default());
    let mut assigned = use_signal(|| existing.as_ref().map(|a| a.assigned.clone()).unwrap_or_default());
    let mut completed = use_signal(|| existing.as_ref().and_then(|a| a.completed.clone()).unwrap_or_default());
    let mut labels = use_signal(|| existing.as_ref().map(|a| a.labels.clone()).unwrap_or_default());
    let mut error = use_signal(|| None::<String>);
    let mut confirm = use_signal(|| false);
    let mut territories = vec![SelectOption::new("", "Choose a territory")];
    territories.extend(dir.territories_sorted().iter().map(|t| SelectOption::new(t.id.to_string(), format!("Territory {}", t.title))));
    let save = move |_| {
        let (Ok(t), Ok(p)) = (territory().parse::<u32>(), person().parse::<u32>()) else {
            error.set(Some("Choose the territory and the publisher.".into()));
            return;
        };
        let done = completed();
        let done = (!done.trim().is_empty()).then_some(done);
        let chosen = labels();
        let r = state.update_directory(|d| match id {
            Some(i) => d.update_territory_assignment(i, p, &assigned(), done.as_deref()).inspect(|_| d.set_assignment_labels(i, &chosen)),
            None => d.assign_territory(t, p, &assigned()).and_then(|i| {
                d.set_assignment_labels(i, &chosen);
                match done.as_deref() {
                    Some(c) => d.complete_territory(i, c).inspect_err(|_| d.delete_territory_assignment(i)),
                    None => Ok(()),
                }
            }),
        });
        match r {
            Ok(()) => {
                toaster.success("Registry saved.");
                onclose.call(());
            }
            Err(e) => error.set(Some(e)),
        }
    };
    rsx! {
        Dialog {
            open: true,
            onclose: move |_| onclose.call(()),
            title: if id.is_some() { "Correct the assignment" } else { "Add a past assignment" },
            description: "For mistakes, or to copy the paper registry into the app.",
            actions: rsx! {
                EditActions {
                    on_save: save,
                    on_cancel: move |_| onclose.call(()),
                    on_delete: id.is_some().then_some(EventHandler::new(move |_| confirm.set(true))),
                }
            },
            Stack {
                if let Some(e) = error() {
                    Alert { tone: Tone::Error, "{e}" }
                }
                if id.is_none() {
                    Field { label: "Territory", required: true,
                        Select { value: territory(), options: territories, onchange: move |v| territory.set(v) }
                    }
                }
                Field { label: "Publisher", required: true,
                    Select { value: person(), options: people_options(&dir, &prefs), onchange: move |v| person.set(v) }
                }
                Grid { cols: 2,
                    Field { label: "Assigned on", required: true,
                        TextInput { kind: InputKind::Date, value: assigned(), oninput: move |v| assigned.set(v) }
                    }
                    Field { label: "Completed on", help: "Empty: still assigned.",
                        TextInput { kind: InputKind::Date, value: completed(), oninput: move |v| completed.set(v) }
                    }
                }
                AssignmentLabelsField { selected: labels(), onchange: move |l| labels.set(l) }
            }
        }
        ConfirmDialog {
            open: confirm(),
            title: "Delete this line of the registry?",
            message: "Only when it was a mistake. If the territory came back, mark it as completed instead.",
            confirm: Action::Delete,
            oncancel: move |_| confirm.set(false),
            onconfirm: move |_| {
                if let Some(i) = id {
                    state.update_directory(|d| d.delete_territory_assignment(i));
                }
                confirm.set(false);
                toaster.success("Line deleted.");
                onclose.call(());
            },
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Statistics
// ---------------------------------------------------------------------------------------------

#[component]
fn StatisticsTab() -> Element {
    let state = use_app_state();
    let prefs = state.preferences();
    let dir = state.directory();
    let mut period = use_signal(|| "12".to_string());
    let today = today_iso();
    let from = match period().as_str() {
        "12" => crate::territories::add_months(&today, -12),
        "24" => crate::territories::add_months(&today, -24),
        _ => None,
    };
    let s = dir.territory_stats(from.as_deref());
    let per_person = dir.territories_per_person(from.as_deref());
    let top = per_person.iter().map(|r| r.1 + r.2).max().unwrap_or(1).max(1);
    let coverage = (s.covered * 100).checked_div(s.total).unwrap_or(0);
    let per_tag = |kind: TagKind| -> Vec<(String, usize, Tone)> {
        let mut v: Vec<(String, usize, Tone)> = dir
            .territory_tags(kind)
            .iter()
            .map(|l| {
                let n = dir.territories.iter().filter(|t| if kind == TagKind::Category { t.category == Some(l.id) } else { t.labels.contains(&l.id) }).count();
                (l.name.clone(), n, l.color.tone())
            })
            .collect();
        if kind == TagKind::Category {
            let none = dir.territories.iter().filter(|t| t.category.is_none()).count();
            if none > 0 {
                v.push(("No category".into(), none, Tone::Neutral));
            }
        }
        v.sort_by_key(|x| std::cmp::Reverse(x.1));
        v
    };
    let categories = per_tag(TagKind::Category);
    let labels = per_tag(TagKind::Label);
    // Not worked the longest (available or assigned): never first.
    let mut waiting: Vec<(String, Option<String>)> = dir.territories_sorted().iter().map(|t| (t.title.clone(), dir.last_completed(t.id))).collect();
    waiting.sort_by(|a, b| a.1.cmp(&b.1));
    let total = s.total.max(1);
    let period_text = match period().as_str() {
        "12" => "in the last 12 months",
        "24" => "in the last 24 months",
        _ => "since the start",
    };

    rsx! {
        Segmented {
            value: period(),
            options: vec![
                Segment { value: "12".into(), label: "Last 12 months".into(), icon: Some(Icon::Calendar) },
                Segment { value: "24".into(), label: "Last 24 months".into(), icon: Some(Icon::History) },
                Segment { value: "all".into(), label: "All".into(), icon: Some(Icon::Chart) },
            ],
            onchange: move |v: String| period.set(v),
        }
        Grid { cols: 4, gap: Gap::Sm, compact: true,
            Card { StatTile { label: "Territories", value: s.total.to_string(), icon: Icon::Map, delta: format!("{} assigned · {} available", s.assigned, s.total - s.assigned) } }
            Card { StatTile { label: "Late", value: s.late.to_string(), icon: Icon::Warning, delta: format!("over {} days", dir.territory_settings.late_days), trend: if s.late > 0 { Trend::Down } else { Trend::Flat } } }
            Card { StatTile { label: "Average time assigned", value: s.average_days.map(duration_label).unwrap_or_else(|| "–".into()), icon: Icon::Clock, delta: format!("{} completed {period_text}", s.completed) } }
            Card { StatTile { label: "Covered", value: format!("{coverage}%"), icon: Icon::Check, delta: format!("{} of {} completed {period_text}", s.covered, s.total) } }
        }
        Grid { cols: 2,
            Stack {
                Card { title: "Territories per category", icon: Icon::Map,
                    if categories.is_empty() {
                        Text { tone: TextTone::Muted, "No categories yet. Add them in Settings." }
                    } else {
                        Stack { gap: Gap::Sm,
                            for (i, (label, n, tone)) in categories.iter().enumerate() {
                                ProgressRow { key: "{i}", label: label.clone(), detail: format!("{n}"), value: *n as f64, max: total as f64, tone: *tone }
                            }
                        }
                    }
                }
                Card { title: "Territories per label", icon: Icon::Tag,
                    if labels.is_empty() {
                        Text { tone: TextTone::Muted, "No labels yet. Add them in Settings." }
                    } else {
                        Stack { gap: Gap::Sm,
                            for (i, (label, n, tone)) in labels.iter().enumerate() {
                                ProgressRow { key: "{i}", label: label.clone(), detail: format!("{n}"), value: *n as f64, max: total as f64, tone: *tone }
                            }
                        }
                    }
                }
            }
            Stack {
                Card { title: "Not worked for the longest time", icon: Icon::History,
                    ItemList {
                        items: waiting
                            .iter()
                            .take(6)
                            .map(|(title, last)| ListEntry {
                                icon: if last.is_none() { Icon::Warning } else { Icon::Map },
                                title: format!("Territory {title}"),
                                subtitle: match last {
                                    Some(l) => format!("Last completed {}", date(&prefs, l)),
                                    None => "Never completed".into(),
                                },
                                meta: last.as_deref().and_then(|l| days_between(l, &today)).map(duration_label).unwrap_or_default(),
                            })
                            .collect::<Vec<_>>(),
                    }
                }
                {
                    let per_label = dir.assignments_per_label(from.as_deref());
                    let top_label = per_label.iter().map(|x| x.1).max().unwrap_or(1).max(1);
                    rsx! {
                        Card { title: "Assignments per label", icon: Icon::Tag,
                            if per_label.is_empty() {
                                Text { tone: TextTone::Muted, "No assignment labels. Add them in Settings." }
                            } else {
                                Stack { gap: Gap::Sm,
                                    Text { tone: TextTone::Small, "Territories given out {period_text}, by label." }
                                    for (l, n) in per_label.iter() {
                                        if let Some(label) = dir.territory_tag(TagKind::Assignment, *l) {
                                            ProgressRow { key: "{l}", label: label.name.clone(), detail: format!("{n}"), value: *n as f64, max: top_label as f64, tone: label.color.tone() }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Card { title: "Per publisher", icon: Icon::Users,
                    if per_person.is_empty() {
                        Text { tone: TextTone::Muted, "Nobody had a territory {period_text}." }
                    } else {
                        Stack { gap: Gap::Sm,
                            Text { tone: TextTone::Small, "Completed {period_text}, and how many they have now." }
                            for (p, done, now) in per_person.iter().take(12) {
                                ProgressRow {
                                    key: "{p}",
                                    label: name(&dir, &prefs, *p),
                                    detail: format!("{done} completed"),
                                    value: (*done + *now) as f64,
                                    max: top as f64,
                                    note: (*now > 0).then(|| if *now == 1 { "has 1 now".to_string() } else { format!("has {now} now") }),
                                    tone: Tone::Neutral,
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// One territory
// ---------------------------------------------------------------------------------------------

/// `/app/ministry/territories/:id`: the map, the do-not-calls, details and history. Also for the
/// publisher who has it (from *My territories*).
#[component]
pub fn TerritoryDetail(id: u32) -> Element {
    let mut state = use_app_state();
    let toaster = use_toaster();
    let navigator = navigator();
    let prefs = state.preferences();
    let dir = state.directory();
    let reader = state.can_read(PAGE);
    let writer = state.can_write(PAGE);
    let mine = state.me().is_some_and(|me| dir.has_territory(me, id));
    let mut assigning = use_signal(|| false);
    let mut returning = use_signal(|| None::<u32>);
    let back = if reader {
        ("Territories".to_string(), EventHandler::new(move |_| { navigator.push(Route::TerritoriesPage {}); }))
    } else {
        ("Home".to_string(), EventHandler::new(move |_| { navigator.push(Route::Dashboard {}); }))
    };
    let Some(t) = dir.territory(id).cloned().filter(|_| reader || mine) else {
        return rsx! {
            Page { title: "Territory", back,
                EmptyState { icon: Icon::Lock, title: "This territory is not available", description: "It was deleted, or it is not assigned to you." }
            }
        };
    };
    let today = today_iso();
    let current = dir.current_assignment(id).cloned();
    let history: Vec<crate::territories::TerritoryAssignment> = dir.territory_history(id).into_iter().filter(|a| a.completed.is_some()).cloned().collect();
    let tags = territory_tags(&dir, &t);
    let subtitle = {
        let mut parts: Vec<String> = Vec::new();
        if let Some(c) = t.category.and_then(|c| dir.territory_tag(TagKind::Category, c)) {
            parts.push(c.name.clone());
        }
        parts.push(match &current {
            Some(a) => format!("{} since {}", name(&dir, &prefs, a.person), date(&prefs, &a.assigned)),
            None => "Available".into(),
        });
        parts.join(" · ")
    };
    let title = t.title.clone();
    // Directions to the middle of the territory, in the device's map app.
    let directions = {
        let areas: Vec<&crate::territories::Point> = t.shapes.iter().filter(|s| s.kind == ShapeKind::Area).flat_map(|s| s.points.iter()).collect();
        let pts: Vec<&crate::territories::Point> = if areas.is_empty() { t.shapes.iter().flat_map(|s| s.points.iter()).collect() } else { areas };
        (!pts.is_empty()).then(|| {
            let n = pts.len() as f64;
            let lat = pts.iter().map(|p| p.lat).sum::<f64>() / n;
            let lon = pts.iter().map(|p| p.lon).sum::<f64>() / n;
            crate::platform::directions_url(lat, lon, &format!("Territory {}", t.title))
        })
    };
    rsx! {
        Page {
            title: format!("Territory {}", t.title),
            subtitle,
            back,
            actions: rsx! {
                if let Some(href) = directions.clone() {
                    LinkButton { href, label: "Directions", icon: Icon::Directions }
                }
                ActionButton { action: Action::Export, variant: ButtonVariant::Secondary, label: "Card (PDF)", onclick: move |_| export_cards(vec![id], format!("territory-{}", crate::people::fold(&title).replace(' ', "-")), toaster) }
                if writer {
                    ActionButton { action: Action::Edit, onclick: move |_| { navigator.push(Route::TerritoryEdit { id }); } }
                }
            },
            if !t.do_not_calls.is_empty() {
                Alert { tone: Tone::Error, title: format!("Do not call ({})", t.do_not_calls.len()),
                    Stack { gap: Gap::Xs,
                        for (i, d) in t.do_not_calls.iter().enumerate() {
                            Text { key: "{i}",
                                strong { "{d.address}" }
                                if let Some(n) = &d.note { " · {n}" }
                                if let Some(x) = &d.date { " · {date(&prefs, x)}" }
                            }
                        }
                    }
                }
            }
            if !tags.is_empty() {
                Cluster { gap: Gap::Sm,
                    for (i, tg) in tags.iter().enumerate() {
                        Badge { key: "{i}", tone: tg.tone, icon: tg.icon.unwrap_or(Icon::Tag), "{tg.label}" }
                    }
                }
            }
            if t.shapes.is_empty() {
                Alert { tone: Tone::Info, title: "No map yet", if writer { "Edit the territory to draw its area." } else { "The territory servant has not drawn it yet." } }
            }
            MapView {
                tiles: dir.territory_settings.tiles(),
                shapes: to_map_shapes(&t.shapes),
                camera: t.view.map(to_camera),
                fallback: dir.territory_settings.home.map(to_camera),
                size: MapSize::Lg,
                on_save_view: if writer {
                    Some(EventHandler::new(move |cam: MapCamera| {
                        state.update_directory(|d| d.set_territory_view(id, Some(from_camera(cam))));
                        toaster.success("This is now the default view of the map.");
                    }))
                } else {
                    None
                },
            }
            Grid { cols: 2,
                Card { title: "Details", icon: Icon::Note,
                    Stack { gap: Gap::Sm,
                        match &t.description {
                            Some(d) => rsx! { Text { "{d}" } },
                            None => rsx! { Text { tone: TextTone::Muted, "No description." } },
                        }
                        if let Some(u) = t.url.clone() {
                            ExternalLink { href: u.clone(), "{u}" }
                        }
                        {
                            let markers: Vec<String> = t.shapes.iter().filter(|s| s.kind == ShapeKind::Marker).filter_map(|s| s.title.clone()).collect();
                            rsx! {
                                if !markers.is_empty() {
                                    Text { tone: TextTone::Small, "Markers: {markers.join(\" · \")}" }
                                }
                            }
                        }
                    }
                }
                Card { title: "Assignment", icon: Icon::Assignment,
                    Stack { gap: Gap::Sm,
                        match &current {
                            Some(a) => {
                                let late = dir.is_late(a, &today);
                                let aid = a.id;
                                rsx! {
                                    Cluster { justify: Justify::Between,
                                        Text { strong { "{name(&dir, &prefs, a.person)}" } " · since {date(&prefs, &a.assigned)}" }
                                        if late {
                                            Badge { tone: Tone::Error, icon: Icon::Warning, "Late" }
                                        }
                                    }
                                    if writer {
                                        Cluster { gap: Gap::Sm,
                                            ActionButton { action: Action::Done, variant: ButtonVariant::Secondary, label: "Completed…", onclick: move |_| returning.set(Some(aid)) }
                                        }
                                    }
                                }
                            }
                            None => rsx! {
                                Text { tone: TextTone::Muted, "Available." }
                                if writer {
                                    Cluster { gap: Gap::Sm,
                                        ActionButton { action: Action::Add, variant: ButtonVariant::Secondary, label: "Assign…", onclick: move |_| assigning.set(true) }
                                    }
                                }
                            },
                        }
                        if reader {
                            if history.is_empty() {
                                Text { tone: TextTone::Small, "Never completed." }
                            } else {
                                ItemList {
                                    items: history
                                        .iter()
                                        .take(8)
                                        .map(|a| ListEntry {
                                            icon: Icon::History,
                                            title: name(&dir, &prefs, a.person),
                                            subtitle: {
                                                let mut t = format!("{} – {}", date(&prefs, &a.assigned), a.completed.as_deref().map(|c| date(&prefs, c)).unwrap_or_default());
                                                for l in assignment_tags(&dir, a) {
                                                    t.push_str(&format!(" · {}", l.label));
                                                }
                                                t
                                            },
                                            meta: a.completed.as_deref().and_then(|c| days_between(&a.assigned, c)).map(duration_label).unwrap_or_default(),
                                        })
                                        .collect::<Vec<_>>(),
                                }
                            }
                        }
                    }
                }
            }
        }
        if assigning() {
            AssignDialog { territory: id, onclose: move |_| assigning.set(false) }
        }
        if let Some(a) = returning() {
            ReturnDialog { assignment: a, onclose: move |_| returning.set(None) }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Editor
// ---------------------------------------------------------------------------------------------

/// `/app/ministry/territories/new`
#[component]
pub fn TerritoryNew() -> Element {
    rsx! { TerritoryEditor { id: None } }
}

/// `/app/ministry/territories/:id/edit`
#[component]
pub fn TerritoryEdit(id: u32) -> Element {
    rsx! { TerritoryEditor { key: "{id}", id: Some(id) } }
}

#[component]
fn TerritoryEditor(id: Option<u32>) -> Element {
    let mut state = use_app_state();
    let navigator = navigator();
    let toaster = use_toaster();
    let dir = state.directory();
    let existing = id.and_then(|n| dir.territory(n).cloned());
    let mut draft = use_signal(|| existing.as_ref().map(TerritoryDraft::from_territory).unwrap_or_default());
    let mut errors = use_signal(Vec::<(&'static str, String)>::new);
    let mut confirm_delete = use_signal(|| false);
    let mut managing = use_signal(|| None::<TagKind>);
    // Re-creates the map after an import, so it fits the new shapes.
    let mut map_key = use_signal(|| 0u32);
    let back_to = move || match id {
        Some(id) => Route::TerritoryDetail { id },
        None => Route::TerritoriesPage {},
    };
    let back = (if id.is_some() { "Territory".to_string() } else { "Territories".to_string() }, EventHandler::new(move |_| { navigator.push(back_to()); }));
    if !state.can_write(PAGE) {
        return rsx! {
            Page { title: "Territory", back,
                EmptyState { icon: Icon::Lock, title: "Only the territory servant can change territories", description: "Ask an administrator for write access to Territories." }
            }
        };
    }
    if id.is_some() && existing.is_none() {
        return rsx! {
            Page { title: "Territory not found", back,
                EmptyState { icon: Icon::Search, title: "This territory does not exist", description: "It may have been deleted." }
            }
        };
    }
    let d = draft();
    let err = move |f: &str| errors.read().iter().find(|(k, _)| *k == f).map(|(_, m)| m.clone());
    let mut save = move || {
        let d = draft();
        match state.update_directory(|dir| dir.save_territory(id, &d)) {
            Ok(saved) => {
                toaster.success(if id.is_some() { "Territory saved." } else { "Territory added." });
                navigator.replace(Route::TerritoryDetail { id: saved });
            }
            Err(e) => {
                toaster.error(e.first().map(|x| x.1.clone()).unwrap_or_default());
                errors.set(e);
            }
        }
    };
    let mut categories = vec![SelectOption::new("", "No category")];
    categories.extend(dir.territory_categories.iter().map(|c| SelectOption::new(c.id.to_string(), c.name.clone())));
    let others: Vec<MapShape> = dir
        .territories
        .iter()
        .filter(|o| Some(o.id) != id)
        .flat_map(|o| o.shapes.iter().filter(|s| s.kind == ShapeKind::Area).map(|s| MapShape { id: String::new(), kind: theo_ui::ShapeKind::Area, points: s.points.iter().map(|p| GeoPoint::new(p.lat, p.lon)).collect(), label: o.title.clone(), tone: MapTone::Muted }))
        .collect();
    let counts = (
        d.shapes.iter().filter(|s| s.kind == ShapeKind::Area).count(),
        d.shapes.iter().filter(|s| s.kind == ShapeKind::Line).count(),
        d.shapes.iter().filter(|s| s.kind == ShapeKind::Marker).count(),
    );
    let geojson_title = d.title.clone();
    let shape_counts = format!("{} · {} · {}", plural(counts.0, "area", "areas"), plural(counts.1, "line", "lines"), plural(counts.2, "marker", "markers"));
    rsx! {
        Page {
            title: existing.as_ref().map(|t| format!("Edit territory {}", t.title)).unwrap_or_else(|| "New territory".into()),
            subtitle: "The title, the area on the map, and what publishers must know",
            back,
            actions: rsx! {
                if id.is_some() {
                    ActionButton { action: Action::Delete, variant: ButtonVariant::Secondary, onclick: move |_| confirm_delete.set(true) }
                }
                ActionButton { action: Action::Cancel, variant: ButtonVariant::Secondary, onclick: move |_| { navigator.push(back_to()); } }
                ActionButton { action: Action::Save, onclick: move |_| save() }
            },
            Card { title: "Territory", icon: Icon::Map,
                Grid { cols: 2,
                    Field { label: "Title or number", required: true, error: err("title"), help: "Unique in the congregation, for example 12 or Old town 3.",
                        TextInput { value: d.title.clone(), placeholder: "For example: 12", oninput: move |v| { draft.write().title = v; errors.write().retain(|e| e.0 != "title"); } }
                    }
                    Field { label: "Category",
                        Select { value: d.category.map(|c| c.to_string()).unwrap_or_default(), options: categories, onchange: move |v: String| draft.write().category = v.parse().ok() }
                    }
                }
                Field { label: "Labels",
                    Stack { gap: Gap::Sm,
                        if dir.territory_labels.is_empty() {
                            Text { tone: TextTone::Muted, "No labels yet." }
                        } else {
                            Cluster { gap: Gap::Sm,
                                for l in dir.territory_labels.iter().cloned() {
                                    TagToggle {
                                        key: "{l.id}",
                                        label: l.name.clone(),
                                        tone: l.color.tone(),
                                        selected: d.labels.contains(&l.id),
                                        onclick: move |_| {
                                            let mut w = draft.write();
                                            if !w.labels.remove(&l.id) {
                                                w.labels.insert(l.id);
                                            }
                                        },
                                    }
                                }
                            }
                        }
                        Cluster { gap: Gap::Sm,
                            ActionButton { action: Action::ManageLabels, variant: ButtonVariant::Ghost, label: "Categories", onclick: move |_| managing.set(Some(TagKind::Category)) }
                            ActionButton { action: Action::ManageLabels, variant: ButtonVariant::Ghost, label: "Labels", onclick: move |_| managing.set(Some(TagKind::Label)) }
                        }
                    }
                }
                Field { label: "Description",
                    TextArea { value: d.description.clone(), rows: 3, placeholder: "For example: best on Saturday mornings; park at the school.", oninput: move |v| draft.write().description = v }
                }
                Field { label: "Link", error: err("url"), help: "Optional, for example the territory in another app.",
                    TextInput { kind: InputKind::Url, value: d.url.clone(), placeholder: "https://", oninput: move |v| { draft.write().url = v; errors.write().retain(|e| e.0 != "url"); } }
                }
            }
            Card { title: "Map", icon: Icon::Area,
                Text { tone: TextTone::Small,
                    "Draw the area with the area tool (several areas are fine), lines for limits like \"only this side of the street\", and markers for useful points. Other territories show in grey."
                }
{
                rsx! { MapView {
                    key: "{map_key}",
                    tiles: dir.territory_settings.tiles(),
                    shapes: to_map_shapes(&d.shapes),
                    background: others,
                    camera: d.view.map(to_camera),
                    fallback: dir.territory_settings.home.map(to_camera),
                    size: MapSize::Lg,
                    on_change: move |shapes: Vec<MapShape>| draft.write().shapes = from_map_shapes(&shapes),
                    on_save_view: move |cam: MapCamera| {
                        draft.write().view = Some(from_camera(cam));
                        toaster.success("Default view set. It is saved with the territory.");
                    },
                } }
                }
                Cluster { justify: Justify::Between,
                    Text { tone: TextTone::Small,
                        "{shape_counts} · "
                        if d.view.is_some() { "Default view: chosen" } else { "Default view: fits the shapes" }
                    }
                    Cluster { gap: Gap::Sm,
                        if d.view.is_some() {
                            ActionButton { action: Action::Undo, variant: ButtonVariant::Ghost, label: "Fit the shapes", onclick: move |_| draft.write().view = None }
                        }
                        FileButton {
                            label: "Import GeoJSON or KML",
                            icon: Icon::Upload,
                            accept: ".geojson,.json,.kml",
                            on_file: move |(_, bytes): (String, Vec<u8>)| {
                                match import_shapes(&String::from_utf8_lossy(&bytes)) {
                                    Ok(shapes) => {
                                        let n = shapes.len();
                                        draft.write().shapes.extend(shapes);
                                        map_key += 1;
                                        toaster.success(format!("{n} shapes added from the file."));
                                    }
                                    Err(e) => toaster.error(e),
                                }
                            },
                        }
                        if id.is_some() {
                            ActionButton {
                                action: Action::Export,
                                variant: ButtonVariant::Ghost,
                                label: "GeoJSON",
                                onclick: move |_| {
                                    if let Some(json) = id.and_then(|i| state.directory().territory_geojson(i)) {
                                        let file = format!("territory-{}.geojson", crate::people::fold(&geojson_title).replace(' ', "-"));
                                        match crate::platform::save_file(&file, "application/geo+json", json.as_bytes()) {
                                            Ok(m) => toaster.success(m),
                                            Err(m) => toaster.error(m),
                                        }
                                    }
                                },
                            }
                        }
                    }
                }
            }
            Card { title: "Do not call", icon: Icon::DoNotCall,
                Text { tone: TextTone::Small, "Addresses publishers must not call on. They show in red on the territory and on the card." }
                if let Some(e) = err("do_not_calls") {
                    Alert { tone: Tone::Error, "{e}" }
                }
                for (i, x) in d.do_not_calls.iter().enumerate() {
                    Grid { key: "{i}", cols: 3,
                        Field { label: "Address", required: true,
                            TextInput { value: x.address.clone(), placeholder: "Street, number, flat", oninput: move |v| if let Some(r) = draft.write().do_not_calls.get_mut(i) { r.address = v } }
                        }
                        Field { label: "Note",
                            TextInput { value: x.note.clone().unwrap_or_default(), placeholder: "Optional", oninput: move |v: String| if let Some(r) = draft.write().do_not_calls.get_mut(i) { r.note = Some(v) } }
                        }
                        Field { label: "Since",
                            Cluster { gap: Gap::Sm,
                                TextInput { kind: InputKind::Date, value: x.date.clone().unwrap_or_default(), oninput: move |v: String| if let Some(r) = draft.write().do_not_calls.get_mut(i) { r.date = Some(v) } }
                                IconButton { icon: Icon::Trash, label: "Remove this address", onclick: move |_| { draft.write().do_not_calls.remove(i); } }
                            }
                        }
                    }
                }
                Cluster {
                    ActionButton {
                        action: Action::Add,
                        variant: ButtonVariant::Secondary,
                        label: "Add an address",
                        onclick: move |_| draft.write().do_not_calls.push(DoNotCall { date: Some(today_iso()), ..Default::default() }),
                    }
                }
            }
            Cluster { justify: Justify::End,
                ActionButton { action: Action::Cancel, variant: ButtonVariant::Secondary, onclick: move |_| { navigator.push(back_to()); } }
                ActionButton { action: Action::Save, onclick: move |_| save() }
            }
        }
        if let Some(kind) = managing() {
            ManageTagsDialog { kind, onclose: move |_| managing.set(None) }
        }
        ConfirmDialog {
            open: confirm_delete(),
            title: "Delete this territory?",
            message: "Its map and its lines in the registry are deleted too.",
            confirm: Action::Delete,
            oncancel: move |_| confirm_delete.set(false),
            onconfirm: move |_| {
                if let Some(i) = id {
                    state.update_directory(|d| d.delete_territory(i));
                }
                confirm_delete.set(false);
                toaster.success("Territory deleted.");
                navigator.replace(Route::TerritoriesPage {});
            },
        }
    }
}

/// Add, rename, recolour and delete the categories (or labels) of territories.
#[component]
fn ManageTagsDialog(kind: TagKind, onclose: EventHandler<()>) -> Element {
    let kind_is_category = kind == TagKind::Category;
    let mut state = use_app_state();
    let toaster = use_toaster();
    let dir = state.directory();
    let mut editing = use_signal(|| None::<u32>);
    let mut text = use_signal(String::new);
    let mut color = use_signal(|| LabelColor::Blue.value().to_string());
    let mut error = use_signal(|| None::<String>);
    let mut confirm = use_signal(|| false);
    let tags = dir.territory_tags(kind).clone();
    let (what, whats, of) = match kind {
        TagKind::Category => ("category", "Categories", "of territories"),
        TagKind::Label => ("label", "Labels", "of territories"),
        TagKind::Assignment => ("label", "Labels", "of assignments"),
    };
    let entries: Vec<ListEntry> = tags
        .iter()
        .map(|l| {
            let used = match kind {
                TagKind::Category => format!("{} territories", dir.territories.iter().filter(|t| t.category == Some(l.id)).count()),
                TagKind::Label => format!("{} territories", dir.territories.iter().filter(|t| t.labels.contains(&l.id)).count()),
                TagKind::Assignment => format!("{} assignments", dir.territory_assignments.iter().filter(|a| a.labels.contains(&l.id)).count()),
            };
            ListEntry { icon: if kind_is_category { Icon::Map } else { Icon::Tag }, title: l.name.clone(), subtitle: format!("{} · {used}", l.color.label()), meta: String::new() }
        })
        .collect();
    let mut reset = move || {
        editing.set(None);
        text.set(String::new());
        color.set(LabelColor::Blue.value().to_string());
        error.set(None);
    };
    let save = move |_| {
        let id = editing();
        match state.update_directory(|d| d.save_territory_tag(kind, id, &text(), LabelColor::parse(&color()))) {
            Ok(_) => {
                toaster.success("Saved.");
                reset();
            }
            Err(e) => error.set(Some(e)),
        }
    };
    let color_options: Vec<(String, String, Tone)> = LabelColor::ALL.iter().map(|c| (c.value().to_string(), c.label().to_string(), c.tone())).collect();
    rsx! {
        Dialog {
            open: true,
            onclose: move |_| onclose.call(()),
            title: "{whats} {of}",
            description: match kind {
                TagKind::Category => "A territory has one category, for example Business or Rural.",
                TagKind::Label => "A territory can have several labels, for example Has an elevator or Difficult access.",
                TagKind::Assignment => "Put them on an assignment, for example when a territory is given for the Memorial campaign.",
            },
            actions: rsx! { ActionButton { action: Action::Done, onclick: move |_| onclose.call(()) } },
            Stack { gap: Gap::Lg,
                if entries.is_empty() {
                    Text { tone: TextTone::Muted, "None yet." }
                } else {
                    ItemList {
                        items: entries,
                        on_select: move |i: usize| {
                            if let Some(l) = state.directory().territory_tags(kind).get(i).cloned() {
                                editing.set(Some(l.id));
                                text.set(l.name);
                                color.set(l.color.value().to_string());
                                error.set(None);
                            }
                        },
                    }
                }
                Card { title: if editing().is_some() { format!("Change the {what}") } else { format!("New {what}") }, icon: Icon::Tag,
                    Grid { cols: 2,
                        Field { label: "Name", required: true, error: error(),
                            TextInput { value: text(), placeholder: match kind {
                                TagKind::Category => "For example: Business",
                                TagKind::Label => "For example: Has an elevator",
                                TagKind::Assignment => "For example: Memorial campaign",
                            }, oninput: move |v| { text.set(v); error.set(None); } }
                        }
                        Field { label: "Colour",
                            ColorPicker { value: color(), options: color_options, onchange: move |v| color.set(v) }
                        }
                    }
                    Cluster { gap: Gap::Sm,
                        ActionButton { action: if editing().is_some() { Action::Save } else { Action::Add }, variant: ButtonVariant::Secondary, label: if editing().is_some() { "Save" } else { "Add" }, onclick: save }
                        if editing().is_some() {
                            ActionButton { action: Action::Cancel, onclick: move |_| reset() }
                            ActionButton { action: Action::Delete, onclick: move |_| confirm.set(true) }
                        }
                    }
                }
            }
        }
        ConfirmDialog {
            open: confirm(),
            title: format!("Delete this {what}?"),
            message: if kind == TagKind::Assignment { "It is taken off every assignment that has it." } else { "It is taken off every territory that has it." },
            confirm: Action::Delete,
            oncancel: move |_| confirm.set(false),
            onconfirm: move |_| {
                if let Some(id) = editing() {
                    state.update_directory(|d| d.delete_territory_tag(kind, id));
                }
                confirm.set(false);
                reset();
                toaster.success("Deleted.");
            },
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Settings
// ---------------------------------------------------------------------------------------------

/// `/app/ministry/territories/settings`: late limit, map provider, categories and labels.
#[component]
pub fn TerritorySettingsPage() -> Element {
    let mut state = use_app_state();
    let toaster = use_toaster();
    let navigator = navigator();
    let dir = state.directory();
    let settings = dir.territory_settings.clone();
    let mut privacy = use_signal(|| false);
    let mut managing = use_signal(|| None::<TagKind>);
    let back = ("Territories".to_string(), EventHandler::new(move |_| { navigator.push(Route::TerritoriesPage {}); }));
    if !state.can_write(PAGE) {
        return rsx! {
            Page { title: "Settings", back,
                EmptyState { icon: Icon::Lock, title: "Only the territory servant can change the settings", description: "Ask an administrator for write access to Territories." }
            }
        };
    }
    let choices: Vec<Choice> = MapProvider::ALL
        .iter()
        .map(|p| Choice { value: p.value().into(), title: p.label().into(), description: p.description().into(), icon: if *p == MapProvider::EsriImagery { Icon::Globe } else { Icon::Layers } })
        .collect();
    let preview: Vec<MapShape> = dir.territories.iter().flat_map(|t| t.shapes.iter().filter(|s| s.kind == ShapeKind::Area).map(|s| MapShape { id: String::new(), kind: theo_ui::ShapeKind::Area, points: s.points.iter().map(|p| GeoPoint::new(p.lat, p.lon)).collect(), label: t.title.clone(), tone: MapTone::Primary })).collect();
    let summary = |kind: TagKind| -> String {
        let l = dir.territory_tags(kind);
        if l.is_empty() { "None yet.".into() } else { l.iter().map(|x| x.name.clone()).collect::<Vec<_>>().join(", ") }
    };
    rsx! {
        Page {
            title: "Settings",
            subtitle: "Territories: when a territory is late, the map, categories and labels",
            back,
            Card { title: "Late territories", icon: Icon::Clock,
                Field {
                    label: "Late after (days)",
                    help: format!(
                        "A territory assigned for longer than {} days (about {}) shows as late. Usually {} days ({}).",
                        settings.late_days,
                        months_text(settings.late_days),
                        crate::territories::LATE_DAYS,
                        months_text(crate::territories::LATE_DAYS)
                    ),
                    TextInput {
                        kind: InputKind::Count,
                        value: settings.late_days.to_string(),
                        oninput: move |v: String| {
                            if let Ok(n) = v.parse::<u16>() && (1..=730).contains(&n) {
                                state.update_directory(|d| d.territory_settings.late_days = n);
                            }
                        },
                    }
                }
            }
            Card { title: "Map", icon: Icon::Map,
                Switch {
                    checked: settings.maps_on,
                    label: "Show map images",
                    description: "Off: the areas are drawn on a plain background.",
                    onchange: move |on: bool| {
                        if on {
                            privacy.set(true);
                        } else {
                            state.update_directory(|d| d.territory_settings.maps_on = false);
                        }
                    },
                }
                if settings.maps_on {
                    Field { label: "Map provider", help: "Which map works best where you live. You can change it any time.",
                        ChoiceCards { value: settings.provider.value().to_string(), choices, onchange: move |v: String| state.update_directory(|d| d.territory_settings.provider = MapProvider::parse(&v)) }
                    }
                    Field { label: "Preview and starting point", help: "New territories open here. Move the map and press the target button to use this view.",
                        MapView {
                            key: "{settings.provider.value()}",
                            tiles: settings.tiles(),
                            shapes: preview,
                            camera: settings.home.map(to_camera),
                            size: MapSize::Md,
                            on_save_view: move |cam: MapCamera| {
                                state.update_directory(|d| d.territory_settings.home = Some(from_camera(cam)));
                                toaster.success("Starting point saved.");
                            },
                        }
                    }
                }
                Text { tone: TextTone::Small, "Privacy: the map provider sees which area is viewed and from which internet address. It never gets the boundaries, names or addresses." }
            }
            Grid { cols: 2,
                Card { title: "Categories", icon: Icon::Map,
                    Text { tone: TextTone::Small, "One per territory. {summary(TagKind::Category)}" }
                    ActionButton { action: Action::ManageLabels, variant: ButtonVariant::Ghost, label: "Manage categories", onclick: move |_| managing.set(Some(TagKind::Category)) }
                }
                Card { title: "Labels", icon: Icon::Tag,
                    Text { tone: TextTone::Small, "Several per territory. {summary(TagKind::Label)}" }
                    ActionButton { action: Action::ManageLabels, variant: ButtonVariant::Ghost, label: "Manage labels", onclick: move |_| managing.set(Some(TagKind::Label)) }
                }
            }
            Card { title: "Assignment labels", icon: Icon::Assignment,
                Text { tone: TextTone::Small, "Put on an assignment, for example for a campaign. {summary(TagKind::Assignment)}" }
                ActionButton { action: Action::ManageLabels, variant: ButtonVariant::Ghost, label: "Manage assignment labels", onclick: move |_| managing.set(Some(TagKind::Assignment)) }
            }
        }
        if let Some(kind) = managing() {
            ManageTagsDialog { kind, onclose: move |_| managing.set(None) }
        }
        ConfirmDialog {
            open: privacy(),
            title: "Turn on map images?",
            message: "The images come from the map provider you choose. It sees which area is viewed and from which internet address, like any website. It never gets the boundaries, names or addresses: those stay encrypted on your devices.",
            confirm: Action::Apply,
            oncancel: move |_| privacy.set(false),
            onconfirm: move |_| {
                state.update_directory(|d| d.territory_settings.maps_on = true);
                privacy.set(false);
                toaster.success("Map images are on.");
            },
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Home widgets
// ---------------------------------------------------------------------------------------------

/// "My territories": the territories I have now. Tapping one opens its page.
#[component]
pub fn MyTerritoriesWidget(
    /// How many rows fit in the widget.
    capacity: usize,
) -> Element {
    let state = use_app_state();
    let navigator = navigator();
    let prefs = state.preferences();
    let Some(me) = state.me() else {
        return rsx! { Text { tone: TextTone::Muted, "Your territories appear here when your account is linked to your person in Users." } };
    };
    let dir = state.directory();
    let today = today_iso();
    let mine: Vec<(u32, ListEntry)> = dir
        .my_territories(me)
        .into_iter()
        .map(|(a, t)| {
            let late = dir.is_late(a, &today);
            let mut sub = format!("Since {}", date(&prefs, &a.assigned));
            if !t.do_not_calls.is_empty() {
                sub.push_str(&format!(" · {} do not call", t.do_not_calls.len()));
            }
            (t.id, ListEntry { icon: if late { Icon::Warning } else { Icon::Map }, title: format!("Territory {}", t.title), subtitle: sub, meta: if late { "Late".into() } else { String::new() } })
        })
        .collect();
    if mine.is_empty() {
        return rsx! { Text { tone: TextTone::Muted, "You have no territory now." } };
    }
    let total = mine.len();
    let shown = if total > capacity { capacity.saturating_sub(1).max(1) } else { total };
    let mine: Vec<(u32, ListEntry)> = mine.into_iter().take(shown).collect();
    let ids: Vec<u32> = mine.iter().map(|m| m.0).collect();
    rsx! {
        ItemList {
            compact: true,
            items: mine.into_iter().map(|m| m.1).collect::<Vec<_>>(),
            on_select: move |i: usize| {
                if let Some(id) = ids.get(i) {
                    navigator.push(Route::TerritoryDetail { id: *id });
                }
            },
        }
        if total > shown {
            Text { tone: TextTone::Small, "and {total - shown} more: make the widget taller to see them." }
        }
    }
}

/// "Territories": how many are assigned and late.
pub fn territories_widget_values(dir: &Directory) -> (String, String, bool) {
    let s = dir.territory_stats(None);
    let delta = match s.late {
        0 => "none late".to_string(),
        1 => "1 late".to_string(),
        n => format!("{n} late"),
    };
    (format!("{} / {}", s.assigned, s.total), delta, s.late == 0)
}
