//! ⚠️ MOCK of territories (M-07): territories with a category, labels, a map (areas, lines and
//! markers), a description, do-not-call addresses and a link; assigning them to publishers and
//! taking them back (the S-13 registry); a "late" limit; map providers; statistics; GeoJSON and
//! KML. Rules: `docs/model/m07-territories.md`.

use crate::people::{parse_date, today_iso, Directory, Label, LabelColor, Person};
use std::collections::BTreeSet;

/// A place on the map (WGS 84 degrees).
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Point {
    pub lat: f64,
    pub lon: f64,
}

impl Point {
    pub fn new(lat: f64, lon: f64) -> Self {
        Point { lat, lon }
    }
}

/// What a shape of a territory map is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ShapeKind {
    /// A closed area: the boundary (or one part of it).
    Area,
    /// An open line, for example "only this side of the street".
    Line,
    /// A point of interest with a short title.
    Marker,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Shape {
    pub kind: ShapeKind,
    pub points: Vec<Point>,
    /// The title of a marker.
    pub title: Option<String>,
    /// The colour of a marker (None: blue).
    #[serde(default)]
    pub color: Option<LabelColor>,
}

/// The default view of a map.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MapView {
    pub center: Point,
    pub zoom: u8,
}

/// An address that must not be called on.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DoNotCall {
    pub address: String,
    /// Why, or details (for example "Asked on the intercom").
    pub note: Option<String>,
    /// When it was noted, `YYYY-MM-DD`.
    pub date: Option<String>,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Territory {
    pub id: u32,
    /// Unique in the congregation (for example "12" or "Old town 3").
    pub title: String,
    /// One category of the congregation (for example "Business").
    pub category: Option<u32>,
    /// Labels of the congregation (for example "Has an elevator").
    pub labels: BTreeSet<u32>,
    pub shapes: Vec<Shape>,
    /// The view the map opens with. None: it fits the shapes.
    pub view: Option<MapView>,
    pub description: Option<String>,
    pub do_not_calls: Vec<DoNotCall>,
    /// A link, for example to the territory in another app.
    pub url: Option<String>,
}

impl Territory {
    pub fn areas(&self) -> usize {
        self.shapes.iter().filter(|s| s.kind == ShapeKind::Area).count()
    }
}

/// A territory given to a publisher: one line of the registry (S-13).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TerritoryAssignment {
    pub id: u32,
    pub territory: u32,
    pub person: u32,
    /// `YYYY-MM-DD`.
    pub assigned: String,
    /// When it came back worked. None: still assigned.
    pub completed: Option<String>,
    /// Assignment labels, for example "Memorial campaign".
    #[serde(default)]
    pub labels: BTreeSet<u32>,
}

/// Where the map images come from (R-040). Only providers that need no API key.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum MapProvider {
    #[default]
    OpenStreetMap,
    OpenTopoMap,
    EsriImagery,
}

impl MapProvider {
    pub const ALL: [MapProvider; 3] = [MapProvider::OpenStreetMap, MapProvider::OpenTopoMap, MapProvider::EsriImagery];

    pub fn value(self) -> &'static str {
        match self {
            MapProvider::OpenStreetMap => "osm",
            MapProvider::OpenTopoMap => "opentopomap",
            MapProvider::EsriImagery => "esri-imagery",
        }
    }

    pub fn parse(s: &str) -> Self {
        Self::ALL.into_iter().find(|p| p.value() == s).unwrap_or_default()
    }

    pub fn label(self) -> &'static str {
        match self {
            MapProvider::OpenStreetMap => "OpenStreetMap",
            MapProvider::OpenTopoMap => "OpenTopoMap",
            MapProvider::EsriImagery => "Esri satellite",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            MapProvider::OpenStreetMap => "The standard map, with house numbers when you zoom in.",
            MapProvider::OpenTopoMap => "Contour lines and paths, for rural territories. Up to zoom 17.",
            MapProvider::EsriImagery => "Aerial photos: buildings and gardens, no street names.",
        }
    }

    /// (url template, subdomains, attribution, max zoom).
    fn template(self) -> (&'static str, &'static [&'static str], &'static str, u8) {
        match self {
            MapProvider::OpenStreetMap => ("https://tile.openstreetmap.org/{z}/{x}/{y}.png", &[], "© OpenStreetMap contributors", 19),
            MapProvider::OpenTopoMap => ("https://{s}.tile.opentopomap.org/{z}/{x}/{y}.png", &["a", "b", "c"], "© OpenStreetMap contributors, SRTM · © OpenTopoMap (CC-BY-SA)", 17),
            MapProvider::EsriImagery => ("https://server.arcgisonline.com/ArcGIS/rest/services/World_Imagery/MapServer/tile/{z}/{y}/{x}", &[], "Tiles © Esri, Maxar, Earthstar Geographics", 19),
        }
    }
}

/// Default of *late after*: 120 days (about 4 months).
pub const LATE_DAYS: u16 = 120;

/// Settings of the territories page.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct TerritorySettings {
    /// A territory assigned for longer than this many days is late. Default 120.
    pub late_days: u16,
    /// Map images on (R-045: an administrator turns them on after the privacy notice).
    pub maps_on: bool,
    pub provider: MapProvider,
    /// Where a new map opens when there is nothing to show yet.
    pub home: Option<MapView>,
}

impl Default for TerritorySettings {
    fn default() -> Self {
        TerritorySettings { late_days: LATE_DAYS, maps_on: false, provider: MapProvider::OpenStreetMap, home: None }
    }
}

impl TerritorySettings {
    /// The tiles to show, or None when maps are off.
    pub fn tiles(&self) -> Option<theo_ui::TileSource> {
        if !self.maps_on {
            return None;
        }
        let (url, subs, attribution, max_zoom) = self.provider.template();
        Some(theo_ui::TileSource {
            url: url.into(),
            subdomains: subs.iter().map(|s| s.to_string()).collect(),
            attribution: attribution.into(),
            max_zoom,
            dark: self.provider == MapProvider::EsriImagery,
        })
    }
}

/// Days as months for people: 120 → "4 months", 70 → "2.3 months", 30 → "1 month".
pub fn months_text(days: u16) -> String {
    let m = (days as f64 / 30.0 * 10.0).round() / 10.0;
    if (m - 1.0).abs() < f64::EPSILON {
        "1 month".into()
    } else if m.fract() == 0.0 {
        format!("{m:.0} months")
    } else {
        format!("{m:.1} months")
    }
}

/// The assignment labels a new congregation starts with.
pub fn default_assignment_labels() -> Vec<Label> {
    vec![
        Label { id: 1, name: "Memorial campaign".into(), color: LabelColor::Blue },
        Label { id: 2, name: "Regional convention campaign".into(), color: LabelColor::Green },
        Label { id: 3, name: "Special campaign".into(), color: LabelColor::Yellow },
    ]
}

/// The colour of a marker on the map component.
pub fn marker_tone(c: Option<LabelColor>) -> theo_ui::MapTone {
    match c {
        Some(LabelColor::Green) => theo_ui::MapTone::Success,
        Some(LabelColor::Yellow) => theo_ui::MapTone::Warning,
        Some(LabelColor::Red) => theo_ui::MapTone::Error,
        Some(LabelColor::Gray) => theo_ui::MapTone::Neutral,
        Some(LabelColor::Blue) | None => theo_ui::MapTone::Primary,
    }
}

fn marker_color(t: theo_ui::MapTone) -> Option<LabelColor> {
    match t {
        theo_ui::MapTone::Success => Some(LabelColor::Green),
        theo_ui::MapTone::Warning => Some(LabelColor::Yellow),
        theo_ui::MapTone::Error => Some(LabelColor::Red),
        theo_ui::MapTone::Neutral | theo_ui::MapTone::Muted => Some(LabelColor::Gray),
        theo_ui::MapTone::Primary | theo_ui::MapTone::Info => None,
    }
}

/// The territory form. Text as typed.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TerritoryDraft {
    pub title: String,
    pub category: Option<u32>,
    pub labels: BTreeSet<u32>,
    pub shapes: Vec<Shape>,
    pub view: Option<MapView>,
    pub description: String,
    pub do_not_calls: Vec<DoNotCall>,
    pub url: String,
}

impl TerritoryDraft {
    pub fn from_territory(t: &Territory) -> Self {
        TerritoryDraft {
            title: t.title.clone(),
            category: t.category,
            labels: t.labels.clone(),
            shapes: t.shapes.clone(),
            view: t.view,
            description: t.description.clone().unwrap_or_default(),
            do_not_calls: t.do_not_calls.clone(),
            url: t.url.clone().unwrap_or_default(),
        }
    }
}

/// Category or label of territories.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TagKind {
    Category,
    Label,
    /// Labels of assignments (for example "Memorial campaign").
    Assignment,
}

/// Numbers inside titles compare as numbers: "2" before "10".
pub fn natural_key(s: &str) -> Vec<(u8, u64, String)> {
    let mut out = Vec::new();
    let mut num = String::new();
    let mut text = String::new();
    for c in s.to_lowercase().chars() {
        if c.is_ascii_digit() {
            if !text.is_empty() {
                out.push((1, 0, std::mem::take(&mut text)));
            }
            num.push(c);
        } else {
            if !num.is_empty() {
                out.push((0, num.parse().unwrap_or(u64::MAX), String::new()));
                num.clear();
            }
            text.push(c);
        }
    }
    if !num.is_empty() {
        out.push((0, num.parse().unwrap_or(u64::MAX), String::new()));
    }
    if !text.is_empty() {
        out.push((1, 0, text));
    }
    out
}

fn day(iso: &str) -> Option<i64> {
    parse_date(iso).map(crate::prefs::days_from_ymd)
}

/// Days from `a` to `b`.
pub fn days_between(a: &str, b: &str) -> Option<i64> {
    Some(day(b)? - day(a)?)
}

/// `YYYY-MM-DD` plus `n` months (the day is kept, or the month's last day).
pub fn add_months(iso: &str, n: i32) -> Option<String> {
    let (y, m, d) = parse_date(iso)?;
    let idx = y * 12 + m as i32 - 1 + n;
    let (y2, m2) = (idx.div_euclid(12), idx.rem_euclid(12) as u32 + 1);
    let leap = (y2 % 4 == 0 && y2 % 100 != 0) || y2 % 400 == 0;
    let last = match m2 {
        2 if leap => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    Some(format!("{y2:04}-{m2:02}-{:02}", d.min(last)))
}

/// "3 months", "5 weeks", "12 days".
pub fn duration_label(days: i64) -> String {
    match days {
        d if d < 0 => "0 days".into(),
        1 => "1 day".into(),
        d if d < 21 => format!("{d} days"),
        d if d < 61 => format!("{} weeks", d / 7),
        d if d < 730 => format!("{} months", d / 30),
        d => format!("{:.1} years", d as f64 / 365.0),
    }
}

/// Statistics of the territories page.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TerritoryStats {
    pub total: usize,
    pub assigned: usize,
    pub late: usize,
    /// Completed in the period.
    pub completed: usize,
    /// Different territories completed in the period.
    pub covered: usize,
    /// Average days from assigned to completed (completed in the period).
    pub average_days: Option<i64>,
    pub never_worked: usize,
}

impl Directory {
    // ----- territories -----

    pub fn territory(&self, id: u32) -> Option<&Territory> {
        self.territories.iter().find(|t| t.id == id)
    }

    /// By title, numbers as numbers.
    pub fn territories_sorted(&self) -> Vec<&Territory> {
        let mut v: Vec<&Territory> = self.territories.iter().collect();
        v.sort_by_key(|t| natural_key(&t.title));
        v
    }

    pub fn save_territory(&mut self, id: Option<u32>, d: &TerritoryDraft) -> Result<u32, Vec<(&'static str, String)>> {
        let mut errors = Vec::new();
        let title = d.title.trim().to_string();
        if title.is_empty() {
            errors.push(("title", "Give the territory a title or number.".to_string()));
        } else if self.territories.iter().any(|t| Some(t.id) != id && t.title.trim().to_lowercase() == title.to_lowercase()) {
            errors.push(("title", format!("There is already a territory \"{title}\".")));
        }
        let url = d.url.trim().to_string();
        if !url.is_empty() && !(url.starts_with("https://") || url.starts_with("http://")) {
            errors.push(("url", "The link starts with https://".to_string()));
        }
        if d.do_not_calls.iter().any(|x| x.address.trim().is_empty()) {
            errors.push(("do_not_calls", "Every do-not-call needs an address.".to_string()));
        }
        if d.do_not_calls.iter().any(|x| x.date.as_deref().is_some_and(|v| !v.is_empty() && parse_date(v).is_none())) {
            errors.push(("do_not_calls", "A date is not valid.".to_string()));
        }
        if !errors.is_empty() {
            return Err(errors);
        }
        let opt = |s: &str| (!s.trim().is_empty()).then(|| s.trim().to_string());
        let shapes: Vec<Shape> = d
            .shapes
            .iter()
            .filter(|s| match s.kind {
                ShapeKind::Area => s.points.len() >= 3,
                ShapeKind::Line => s.points.len() >= 2,
                ShapeKind::Marker => !s.points.is_empty(),
            })
            .map(|s| Shape { kind: s.kind, points: s.points.clone(), title: s.title.as_deref().and_then(opt), color: if s.kind == ShapeKind::Marker { s.color } else { None } })
            .collect();
        let category = d.category.filter(|c| self.territory_categories.iter().any(|x| x.id == *c));
        let labels: BTreeSet<u32> = d.labels.iter().copied().filter(|l| self.territory_labels.iter().any(|x| x.id == *l)).collect();
        let dnc: Vec<DoNotCall> = d
            .do_not_calls
            .iter()
            .map(|x| DoNotCall { address: x.address.trim().to_string(), note: x.note.as_deref().and_then(opt), date: x.date.as_deref().and_then(opt) })
            .collect();
        let id = id.unwrap_or_else(|| self.territories.iter().map(|t| t.id).max().unwrap_or(0) + 1);
        let t = Territory { id, title, category, labels, shapes, view: d.view, description: opt(&d.description), do_not_calls: dnc, url: opt(&url) };
        match self.territories.iter_mut().find(|x| x.id == id) {
            Some(x) => *x = t,
            None => self.territories.push(t),
        }
        Ok(id)
    }

    /// Deletes a territory and its registry lines.
    pub fn delete_territory(&mut self, id: u32) {
        self.territories.retain(|t| t.id != id);
        self.territory_assignments.retain(|a| a.territory != id);
    }

    pub fn set_territory_view(&mut self, id: u32, view: Option<MapView>) {
        if let Some(t) = self.territories.iter_mut().find(|t| t.id == id) {
            t.view = view;
        }
    }

    // ----- categories and labels -----

    pub fn territory_tags(&self, kind: TagKind) -> &Vec<Label> {
        match kind {
            TagKind::Category => &self.territory_categories,
            TagKind::Label => &self.territory_labels,
            TagKind::Assignment => &self.assignment_labels,
        }
    }

    pub fn save_territory_tag(&mut self, kind: TagKind, id: Option<u32>, name: &str, color: LabelColor) -> Result<u32, String> {
        let name = name.trim().to_string();
        let list = match kind {
            TagKind::Category => &mut self.territory_categories,
            TagKind::Label => &mut self.territory_labels,
            TagKind::Assignment => &mut self.assignment_labels,
        };
        if name.is_empty() {
            return Err("Give it a name.".into());
        }
        if list.iter().any(|l| Some(l.id) != id && l.name.to_lowercase() == name.to_lowercase()) {
            return Err(format!("\"{name}\" already exists."));
        }
        let id = id.unwrap_or_else(|| list.iter().map(|l| l.id).max().unwrap_or(0) + 1);
        match list.iter_mut().find(|l| l.id == id) {
            Some(l) => {
                l.name = name;
                l.color = color;
            }
            None => list.push(Label { id, name, color }),
        }
        Ok(id)
    }

    /// Deletes a category or label; territories lose it.
    pub fn delete_territory_tag(&mut self, kind: TagKind, id: u32) {
        match kind {
            TagKind::Category => {
                self.territory_categories.retain(|l| l.id != id);
                for t in &mut self.territories {
                    if t.category == Some(id) {
                        t.category = None;
                    }
                }
            }
            TagKind::Label => {
                self.territory_labels.retain(|l| l.id != id);
                for t in &mut self.territories {
                    t.labels.remove(&id);
                }
            }
            TagKind::Assignment => {
                self.assignment_labels.retain(|l| l.id != id);
                for a in &mut self.territory_assignments {
                    a.labels.remove(&id);
                }
            }
        }
    }

    pub fn territory_tag(&self, kind: TagKind, id: u32) -> Option<&Label> {
        self.territory_tags(kind).iter().find(|l| l.id == id)
    }

    // ----- assignments -----

    /// Who can get a territory: everyone except Bible students and visitors, not moved away.
    pub fn territory_people(&self) -> Vec<&Person> {
        let mut v: Vec<&Person> = self.people.iter().filter(|p| p.publisher_type.is_publisher() && p.moved_away.is_none()).collect();
        v.sort_by_key(|p| (p.last_name.to_lowercase(), p.first_name.to_lowercase()));
        v
    }

    /// The open assignment of a territory (who has it now).
    pub fn current_assignment(&self, territory: u32) -> Option<&TerritoryAssignment> {
        self.territory_assignments.iter().find(|a| a.territory == territory && a.completed.is_none())
    }

    pub fn territory_assignment(&self, id: u32) -> Option<&TerritoryAssignment> {
        self.territory_assignments.iter().find(|a| a.id == id)
    }

    /// Every assignment of a territory, newest first.
    pub fn territory_history(&self, territory: u32) -> Vec<&TerritoryAssignment> {
        let mut v: Vec<&TerritoryAssignment> = self.territory_assignments.iter().filter(|a| a.territory == territory).collect();
        v.sort_by(|a, b| (b.completed.is_none(), &b.assigned, b.id).cmp(&(a.completed.is_none(), &a.assigned, a.id)));
        v
    }

    /// When the territory was last completed.
    pub fn last_completed(&self, territory: u32) -> Option<String> {
        self.territory_assignments.iter().filter(|a| a.territory == territory).filter_map(|a| a.completed.clone()).max()
    }

    /// The date an open assignment becomes late.
    pub fn late_from(&self, a: &TerritoryAssignment) -> Option<String> {
        crate::events::add_days(&a.assigned, self.territory_settings.late_days.max(1) as i64)
    }

    pub fn is_late(&self, a: &TerritoryAssignment, today: &str) -> bool {
        a.completed.is_none() && self.late_from(a).is_some_and(|d| d.as_str() <= today)
    }

    fn check_assignment(&self, id: Option<u32>, territory: u32, person: u32, assigned: &str, completed: Option<&str>) -> Result<(), String> {
        if self.territory(territory).is_none() {
            return Err("This territory does not exist any more.".into());
        }
        if !self.territory_people().iter().any(|p| p.id == person) {
            return Err("Choose a publisher. Bible students and visitors cannot get a territory.".into());
        }
        if parse_date(assigned).is_none() {
            return Err("The date it was assigned is not valid.".into());
        }
        if assigned > today_iso().as_str() {
            return Err("The date it was assigned is in the future.".into());
        }
        if let Some(c) = completed {
            if parse_date(c).is_none() {
                return Err("The date it was completed is not valid.".into());
            }
            if c < assigned {
                return Err("It cannot be completed before it was assigned.".into());
            }
            if c > today_iso().as_str() {
                return Err("The date it was completed is in the future.".into());
            }
        }
        if completed.is_none() && self.territory_assignments.iter().any(|a| Some(a.id) != id && a.territory == territory && a.completed.is_none()) {
            return Err("The territory is already assigned. Take it back first.".into());
        }
        // Assignments of one territory do not overlap.
        let end = completed.unwrap_or("9999-12-31");
        if self.territory_assignments.iter().any(|a| Some(a.id) != id && a.territory == territory && a.assigned.as_str() < end && assigned < a.completed.as_deref().unwrap_or("9999-12-31")) {
            return Err("It overlaps with another assignment of this territory in the registry.".into());
        }
        Ok(())
    }

    /// Gives a territory to a publisher.
    pub fn assign_territory(&mut self, territory: u32, person: u32, date: &str) -> Result<u32, String> {
        self.check_assignment(None, territory, person, date, None)?;
        let id = self.territory_assignments.iter().map(|a| a.id).max().unwrap_or(0) + 1;
        self.territory_assignments.push(TerritoryAssignment { id, territory, person, assigned: date.into(), completed: None, labels: BTreeSet::new() });
        Ok(id)
    }

    /// The territory came back worked.
    pub fn complete_territory(&mut self, assignment: u32, date: &str) -> Result<(), String> {
        let a = self.territory_assignment(assignment).cloned().ok_or("This assignment does not exist any more.")?;
        self.check_assignment(Some(a.id), a.territory, a.person, &a.assigned, Some(date))?;
        if let Some(x) = self.territory_assignments.iter_mut().find(|x| x.id == assignment) {
            x.completed = Some(date.into());
        }
        Ok(())
    }

    /// Corrects a line of the registry.
    pub fn update_territory_assignment(&mut self, id: u32, person: u32, assigned: &str, completed: Option<&str>) -> Result<(), String> {
        let a = self.territory_assignment(id).cloned().ok_or("This assignment does not exist any more.")?;
        self.check_assignment(Some(id), a.territory, person, assigned, completed)?;
        if let Some(x) = self.territory_assignments.iter_mut().find(|x| x.id == id) {
            x.person = person;
            x.assigned = assigned.into();
            x.completed = completed.map(Into::into);
        }
        Ok(())
    }

    /// Sets the labels of an assignment (only labels that exist).
    pub fn set_assignment_labels(&mut self, id: u32, labels: &BTreeSet<u32>) {
        let known: BTreeSet<u32> = labels.iter().copied().filter(|l| self.assignment_labels.iter().any(|x| x.id == *l)).collect();
        if let Some(a) = self.territory_assignments.iter_mut().find(|a| a.id == id) {
            a.labels = known;
        }
    }

    /// (label, assignments that started in the period), most first.
    pub fn assignments_per_label(&self, from: Option<&str>) -> Vec<(u32, usize)> {
        let mut v: Vec<(u32, usize)> = self
            .assignment_labels
            .iter()
            .map(|l| (l.id, self.territory_assignments.iter().filter(|a| a.labels.contains(&l.id) && from.is_none_or(|f| a.assigned.as_str() >= f)).count()))
            .collect();
        v.sort_by_key(|x| std::cmp::Reverse(x.1));
        v
    }

    /// Deletes a line of the registry (made by mistake).
    pub fn delete_territory_assignment(&mut self, id: u32) {
        self.territory_assignments.retain(|a| a.id != id);
    }

    /// The territories a person has now, oldest first.
    pub fn my_territories(&self, person: u32) -> Vec<(&TerritoryAssignment, &Territory)> {
        let mut v: Vec<_> = self.territory_assignments.iter().filter(|a| a.person == person && a.completed.is_none()).filter_map(|a| self.territory(a.territory).map(|t| (a, t))).collect();
        v.sort_by(|a, b| a.0.assigned.cmp(&b.0.assigned));
        v
    }

    /// A person may see a territory when it is assigned to them now.
    pub fn has_territory(&self, person: u32, territory: u32) -> bool {
        self.current_assignment(territory).is_some_and(|a| a.person == person)
    }

    // ----- statistics -----

    /// From `from` (inclusive) to today; `None` = since the start.
    pub fn territory_stats(&self, from: Option<&str>) -> TerritoryStats {
        let today = today_iso();
        let in_period = |d: &str| from.is_none_or(|f| d >= f) && d <= today.as_str();
        let done: Vec<&TerritoryAssignment> = self.territory_assignments.iter().filter(|a| a.completed.as_deref().is_some_and(in_period)).collect();
        let days: Vec<i64> = done.iter().filter_map(|a| days_between(&a.assigned, a.completed.as_deref()?)).collect();
        let covered: BTreeSet<u32> = done.iter().map(|a| a.territory).collect();
        TerritoryStats {
            total: self.territories.len(),
            assigned: self.territories.iter().filter(|t| self.current_assignment(t.id).is_some()).count(),
            late: self.territory_assignments.iter().filter(|a| self.is_late(a, &today)).count(),
            completed: done.len(),
            covered: covered.len(),
            average_days: (!days.is_empty()).then(|| days.iter().sum::<i64>() / days.len() as i64),
            never_worked: self.territories.iter().filter(|t| self.last_completed(t.id).is_none()).count(),
        }
    }

    /// (person, territories completed in the period, territories they have now), most first.
    pub fn territories_per_person(&self, from: Option<&str>) -> Vec<(u32, usize, usize)> {
        let today = today_iso();
        let mut v: Vec<(u32, usize, usize)> = self
            .territory_people()
            .iter()
            .map(|p| {
                let done = self.territory_assignments.iter().filter(|a| a.person == p.id && a.completed.as_deref().is_some_and(|d| from.is_none_or(|f| d >= f) && d <= today.as_str())).count();
                let now = self.territory_assignments.iter().filter(|a| a.person == p.id && a.completed.is_none()).count();
                (p.id, done, now)
            })
            .filter(|(_, d, n)| d + n > 0)
            .collect();
        v.sort_by_key(|x| std::cmp::Reverse(x.1 + x.2));
        v
    }

    // ----- GeoJSON and KML (R-044) -----

    /// The map of a territory as GeoJSON (areas as polygons, lines, markers as points).
    pub fn territory_geojson(&self, id: u32) -> Option<String> {
        let t = self.territory(id)?;
        let features: Vec<serde_json::Value> = t
            .shapes
            .iter()
            .map(|s| {
                let coords = |pts: &[Point]| pts.iter().map(|p| serde_json::json!([p.lon, p.lat])).collect::<Vec<_>>();
                let geometry = match s.kind {
                    ShapeKind::Area => {
                        let mut ring = coords(&s.points);
                        if let Some(f) = ring.first().cloned() {
                            ring.push(f);
                        }
                        serde_json::json!({ "type": "Polygon", "coordinates": [ring] })
                    }
                    ShapeKind::Line => serde_json::json!({ "type": "LineString", "coordinates": coords(&s.points) }),
                    ShapeKind::Marker => {
                        let p = s.points.first().copied().unwrap_or_default();
                        serde_json::json!({ "type": "Point", "coordinates": [p.lon, p.lat] })
                    }
                };
                serde_json::json!({ "type": "Feature", "properties": { "territory": t.title, "name": s.title.clone().unwrap_or_default() }, "geometry": geometry })
            })
            .collect();
        serde_json::to_string_pretty(&serde_json::json!({ "type": "FeatureCollection", "features": features })).ok()
    }
}

/// Shapes from a GeoJSON or KML file (polygons, multipolygons, lines and points; only the outer
/// ring of a polygon). Errors when nothing could be read.
pub fn import_shapes(text: &str) -> Result<Vec<Shape>, String> {
    let shapes = if text.trim_start().starts_with('{') { from_geojson(text)? } else { from_kml(text) };
    if shapes.is_empty() {
        return Err("The file has no areas, lines or points.".into());
    }
    Ok(shapes)
}

fn from_geojson(text: &str) -> Result<Vec<Shape>, String> {
    let v: serde_json::Value = serde_json::from_str(text).map_err(|_| "The file is not valid GeoJSON.".to_string())?;
    fn point(c: &serde_json::Value) -> Option<Point> {
        Some(Point::new(c.get(1)?.as_f64()?, c.get(0)?.as_f64()?))
    }
    fn ring(c: &serde_json::Value) -> Vec<Point> {
        let mut pts: Vec<Point> = c.as_array().map(|a| a.iter().filter_map(point).collect()).unwrap_or_default();
        if pts.len() > 1 && pts.first() == pts.last() {
            pts.pop();
        }
        pts
    }
    fn geometry(g: &serde_json::Value, name: Option<String>, out: &mut Vec<Shape>) {
        let c = &g["coordinates"];
        match g["type"].as_str().unwrap_or("") {
            "Polygon" => out.push(Shape { kind: ShapeKind::Area, points: ring(&c[0]), title: None, color: None }),
            "MultiPolygon" => {
                for p in c.as_array().into_iter().flatten() {
                    out.push(Shape { kind: ShapeKind::Area, points: ring(&p[0]), title: None, color: None });
                }
            }
            "LineString" => out.push(Shape { kind: ShapeKind::Line, points: ring(c), title: None, color: None }),
            "MultiLineString" => {
                for l in c.as_array().into_iter().flatten() {
                    out.push(Shape { kind: ShapeKind::Line, points: ring(l), title: None, color: None });
                }
            }
            "Point" => {
                if let Some(p) = point(c) {
                    out.push(Shape { kind: ShapeKind::Marker, points: vec![p], title: name, color: None });
                }
            }
            "GeometryCollection" => {
                for g in g["geometries"].as_array().into_iter().flatten() {
                    geometry(g, name.clone(), out);
                }
            }
            _ => {}
        }
    }
    let mut out = Vec::new();
    let features: Vec<serde_json::Value> = match v["type"].as_str() {
        Some("FeatureCollection") => v["features"].as_array().cloned().unwrap_or_default(),
        Some("Feature") => vec![v.clone()],
        _ => vec![serde_json::json!({ "geometry": v })],
    };
    for f in &features {
        let name = f["properties"]["name"].as_str().filter(|s| !s.is_empty()).map(String::from);
        geometry(&f["geometry"], name, &mut out);
    }
    out.retain(|s| match s.kind {
        ShapeKind::Area => s.points.len() >= 3,
        ShapeKind::Line => s.points.len() >= 2,
        ShapeKind::Marker => true,
    });
    Ok(out)
}

/// KML (Google My Maps, Google Earth): Placemarks with Polygon, LineString or Point.
fn from_kml(text: &str) -> Vec<Shape> {
    fn between<'a>(s: &'a str, open: &str, close: &str) -> Vec<&'a str> {
        let mut out = Vec::new();
        let mut rest = s;
        while let Some(a) = rest.find(open) {
            let after = &rest[a + open.len()..];
            let Some(start) = after.find('>') else { break };
            let body = &after[start + 1..];
            let Some(b) = body.find(close) else { break };
            out.push(&body[..b]);
            rest = &body[b + close.len()..];
        }
        out
    }
    fn coords(s: &str) -> Vec<Point> {
        let mut pts: Vec<Point> = s
            .split_whitespace()
            .filter_map(|t| {
                let mut it = t.split(',');
                let lon = it.next()?.parse().ok()?;
                let lat = it.next()?.parse().ok()?;
                Some(Point::new(lat, lon))
            })
            .collect();
        if pts.len() > 1 && pts.first() == pts.last() {
            pts.pop();
        }
        pts
    }
    let mut out = Vec::new();
    for pm in between(text, "<Placemark", "</Placemark>") {
        let name = between(pm, "<name", "</name>").first().map(|n| n.replace("<![CDATA[", "").replace("]]>", "").trim().to_string()).filter(|n| !n.is_empty());
        for poly in between(pm, "<Polygon", "</Polygon>") {
            let outer = between(poly, "<outerBoundaryIs", "</outerBoundaryIs>");
            let src = outer.first().copied().unwrap_or(poly);
            if let Some(c) = between(src, "<coordinates", "</coordinates>").first() {
                out.push(Shape { kind: ShapeKind::Area, points: coords(c), title: None, color: None });
            }
        }
        for line in between(pm, "<LineString", "</LineString>") {
            if let Some(c) = between(line, "<coordinates", "</coordinates>").first() {
                out.push(Shape { kind: ShapeKind::Line, points: coords(c), title: None, color: None });
            }
        }
        for p in between(pm, "<Point", "</Point>") {
            if let Some(c) = between(p, "<coordinates", "</coordinates>").first()
                && let Some(pt) = coords(c).first()
            {
                out.push(Shape { kind: ShapeKind::Marker, points: vec![*pt], title: name.clone(), color: None });
            }
        }
    }
    out.retain(|s| match s.kind {
        ShapeKind::Area => s.points.len() >= 3,
        ShapeKind::Line => s.points.len() >= 2,
        ShapeKind::Marker => true,
    });
    out
}

/// Shapes for the map component.
pub fn to_map_shapes(shapes: &[Shape]) -> Vec<theo_ui::MapShape> {
    shapes
        .iter()
        .map(|s| theo_ui::MapShape {
            id: String::new(),
            kind: match s.kind {
                ShapeKind::Area => theo_ui::ShapeKind::Area,
                ShapeKind::Line => theo_ui::ShapeKind::Line,
                ShapeKind::Marker => theo_ui::ShapeKind::Marker,
            },
            points: s.points.iter().map(|p| theo_ui::GeoPoint::new(p.lat, p.lon)).collect(),
            label: s.title.clone().unwrap_or_default(),
            tone: if s.kind == ShapeKind::Marker { marker_tone(s.color) } else { theo_ui::MapTone::Primary },
        })
        .collect()
}

/// Shapes from the map component.
pub fn from_map_shapes(shapes: &[theo_ui::MapShape]) -> Vec<Shape> {
    shapes
        .iter()
        .map(|s| Shape {
            kind: match s.kind {
                theo_ui::ShapeKind::Area => ShapeKind::Area,
                theo_ui::ShapeKind::Line => ShapeKind::Line,
                theo_ui::ShapeKind::Marker => ShapeKind::Marker,
            },
            points: s.points.iter().map(|p| Point::new(p.lat, p.lon)).collect(),
            title: (!s.label.trim().is_empty()).then(|| s.label.trim().to_string()),
            color: if s.kind == theo_ui::ShapeKind::Marker { marker_color(s.tone) } else { None },
        })
        .collect()
}

pub fn to_camera(v: MapView) -> theo_ui::MapCamera {
    theo_ui::MapCamera { center: theo_ui::GeoPoint::new(v.center.lat, v.center.lon), zoom: v.zoom }
}

pub fn from_camera(c: theo_ui::MapCamera) -> MapView {
    MapView { center: Point::new(c.center.lat, c.center.lon), zoom: c.zoom }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> Directory {
        let mut d = crate::mock::demo_directory(36);
        d.territories.clear();
        d.territory_assignments.clear();
        let square = vec![Shape { kind: ShapeKind::Area, points: vec![Point::new(0.0, 0.0), Point::new(0.0, 1.0), Point::new(1.0, 1.0)], title: None, color: None }];
        for t in ["1", "2", "10"] {
            let _ = d.save_territory(None, &TerritoryDraft { title: t.into(), shapes: square.clone(), ..Default::default() });
        }
        d
    }

    #[test]
    fn titles_are_unique_and_sorted_as_numbers() {
        let mut d = setup();
        assert!(d.save_territory(None, &TerritoryDraft { title: " 10 ".into(), ..Default::default() }).is_err());
        let order: Vec<&str> = d.territories_sorted().iter().map(|t| t.title.as_str()).collect();
        assert_eq!(order, ["1", "2", "10"]);
    }

    #[test]
    fn one_publisher_at_a_time_and_no_students() {
        let mut d = setup();
        let student = d.people.iter().find(|p| !p.publisher_type.is_publisher()).map(|p| p.id).unwrap_or(0);
        assert!(d.assign_territory(1, student, "2026-01-10").is_err());
        let a = d.assign_territory(1, 1, "2026-01-10").unwrap_or(0);
        assert!(d.assign_territory(1, 2, "2026-01-12").is_err(), "already assigned");
        assert!(d.complete_territory(a, "2026-01-01").is_err(), "before it was assigned");
        assert!(d.complete_territory(a, "2026-03-01").is_ok());
        assert!(d.assign_territory(1, 2, "2026-03-02").is_ok());
        assert_eq!(d.territory_history(1).len(), 2);
        assert_eq!(d.territory_history(1)[0].person, 2, "newest first");
    }

    #[test]
    fn late_after_the_limit() {
        let mut d = setup();
        let today = today_iso();
        let old = add_months(&today, -5).unwrap_or_default();
        let a = d.assign_territory(2, 1, &old).unwrap_or(0);
        let x = d.territory_assignment(a).cloned();
        assert!(x.as_ref().is_some_and(|x| d.is_late(x, &today)));
        d.territory_settings.late_days = 180;
        assert!(x.as_ref().is_some_and(|x| !d.is_late(x, &today)));
    }

    #[test]
    fn late_days_read_as_months() {
        assert_eq!(months_text(120), "4 months");
        assert_eq!(months_text(70), "2.3 months");
        assert_eq!(months_text(30), "1 month");
    }

    #[test]
    fn add_months_keeps_the_month_end() {
        assert_eq!(add_months("2026-01-31", 1).as_deref(), Some("2026-02-28"));
        assert_eq!(add_months("2026-11-15", 4).as_deref(), Some("2027-03-15"));
    }

    #[test]
    fn geojson_round_trip_and_kml() {
        let d = setup();
        let json = d.territory_geojson(1).unwrap_or_default();
        let shapes = import_shapes(&json).unwrap_or_default();
        assert_eq!(shapes.len(), 1);
        assert_eq!(shapes[0].points.len(), 3);
        let kml = "<kml><Placemark><name>Gate</name><Point><coordinates>-117.3,33.9,0</coordinates></Point></Placemark><Placemark><Polygon><outerBoundaryIs><LinearRing><coordinates>1,2,0 3,4,0 5,6,0 1,2,0</coordinates></LinearRing></outerBoundaryIs></Polygon></Placemark></kml>";
        let s = import_shapes(kml).unwrap_or_default();
        assert_eq!(s.len(), 2);
        assert_eq!(s[0].title.as_deref(), Some("Gate"));
        assert_eq!(s[1].kind, ShapeKind::Area);
        assert_eq!(s[1].points.len(), 3);
    }
}
