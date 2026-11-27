//! Territories: the model and its rules are in `theo-core`. This module adds the conversions to
//! and from the `theo-ui` map component.

pub use theo_core::territories::*;
use theo_core::people::LabelColor;

/// The map tiles of the territory settings.
pub trait Tiles {
    /// The tiles to show, or None when maps are off.
    fn tiles(&self) -> Option<theo_ui::TileSource>;
}

impl Tiles for TerritorySettings {
    fn tiles(&self) -> Option<theo_ui::TileSource> {
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

