//! Maps (REQUIREMENTS.md 1.7, R-046): a raster tile grid, an SVG layer for areas, lines and
//! markers, pan and zoom with mouse, wheel and touch, and drawing and editing. Plain Rust: no JS
//! map library. The shapes never go to the tile provider: only tile addresses are requested.

use crate::icon::{Icon, IconSize, IconView};
use crate::overlay::Dialog;
use crate::action::{Action, ActionButton};
use crate::feedback::Tone;
use crate::form::{ColorPicker, Field, TextInput};
use dioxus::html::input_data::MouseButton;
use dioxus::prelude::*;
use std::f64::consts::PI;

const TILE: f64 = 256.0;
const MIN_ZOOM: u8 = 2;
/// How close (in pixels) a tap must be to a point to hit it.
const HIT: f64 = 14.0;

/// A place on the earth (WGS 84, degrees).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GeoPoint {
    /// Latitude, -90 to 90.
    pub lat: f64,
    /// Longitude, -180 to 180.
    pub lon: f64,
}

impl GeoPoint {
    /// A point from latitude and longitude.
    pub fn new(lat: f64, lon: f64) -> Self {
        GeoPoint { lat, lon }
    }
}

/// What the map shows: its centre and zoom level (2 = the world, 19 = a house).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MapCamera {
    /// The centre of the view.
    pub center: GeoPoint,
    /// Zoom level.
    pub zoom: u8,
}

/// Where the map images come from (a tile provider, R-040).
#[derive(Clone, Debug, PartialEq)]
pub struct TileSource {
    /// Address with `{z}`, `{x}`, `{y}` and optionally `{s}` (a subdomain).
    pub url: String,
    /// Subdomains for `{s}`, used in turn.
    pub subdomains: Vec<String>,
    /// Shown on the map, as the provider asks (for example "© OpenStreetMap contributors").
    pub attribution: String,
    /// The highest zoom the provider has.
    pub max_zoom: u8,
    /// The images are dark (for example aerial photos): labels get a light background.
    pub dark: bool,
}

impl TileSource {
    /// The address of one tile.
    pub fn tile_url(&self, z: u8, x: i64, y: i64) -> String {
        let s = if self.subdomains.is_empty() { String::new() } else { self.subdomains[((x + y).unsigned_abs() as usize) % self.subdomains.len()].clone() };
        self.url.replace("{z}", &z.to_string()).replace("{x}", &x.to_string()).replace("{y}", &y.to_string()).replace("{s}", &s)
    }
}

/// The colour of a shape.
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MapTone {
    #[default]
    Primary,
    Muted,
    Info,
    Success,
    Warning,
    Error,
    /// Dark grey (the same in both themes).
    Neutral,
}

impl MapTone {
    /// The colours a marker can have, with their names.
    pub const MARKER: [(MapTone, &'static str); 5] = [
        (MapTone::Primary, "Blue"),
        (MapTone::Success, "Green"),
        (MapTone::Warning, "Yellow"),
        (MapTone::Error, "Red"),
        (MapTone::Neutral, "Grey"),
    ];

    fn value(self) -> &'static str {
        match self {
            MapTone::Primary => "primary",
            MapTone::Muted => "muted",
            MapTone::Info => "info",
            MapTone::Success => "success",
            MapTone::Warning => "warning",
            MapTone::Error => "error",
            MapTone::Neutral => "neutral",
        }
    }

    fn picker_tone(self) -> Tone {
        match self {
            MapTone::Success => Tone::Success,
            MapTone::Warning => Tone::Warning,
            MapTone::Error => Tone::Error,
            MapTone::Neutral | MapTone::Muted => Tone::Neutral,
            MapTone::Primary | MapTone::Info => Tone::Info,
        }
    }

    /// Fixed colours for the greys (map images are light in both themes).
    fn style(self) -> &'static str {
        match self {
            MapTone::Muted => "stroke: #6b7280; fill: #6b7280;",
            MapTone::Neutral => "stroke: #4b5563; fill: #4b5563;",
            _ => "",
        }
    }

    fn stroke(self) -> &'static str {
        match self {
            MapTone::Primary => "stroke-primary fill-primary",
            MapTone::Muted | MapTone::Neutral => "",
            MapTone::Info => "stroke-info fill-info",
            MapTone::Success => "stroke-success fill-success",
            MapTone::Warning => "stroke-warning fill-warning",
            MapTone::Error => "stroke-error fill-error",
        }
    }
}

/// What a shape is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShapeKind {
    /// A closed area (a boundary).
    Area,
    /// An open line (for example "only this side of the street").
    Line,
    /// A point with a short title.
    Marker,
}

/// One area, line or marker on the map.
#[derive(Clone, Debug, PartialEq)]
pub struct MapShape {
    /// Given back by `on_select` (for example the territory). May be empty.
    pub id: String,
    /// Area, line or marker.
    pub kind: ShapeKind,
    /// The corners (a marker has one).
    pub points: Vec<GeoPoint>,
    /// The title of a marker, or the name written in the middle of an area.
    pub label: String,
    /// Its colour.
    pub tone: MapTone,
}

/// How tall the map is.
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MapSize {
    Sm,
    #[default]
    Md,
    Lg,
}

/// The position of a point in world pixels at a zoom level (Web Mercator).
pub fn project(p: GeoPoint, zoom: u8) -> (f64, f64) {
    let n = TILE * 2f64.powi(zoom as i32);
    let lat = p.lat.clamp(-85.0511, 85.0511).to_radians();
    ((p.lon + 180.0) / 360.0 * n, (1.0 - (lat.tan() + 1.0 / lat.cos()).ln() / PI) / 2.0 * n)
}

/// The point at world pixels at a zoom level.
pub fn unproject((x, y): (f64, f64), zoom: u8) -> GeoPoint {
    let n = TILE * 2f64.powi(zoom as i32);
    GeoPoint { lat: (PI * (1.0 - 2.0 * y / n)).sinh().atan().to_degrees(), lon: x / n * 360.0 - 180.0 }
}

/// The tiles of a `width` × `height` view: (address, left, top) in pixels.
pub fn tile_grid(source: &TileSource, cam: MapCamera, width: f64, height: f64) -> Vec<(String, f64, f64)> {
    let c = project(cam.center, cam.zoom);
    let n = 1i64 << cam.zoom;
    let (x0, x1) = (((c.0 - width / 2.0) / TILE).floor() as i64, ((c.0 + width / 2.0) / TILE).floor() as i64);
    let (y0, y1) = ((((c.1 - height / 2.0) / TILE).floor() as i64).max(0), (((c.1 + height / 2.0) / TILE).floor() as i64).min(n - 1));
    let mut v = Vec::new();
    for ty in y0..=y1 {
        for tx in x0..=x1 {
            v.push((source.tile_url(cam.zoom, tx.rem_euclid(n), ty), tx as f64 * TILE - c.0 + width / 2.0, ty as f64 * TILE - c.1 + height / 2.0));
        }
    }
    v
}

/// Where a point is in a `width` × `height` view, in pixels from the top left.
pub fn to_pixels(cam: MapCamera, width: f64, height: f64, p: GeoPoint) -> (f64, f64) {
    let c = project(cam.center, cam.zoom);
    let (x, y) = project(p, cam.zoom);
    (x - c.0 + width / 2.0, y - c.1 + height / 2.0)
}

/// The view that shows all `points` in a `width` × `height` map, with a margin.
pub fn fit_camera(points: &[GeoPoint], width: f64, height: f64, max_zoom: u8) -> Option<MapCamera> {
    let first = points.first()?;
    let (mut lo, mut hi) = (*first, *first);
    for p in points {
        lo = GeoPoint::new(lo.lat.min(p.lat), lo.lon.min(p.lon));
        hi = GeoPoint::new(hi.lat.max(p.lat), hi.lon.max(p.lon));
    }
    let center = GeoPoint::new((lo.lat + hi.lat) / 2.0, (lo.lon + hi.lon) / 2.0);
    // A smaller margin on small maps (phones), so the shapes stay as big as possible.
    let margin = if width.min(height) < 420.0 { 20.0 } else { 48.0 };
    let (w, h) = ((width - margin).max(64.0), (height - margin).max(64.0));
    let mut zoom = max_zoom.min(18);
    while zoom > MIN_ZOOM {
        let (x0, y0) = project(GeoPoint::new(hi.lat, lo.lon), zoom);
        let (x1, y1) = project(GeoPoint::new(lo.lat, hi.lon), zoom);
        if x1 - x0 <= w && y1 - y0 <= h {
            break;
        }
        zoom -= 1;
    }
    Some(MapCamera { center, zoom })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Tool {
    Pan,
    Area,
    Line,
    Marker,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Target {
    Pan,
    /// A corner of a shape (shape, corner).
    Vertex(usize, usize),
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Drag {
    start: (f64, f64),
    last: (f64, f64),
    moved: bool,
    target: Target,
}

fn dist(a: (f64, f64), b: (f64, f64)) -> f64 {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
}

fn dist_to_segment(p: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len = dx * dx + dy * dy;
    if len == 0.0 {
        return dist(p, a);
    }
    let t = (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / len).clamp(0.0, 1.0);
    dist(p, (a.0 + t * dx, a.1 + t * dy))
}

fn inside(p: (f64, f64), poly: &[(f64, f64)]) -> bool {
    let mut c = false;
    let n = poly.len();
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + n - 1) % n]);
        if (a.1 > p.1) != (b.1 > p.1) && p.0 < (b.0 - a.0) * (p.1 - a.1) / (b.1 - a.1) + a.0 {
            c = !c;
        }
    }
    c
}

fn centroid(pts: &[(f64, f64)]) -> (f64, f64) {
    let n = pts.len().max(1) as f64;
    (pts.iter().map(|p| p.0).sum::<f64>() / n, pts.iter().map(|p| p.1).sum::<f64>() / n)
}

/// Width of the pill behind a label (12 px semibold text), in pixels.
fn label_width(s: &str) -> f64 {
    s.chars().map(|c| if c.is_ascii_digit() || c == ' ' { 7.0 } else if c.is_uppercase() { 8.0 } else { 6.6 }).sum::<f64>() + 14.0
}

fn points_attr(pts: &[(f64, f64)]) -> String {
    pts.iter().map(|(x, y)| format!("{x:.1},{y:.1}")).collect::<Vec<_>>().join(" ")
}

/// A map with tiles and shapes. Drag to move, wheel, pinch or the buttons to zoom.
///
/// With `on_change` it is an editor: draw areas and lines, add markers with a title, select a
/// shape to move its corners (drag a small middle dot to add a corner) or delete it.
#[component]
pub fn MapView(
    /// Map images; `None` when maps are off (the shapes show on a plain background).
    tiles: Option<TileSource>,
    /// The shapes (editable when `on_change` is set).
    shapes: Vec<MapShape>,
    /// Shapes in the background, for orientation (for example the other territories).
    #[props(default)]
    background: Vec<MapShape>,
    /// The first view. Without it the map fits the shapes, then the background, then `fallback`.
    #[props(default)]
    camera: Option<MapCamera>,
    /// The view when there is nothing to show.
    #[props(default)]
    fallback: Option<MapCamera>,
    #[props(default)] size: MapSize,
    /// Makes the map an editor: called with every change of the shapes.
    #[props(default)]
    on_change: Option<EventHandler<Vec<MapShape>>>,
    /// Called with the `id` of a shape that is tapped (not in the editor).
    #[props(default)]
    on_select: Option<EventHandler<String>>,
    /// Shows a button that saves the current view (for example as the default view).
    #[props(default)]
    on_save_view: Option<EventHandler<MapCamera>>,
) -> Element {
    let mut box_size = use_signal(|| (0.0f64, 0.0f64));
    let mut cam = use_signal(|| None::<MapCamera>);
    let mut tool = use_signal(|| Tool::Pan);
    let mut draft = use_signal(Vec::<GeoPoint>::new);
    let mut hover = use_signal(|| None::<(f64, f64)>);
    let mut selected = use_signal(|| None::<usize>);
    let mut drag = use_signal(|| None::<Drag>);
    // A corner being moved: (shape, corner, where).
    let mut moving = use_signal(|| None::<(usize, usize, GeoPoint)>);
    let mut pointers = use_signal(Vec::<(i32, (f64, f64))>::new);
    let mut pinch = use_signal(|| None::<f64>);
    let mut wheel = use_signal(|| 0.0f64);
    // A marker being named: (where, the marker it changes).
    let mut naming = use_signal(|| None::<(GeoPoint, Option<usize>)>);
    let mut name = use_signal(String::new);
    let mut marker_tone = use_signal(|| MapTone::Primary);
    // A corner of the selected shape that was tapped (it can then be deleted).
    let mut corner = use_signal(|| None::<usize>);
    let mut full = use_signal(|| false);

    let editable = on_change.is_some();
    let max_zoom = tiles.as_ref().map(|t| t.max_zoom).unwrap_or(19).clamp(MIN_ZOOM, 22);
    let (w, h) = box_size();
    let (mw, mh) = if w > 0.0 { (w, h) } else { (640.0, 360.0) };
    let all_points = |list: &[MapShape]| list.iter().flat_map(|s| s.points.iter().copied()).collect::<Vec<_>>();
    let fitted = {
        let own = all_points(&shapes);
        let pts = if own.is_empty() { all_points(&background) } else { own };
        fit_camera(&pts, mw, mh, max_zoom)
    };
    // A saved view that is too close for this map (for example made on a big screen and shown on
    // a phone) gives way to the view that fits the shapes.
    let has_own = shapes.iter().any(|s| !s.points.is_empty());
    let first_view = match (camera, fitted) {
        (Some(c), Some(f)) if has_own && c.zoom > f.zoom => Some(f),
        (c, _) => c,
    };
    let view = cam().or(first_view).or(fitted).or(fallback).unwrap_or(MapCamera { center: GeoPoint::new(20.0, 0.0), zoom: MIN_ZOOM });
    let z = view.zoom;
    let c = project(view.center, z);
    let to_screen = move |p: GeoPoint| {
        let (x, y) = project(p, z);
        (x - c.0 + mw / 2.0, y - c.1 + mh / 2.0)
    };
    let to_geo = move |(sx, sy): (f64, f64)| unproject((sx + c.0 - mw / 2.0, sy + c.1 - mh / 2.0), z);

    // The shapes as drawn, with a corner being moved.
    let shown: Vec<MapShape> = shapes
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let mut s = s.clone();
            if let Some((si, k, p)) = moving()
                && si == i
                && let Some(pt) = s.points.get_mut(k)
            {
                *pt = p;
            }
            s
        })
        .collect();

    // ----- camera changes -----
    let mut zoom_at = move |anchor: (f64, f64), to: u8| {
        let to = to.clamp(MIN_ZOOM, max_zoom);
        if to == z {
            return;
        }
        let g = to_geo(anchor);
        let (gx, gy) = project(g, to);
        let center = unproject((gx - (anchor.0 - mw / 2.0), gy - (anchor.1 - mh / 2.0)), to);
        cam.set(Some(MapCamera { center, zoom: to }));
    };
    let mut pan_by = move |dx: f64, dy: f64| {
        let center = unproject((c.0 - dx, c.1 - dy), z);
        cam.set(Some(MapCamera { center: GeoPoint::new(center.lat.clamp(-85.0, 85.0), center.lon), zoom: z }));
    };

    // ----- hit testing (screen pixels) -----
    let shapes_for_hit = shown.clone();
    let hit_shape = move |p: (f64, f64)| -> Option<usize> {
        // Markers first, then lines, then areas (smallest last drawn on top).
        let order = [ShapeKind::Marker, ShapeKind::Line, ShapeKind::Area];
        for kind in order {
            for (i, s) in shapes_for_hit.iter().enumerate().rev().filter(|(_, s)| s.kind == kind) {
                let pts: Vec<(f64, f64)> = s.points.iter().map(|g| to_screen(*g)).collect();
                let hit = match kind {
                    ShapeKind::Marker => pts.first().is_some_and(|m| dist(*m, p) < HIT),
                    ShapeKind::Line => pts.windows(2).any(|w| dist_to_segment(p, w[0], w[1]) < HIT / 2.0 + 2.0),
                    ShapeKind::Area => pts.len() > 2 && (inside(p, &pts) || (0..pts.len()).any(|k| dist_to_segment(p, pts[k], pts[(k + 1) % pts.len()]) < HIT / 2.0)),
                };
                if hit {
                    return Some(i);
                }
            }
        }
        None
    };
    let hit_shape_right = hit_shape.clone();
    let shapes_for_vertex = shown.clone();
    // A corner of the selected shape, or a middle dot (which adds a corner): (corner, is new).
    let hit_vertex = move |p: (f64, f64)| -> Option<(usize, usize, bool)> {
        let i = selected()?;
        let s = shapes_for_vertex.get(i)?;
        let pts: Vec<(f64, f64)> = s.points.iter().map(|g| to_screen(*g)).collect();
        if let Some(k) = pts.iter().position(|q| dist(*q, p) < HIT) {
            return Some((i, k, false));
        }
        if s.kind == ShapeKind::Marker {
            return None;
        }
        let n = pts.len();
        let segments = if s.kind == ShapeKind::Area { n } else { n.saturating_sub(1) };
        (0..segments).find(|k| dist(((pts[*k].0 + pts[(k + 1) % n].0) / 2.0, (pts[*k].1 + pts[(k + 1) % n].1) / 2.0), p) < HIT).map(|k| (i, k + 1, true))
    };

    let hit_vertex_right = hit_vertex.clone();

    // ----- editing -----
    let base = shapes.clone();
    let commit = move |list: Vec<MapShape>| {
        if let Some(h) = &on_change {
            h.call(list);
        }
    };
    // Opens the marker dialog a moment later: on touch screens the tap's click would otherwise
    // land on the new dialog's backdrop and close it at once.
    let open_marker = move |at: GeoPoint, index: Option<usize>, label: String, tone: MapTone| {
        spawn(async move {
            crate::sleep(std::time::Duration::from_millis(300)).await;
            name.set(label);
            marker_tone.set(tone);
            naming.set(Some((at, index)));
        });
    };
    // Deletes a corner, if the shape keeps enough (3 for an area, 2 for a line).
    let remove_corner = {
        let base = base.clone();
        move |i: usize, k: usize| -> bool {
            let mut list = base.clone();
            let Some(s) = list.get_mut(i) else { return false };
            let min = if s.kind == ShapeKind::Area { 3 } else { 2 };
            if s.kind == ShapeKind::Marker || s.points.len() <= min || k >= s.points.len() {
                return false;
            }
            s.points.remove(k);
            corner.set(None);
            commit(list);
            true
        }
    };
    let finish = {
        let base = base.clone();
        move || {
            let pts = draft();
            let kind = if tool() == Tool::Area { ShapeKind::Area } else { ShapeKind::Line };
            let enough = if kind == ShapeKind::Area { pts.len() >= 3 } else { pts.len() >= 2 };
            if enough {
                let mut list = base.clone();
                list.push(MapShape { id: String::new(), kind, points: pts, label: String::new(), tone: MapTone::Primary });
                selected.set(Some(list.len() - 1));
                commit(list);
            }
            draft.set(Vec::new());
            tool.set(Tool::Pan);
        }
    };

    let on_click = {
        let base = base.clone();
        let mut finish = finish.clone();
        let shapes = shapes.clone();
        move |p: (f64, f64)| {
            if !editable {
                if let (Some(h), Some(i)) = (&on_select, hit_shape(p))
                    && let Some(s) = shapes.get(i)
                    && !s.id.is_empty()
                {
                    h.call(s.id.clone());
                }
                return;
            }
            match tool() {
                Tool::Area | Tool::Line => {
                    let pts = draft();
                    if tool() == Tool::Area && pts.len() >= 3 && pts.first().is_some_and(|f| dist(to_screen(*f), p) < HIT) {
                        finish();
                    } else {
                        draft.write().push(to_geo(p));
                    }
                }
                Tool::Marker => open_marker(to_geo(p), None, String::new(), MapTone::Primary),
                Tool::Pan => {
                    let hit = hit_shape(p);
                    corner.set(None);
                    selected.set(hit);
                    // Tapping a marker opens it: title, colour, delete.
                    if let Some(i) = hit
                        && let Some(s) = base.get(i).filter(|s| s.kind == ShapeKind::Marker)
                    {
                        open_marker(s.points.first().copied().unwrap_or_default(), Some(i), s.label.clone(), s.tone);
                    }
                }
            }
        }
    };

    let base_down = base.clone();
    let on_down = move |e: PointerEvent| {
        // The right button deletes a corner (see oncontextmenu); it does not drag.
        if e.trigger_button() == Some(MouseButton::Secondary) {
            return;
        }
        let p = e.element_coordinates();
        let p = (p.x, p.y);
        pointers.write().push((e.pointer_id(), p));
        if pointers.read().len() == 2 {
            let ps = pointers();
            pinch.set(Some(dist(ps[0].1, ps[1].1)));
            drag.set(None);
            moving.set(None);
            return;
        }
        let target = if editable && tool() == Tool::Pan {
            match hit_vertex(p) {
                Some((i, k, _)) => Target::Vertex(i, k),
                None => Target::Pan,
            }
        } else {
            Target::Pan
        };
        // Insert the new corner from a middle dot when the drag starts.
        if let Target::Vertex(i, k) = target
            && let Some((_, _, true)) = hit_vertex(p)
        {
            let mut list = base_down.clone();
            if let Some(s) = list.get_mut(i) {
                s.points.insert(k, to_geo(p));
                commit(list);
            }
        }
        drag.set(Some(Drag { start: p, last: p, moved: false, target }));
    };

    let on_move = move |e: PointerEvent| {
        let p = e.element_coordinates();
        let p = (p.x, p.y);
        let id = e.pointer_id();
        if let Some(entry) = pointers.write().iter_mut().find(|(i, _)| *i == id) {
            entry.1 = p;
        }
        if let Some(base_d) = pinch() {
            let ps = pointers();
            if ps.len() == 2 {
                let d = dist(ps[0].1, ps[1].1);
                let mid = ((ps[0].1 .0 + ps[1].1 .0) / 2.0, (ps[0].1 .1 + ps[1].1 .1) / 2.0);
                if d > base_d * 1.5 {
                    zoom_at(mid, z + 1);
                    pinch.set(Some(d));
                } else if d < base_d / 1.5 {
                    zoom_at(mid, z.saturating_sub(1));
                    pinch.set(Some(d));
                }
            }
            return;
        }
        hover.set(Some(p));
        let Some(mut d) = drag() else { return };
        if !d.moved && dist(d.start, p) > 4.0 {
            d.moved = true;
        }
        if d.moved {
            match d.target {
                Target::Pan => pan_by(p.0 - d.last.0, p.1 - d.last.1),
                Target::Vertex(i, k) => moving.set(Some((i, k, to_geo(p)))),
            }
        }
        d.last = p;
        drag.set(Some(d));
    };

    let mut end = {
        let base = base.clone();
        let mut on_click = on_click.clone();
        move |id: i32, at: Option<(f64, f64)>| {
            pointers.write().retain(|(i, _)| *i != id);
            if pinch().is_some() {
                if pointers.read().len() < 2 {
                    pinch.set(None);
                }
                return;
            }
            let Some(d) = drag() else { return };
            drag.set(None);
            if let Some((i, k, p)) = moving() {
                moving.set(None);
                let mut list = base.clone();
                if let Some(pt) = list.get_mut(i).and_then(|s| s.points.get_mut(k)) {
                    *pt = p;
                }
                commit(list);
            } else if !d.moved
                && let Some(p) = at
            {
                // A tapped corner is selected (then it can be deleted); a marker opens.
                match d.target {
                    Target::Vertex(i, k) if base.get(i).is_some_and(|s| s.kind != ShapeKind::Marker) => corner.set(Some(k)),
                    _ => on_click(p),
                }
            }
        }
    };
    let mut end2 = end.clone();
    let mut end3 = end.clone();

    // ----- drawing -----
    let tiles_shown: Vec<(String, f64, f64)> = match &tiles {
        Some(t) if w > 0.0 => tile_grid(t, view, mw, mh),
        _ => Vec::new(),
    };
    let height = match size {
        MapSize::Sm => "h-56",
        MapSize::Md => "h-80 sm:h-96",
        MapSize::Lg => "h-[65vh] min-h-96",
    };
    let cursor = match (editable, tool()) {
        (true, Tool::Area | Tool::Line | Tool::Marker) => "cursor-crosshair",
        _ => "cursor-grab active:cursor-grabbing",
    };
    // Labels stand out from the map: a dark background on light maps, a light one on dark
    // images (aerial photos); without map images they follow the theme.
    // (pill class, pill style, text class, text style)
    let pill: (&str, &str, &str, &str) = match &tiles {
        Some(t) if t.dark => ("", "fill: #ffffff; fill-opacity: 0.82;", "", "fill: #111827;"),
        Some(_) => ("", "fill: #111827; fill-opacity: 0.72;", "", "fill: #ffffff;"),
        None => ("fill-base-content/75", "", "fill-base-100", ""),
    };
    let draw = |s: &MapShape, muted: bool, sel: bool| -> Element {
        let pts: Vec<(f64, f64)> = s.points.iter().map(|g| to_screen(*g)).collect();
        let tone = if muted { MapTone::Muted } else { s.tone };
        let width = if sel { "3.5" } else if muted { "1.5" } else { "2.5" };
        match s.kind {
            ShapeKind::Area => {
                let (cx, cy) = centroid(&pts);
                rsx! {
                    polygon { class: tone.stroke(), style: tone.style(), points: points_attr(&pts), "fill-opacity": if muted { "0.08" } else { "0.16" }, "stroke-width": width, "stroke-linejoin": "round", "stroke-dasharray": if muted { "5 4" } else { "" } }
                    if !s.label.is_empty() {
                        rect { x: "{cx - label_width(&s.label) / 2.0:.1}", y: "{cy - 10.0:.1}", width: "{label_width(&s.label):.1}", height: "20", rx: "6", class: pill.0, style: pill.1 }
                        text { class: "text-xs font-semibold {pill.2}", style: pill.3, x: "{cx:.1}", y: "{cy:.1}", "text-anchor": "middle", "dominant-baseline": "central", "{s.label}" }
                    }
                }
            }
            ShapeKind::Line => rsx! {
                polyline { class: tone.stroke(), style: tone.style(), points: points_attr(&pts), fill: "none", "stroke-width": if sel { "5" } else { "4" }, "stroke-linecap": "round", "stroke-linejoin": "round", "stroke-dasharray": "8 6" }
            },
            ShapeKind::Marker => {
                let (x, y) = pts.first().copied().unwrap_or_default();
                rsx! {
                    circle { class: tone.stroke(), style: "{tone.style()} stroke: #ffffff;", cx: "{x:.1}", cy: "{y:.1}", r: if sel { "9" } else { "7" }, "stroke-width": "2.5" }
                    if !s.label.is_empty() {
                        rect { x: "{x + 10.0:.1}", y: "{y - 10.0:.1}", width: "{label_width(&s.label):.1}", height: "20", rx: "6", class: pill.0, style: pill.1 }
                        text { class: "text-xs font-semibold {pill.2}", style: pill.3, x: "{x + 10.0 + label_width(&s.label) / 2.0:.1}", y: "{y:.1}", "text-anchor": "middle", "dominant-baseline": "central", "{s.label}" }
                    }
                }
            }
        }
    };
    let sel = selected().filter(|i| *i < shown.len());
    let picked = corner();
    let handles: Vec<(f64, f64, bool)> = match sel.and_then(|i| shown.get(i)) {
        Some(s) if editable && s.kind != ShapeKind::Marker => {
            let pts: Vec<(f64, f64)> = s.points.iter().map(|g| to_screen(*g)).collect();
            let n = pts.len();
            let segments = if s.kind == ShapeKind::Area { n } else { n.saturating_sub(1) };
            let mut v: Vec<(f64, f64, bool)> = pts.iter().map(|p| (p.0, p.1, false)).collect();
            v.extend((0..segments).map(|k| ((pts[k].0 + pts[(k + 1) % n].0) / 2.0, (pts[k].1 + pts[(k + 1) % n].1) / 2.0, true)));
            v
        }
        _ => Vec::new(),
    };
    let draft_pts: Vec<(f64, f64)> = draft().iter().map(|g| to_screen(*g)).collect();
    let drafting = !draft_pts.is_empty();
    let preview: Vec<(f64, f64)> = match (drafting, hover()) {
        (true, Some(hp)) => draft_pts.iter().copied().chain(std::iter::once(hp)).collect(),
        _ => draft_pts.clone(),
    };
    let hint = match tool() {
        Tool::Area if drafting => "Tap to add corners. Tap the first corner or ✓ to close the area.",
        Tool::Area => "Tap the map to place the first corner of the area.",
        Tool::Line if drafting => "Tap to add points. Tap ✓ to finish the line.",
        Tool::Line => "Tap the map to start the line.",
        Tool::Marker => "Tap the map where the marker goes.",
        Tool::Pan if corner().is_some() => "Corner selected: delete it with the minus button. On a computer, right-click a corner to delete it.",
        Tool::Pan if sel.is_some() => "Drag a corner to move it, or a small dot to add one. Tap a corner to select it; right-click deletes it.",
        Tool::Pan => "Drag to move the map. Tap a shape to change it.",
    };
    let shapes_count = shown.len();
    let fit_pts = {
        let own = all_points(&shapes);
        if own.is_empty() { all_points(&background) } else { own }
    };

    let tool_button = move |t: Tool, icon: Icon, label: &'static str| {
        let active = tool() == t;
        rsx! {
            button {
                r#type: "button",
                class: if active { "btn btn-sm btn-square btn-primary" } else { "btn btn-sm btn-square btn-ghost" },
                title: label,
                "aria-label": label,
                "aria-pressed": if active { "true" } else { "false" },
                onclick: move |_| {
                    tool.set(t);
                    draft.set(Vec::new());
                    if t != Tool::Pan {
                        selected.set(None);
                    }
                },
                IconView { icon, size: IconSize::Sm }
            }
        }
    };
    let small = |icon: Icon, label: &'static str, handler: EventHandler<()>| {
        rsx! {
            button { r#type: "button", class: "btn btn-sm btn-square btn-ghost", title: label, "aria-label": label, onclick: move |_| handler.call(()),
                IconView { icon, size: IconSize::Sm }
            }
        }
    };

    rsx! {
        div {
            class: if full() { "fixed inset-0 z-[54] overflow-hidden bg-base-200 select-none".to_string() } else { format!("relative isolate w-full overflow-hidden rounded-box border border-base-300 bg-base-200 select-none {height}") },
            "data-scroll-lock": if full() { Some("true") } else { None },
            onmounted: move |e| {
                let d = e.data();
                spawn(async move {
                    if let Ok(r) = d.get_client_rect().await {
                        box_size.set((r.width(), r.height()));
                    }
                });
            },
            onresize: move |e| {
                if let Ok(s) = e.get_border_box_size() {
                    box_size.set((s.width, s.height));
                }
            },
            for (url, x, y) in tiles_shown {
                img { key: "{url}", src: "{url}", alt: "", draggable: "false", class: "pointer-events-none absolute max-w-none", style: "left: {x:.1}px; top: {y:.1}px; width: 256px; height: 256px;" }
            }
            svg { class: "pointer-events-none absolute inset-0 h-full w-full", "aria-hidden": "true",
                for (i, s) in background.iter().enumerate() {
                    g { key: "b{i}", {draw(s, true, false)} }
                }
                for (i, s) in shown.iter().enumerate().filter(|(_, s)| s.kind == ShapeKind::Area) {
                    g { key: "a{i}", {draw(s, false, sel == Some(i))} }
                }
                for (i, s) in shown.iter().enumerate().filter(|(_, s)| s.kind == ShapeKind::Line) {
                    g { key: "l{i}", {draw(s, false, sel == Some(i))} }
                }
                for (i, s) in shown.iter().enumerate().filter(|(_, s)| s.kind == ShapeKind::Marker) {
                    g { key: "m{i}", {draw(s, false, sel == Some(i))} }
                }
                for (i, (x, y, mid)) in handles.iter().enumerate() {
                    circle { key: "h{i}", class: if !*mid && picked == Some(i) { "fill-error stroke-base-100" } else { "fill-base-100 stroke-primary" }, cx: "{x:.1}", cy: "{y:.1}", r: if *mid { "4" } else { "6.5" }, "stroke-width": "2", "fill-opacity": if *mid { "0.75" } else { "1" } }
                }
                if drafting {
                    polyline { class: "stroke-primary", points: points_attr(&preview), fill: "none", "stroke-width": "2.5", "stroke-dasharray": "6 4" }
                    for (i, (x, y)) in draft_pts.iter().enumerate() {
                        circle { key: "d{i}", class: "fill-base-100 stroke-primary", cx: "{x:.1}", cy: "{y:.1}", r: if i == 0 { "7" } else { "5" }, "stroke-width": "2" }
                    }
                }
            }
            // Catches every touch, click and wheel turn (the layers below do not).
            div { class: "absolute inset-0 z-10 {cursor}",
                style: "touch-action: none;",
                onpointerdown: on_down,
                onpointermove: on_move,
                onpointerup: move |e: PointerEvent| {
                    let p = e.element_coordinates();
                    end(e.pointer_id(), Some((p.x, p.y)))
                },
                onpointercancel: move |e: PointerEvent| end2(e.pointer_id(), None),
                onpointerleave: move |e: PointerEvent| {
                    hover.set(None);
                    end3(e.pointer_id(), None)
                },
                oncontextmenu: {
                    let mut remove_corner = remove_corner.clone();
                    move |e: MouseEvent| {
                        e.prevent_default();
                        if !editable {
                            return;
                        }
                        let p = e.element_coordinates();
                        let p = (p.x, p.y);
                        if !draft.read().is_empty() {
                            draft.write().pop();
                        } else if tool() == Tool::Pan {
                            match hit_vertex_right(p) {
                                Some((i, k, false)) => {
                                    remove_corner(i, k);
                                }
                                _ => selected.set(hit_shape_right(p)),
                            }
                        }
                    }
                },
                ondoubleclick: move |e: MouseEvent| {
                    if !editable || tool() == Tool::Pan {
                        let p = e.element_coordinates();
                        zoom_at((p.x, p.y), z + 1);
                    }
                },
                onwheel: move |e: WheelEvent| {
                    e.prevent_default();
                    let dy = e.delta().strip_units().y;
                    let acc = wheel() + dy;
                    let p = e.element_coordinates();
                    if acc <= -80.0 {
                        wheel.set(0.0);
                        zoom_at((p.x, p.y), z + 1);
                    } else if acc >= 80.0 {
                        wheel.set(0.0);
                        zoom_at((p.x, p.y), z.saturating_sub(1));
                    } else {
                        wheel.set(acc);
                    }
                },
            }
            // Zoom and view
            div { class: "absolute right-2 top-2 z-20 flex flex-col gap-1 rounded-box border border-base-300 bg-base-100/95 p-1 shadow-sm",
                {small(if full() { Icon::ExitFullScreen } else { Icon::FullScreen }, if full() { "Leave full screen" } else { "Full screen" }, EventHandler::new(move |_| {
                    full.set(!full());
                    cam.set(Some(view));
                }))}
                {small(Icon::ZoomIn, "Zoom in", EventHandler::new(move |_| zoom_at((mw / 2.0, mh / 2.0), z + 1)))}
                {small(Icon::ZoomOut, "Zoom out", EventHandler::new(move |_| zoom_at((mw / 2.0, mh / 2.0), z.saturating_sub(1))))}
                {small(Icon::Fit, "Show everything", EventHandler::new(move |_| {
                    if let Some(f) = fit_camera(&fit_pts, mw, mh, max_zoom) { cam.set(Some(f)); }
                }))}
                if let Some(h) = on_save_view {
                    {small(Icon::Locate, "Use this view as the default", EventHandler::new(move |_| h.call(view)))}
                }
            }
            if editable {
                div { class: "absolute left-2 top-2 z-20 flex flex-col gap-1 rounded-box border border-base-300 bg-base-100/95 p-1 shadow-sm",
                    {tool_button(Tool::Pan, Icon::Pointer, "Move the map and select")}
                    {tool_button(Tool::Area, Icon::Area, "Draw an area")}
                    {tool_button(Tool::Line, Icon::Line, "Draw a line")}
                    {tool_button(Tool::Marker, Icon::Marker, "Add a marker")}
                    if drafting {
                        div { class: "my-0.5 border-t border-base-300" }
                        {small(Icon::Undo, "Take back the last point", EventHandler::new(move |_| { draft.write().pop(); }))}
                        {small(Icon::Check, "Finish", { let mut finish = finish.clone(); EventHandler::new(move |_| finish()) })}
                        {small(Icon::Close, "Cancel", EventHandler::new(move |_| { draft.set(Vec::new()); tool.set(Tool::Pan); }))}
                    }
                    if let Some(i) = sel {
                        div { class: "my-0.5 border-t border-base-300" }
                        if shown.get(i).is_some_and(|s| s.kind == ShapeKind::Marker) {
                            {small(Icon::Edit, "Edit the marker", {
                                let base = base.clone();
                                EventHandler::new(move |_| {
                                    if let Some(s) = base.get(i) {
                                        name.set(s.label.clone());
                                        marker_tone.set(s.tone);
                                        naming.set(Some((s.points.first().copied().unwrap_or_default(), Some(i))));
                                    }
                                })
                            })}
                        }
                        if let Some(k) = corner() {
                            {small(Icon::RemovePoint, "Delete the selected corner", {
                                let mut remove_corner = remove_corner.clone();
                                EventHandler::new(move |_| {
                                    remove_corner(i, k);
                                })
                            })}
                        }
                        {small(Icon::Trash, "Delete the selected shape", {
                            let base = base.clone();
                            EventHandler::new(move |_| {
                                let mut list = base.clone();
                                if i < list.len() {
                                    list.remove(i);
                                    selected.set(None);
                                    corner.set(None);
                                    commit(list);
                                }
                            })
                        })}
                    }
                }
                div { class: "pointer-events-none absolute bottom-6 left-2 z-20 max-w-[75%] rounded-field bg-base-100/90 px-2 py-1 text-xs text-base-content/80 shadow-sm",
                    "{hint}"
                    if shapes_count == 0 && !drafting && tool() == Tool::Pan {
                        " Start with the area tool."
                    }
                }
            }
            match &tiles {
                Some(t) => rsx! {
                    div { class: "pointer-events-none absolute bottom-0 right-0 z-20 rounded-tl bg-base-100/85 px-1.5 py-0.5 text-[10px] text-base-content/70", "{t.attribution}" }
                },
                None => rsx! {
                    div { class: "pointer-events-none absolute bottom-0 right-0 z-20 rounded-tl bg-base-100/85 px-1.5 py-0.5 text-[10px] text-base-content/70", "Map images are off" }
                },
            }
        }
        if let Some((at, index)) = naming() {
            Dialog {
                open: true,
                onclose: move |_| naming.set(None),
                title: if index.is_some() { "Marker" } else { "Add a marker" },
                description: "A short title, shown next to the point. For example: Gate code 1234, or Dog.",
                actions: rsx! {
                    if let Some(i) = index {
                        ActionButton {
                            action: Action::Delete,
                            onclick: {
                                let base = base.clone();
                                move |_| {
                                    let mut list = base.clone();
                                    if i < list.len() {
                                        list.remove(i);
                                        commit(list);
                                    }
                                    selected.set(None);
                                    naming.set(None);
                                }
                            },
                        }
                    }
                    ActionButton { action: Action::Cancel, onclick: move |_| naming.set(None) }
                    ActionButton {
                        action: Action::Save,
                        onclick: {
                            let base = base.clone();
                            move |_| {
                                let label = name().trim().to_string();
                                let tone = marker_tone();
                                let mut list = base.clone();
                                match index {
                                    Some(i) => {
                                        if let Some(s) = list.get_mut(i) {
                                            s.label = label;
                                            s.tone = tone;
                                        }
                                    }
                                    None => {
                                        list.push(MapShape { id: String::new(), kind: ShapeKind::Marker, points: vec![at], label, tone });
                                        selected.set(Some(list.len() - 1));
                                    }
                                }
                                commit(list);
                                naming.set(None);
                                tool.set(Tool::Pan);
                            }
                        },
                    }
                },
                Field { label: "Title",
                    TextInput { value: name(), placeholder: "For example: Entrance at the back", oninput: move |v| name.set(v) }
                }
                Field { label: "Colour",
                    ColorPicker {
                        value: marker_tone().value().to_string(),
                        options: MapTone::MARKER.iter().map(|(t, l)| (t.value().to_string(), l.to_string(), t.picker_tone())).collect::<Vec<_>>(),
                        onchange: move |v: String| {
                            if let Some((t, _)) = MapTone::MARKER.iter().find(|(t, _)| t.value() == v) {
                                marker_tone.set(*t);
                            }
                        },
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_and_back() {
        let p = GeoPoint::new(33.98, -117.37);
        let q = unproject(project(p, 15), 15);
        assert!((p.lat - q.lat).abs() < 1e-9 && (p.lon - q.lon).abs() < 1e-9);
    }

    #[test]
    fn fit_shows_every_point() {
        let pts = [GeoPoint::new(33.97, -117.38), GeoPoint::new(33.99, -117.36)];
        let cam = fit_camera(&pts, 600.0, 400.0, 19).map(|c| c.zoom).unwrap_or(0);
        assert!((12..=15).contains(&cam), "{cam}");
    }

    #[test]
    fn point_in_polygon() {
        let sq = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)];
        assert!(inside((5.0, 5.0), &sq));
        assert!(!inside((15.0, 5.0), &sq));
    }

    #[test]
    fn tile_url_fills_the_template() {
        let t = TileSource { url: "https://{s}.example.org/{z}/{x}/{y}.png".into(), subdomains: vec!["a".into(), "b".into()], attribution: String::new(), max_zoom: 19, dark: false };
        assert_eq!(t.tile_url(3, 1, 2), "https://b.example.org/3/1/2.png");
    }
}
