//! People: the model and its rules are in `theo-core`. This module adds what needs `theo-ui`
//! types: the label colours as tones, and the list filters (facets) with their icons.

pub use theo_core::people::*;
use theo_core::prefs::Preferences;
use theo_ui::{Facet, FacetOption, FilterSelection, Icon};

/// The tone a label colour is shown with.
pub trait LabelTone {
    fn tone(self) -> theo_ui::Tone;
}

impl LabelTone for LabelColor {
    fn tone(self) -> theo_ui::Tone {
        match self {
            LabelColor::Gray => theo_ui::Tone::Neutral,
            LabelColor::Blue => theo_ui::Tone::Info,
            LabelColor::Green => theo_ui::Tone::Success,
            LabelColor::Yellow => theo_ui::Tone::Warning,
            LabelColor::Red => theo_ui::Tone::Error,
        }
    }
}

pub const FACET_STATUS: &str = "status";
pub const FACET_TYPE: &str = "type";
pub const FACET_APPOINTMENT: &str = "appointment";
pub const FACET_ANOINTED: &str = "anointed";
pub const FACET_GROUP: &str = "group";
pub const FACET_GENDER: &str = "gender";
pub const FACET_CONSENT: &str = "consent";
pub const FACET_ACTIVITY: &str = "activity";
pub const FACET_EMERGENCY: &str = "emergency";
pub const FACET_LABELS: &str = "labels";

/// (id, label, icon, values) of one filter.
pub type FacetDef = (&'static str, &'static str, Icon, Vec<(String, String)>);

/// The values a person has for a filter.
pub type FacetValues<'a> = Box<dyn Fn(&Person) -> Vec<String> + 'a>;

/// A filter a screen adds to the people filters, with the values of each person.
pub struct ExtraFacet<'a> {
    pub def: FacetDef,
    pub values: FacetValues<'a>,
}

/// The people filters of *All users*, in display order.
pub const ALL_FACETS: [&str; 10] = [
    FACET_STATUS, FACET_LABELS, FACET_TYPE, FACET_APPOINTMENT, FACET_GROUP, FACET_ACTIVITY, FACET_GENDER, FACET_ANOINTED, FACET_CONSENT,
    FACET_EMERGENCY,
];

/// The standard people filters listed in `ids`, in that order (the same filter looks and works
/// the same on every list).
pub fn standard_facets(dir: &Directory, ids: &[&str]) -> Vec<FacetDef> {
    let all = facet_defs(dir);
    ids.iter().filter_map(|id| all.iter().find(|d| d.0 == *id).cloned()).collect()
}

/// Every people filter, in display order.
fn facet_defs(dir: &Directory) -> Vec<FacetDef> {
    let v = |a: &str, b: &str| (a.to_string(), b.to_string());
    let mut groups: Vec<(String, String)> = dir.groups.iter().map(|g| (g.id.to_string(), g.name.clone())).collect();
    groups.push(v("none", "No group"));
    vec![
        (FACET_STATUS, "Status", Icon::User, vec![v("current", "Current"), v("moved-away", "Moved away")]),
        (FACET_TYPE, "Type", Icon::Users, PublisherType::ALL.iter().map(|t| v(t.value(), t.label())).collect()),
        (FACET_APPOINTMENT, "Appointment", Icon::Privilege, vec![v("elder", "Elder"), v("ms", "Ministerial servant"), v("none", "No appointment")]),
        (FACET_GROUP, "Group", Icon::Group, groups),
        (FACET_ACTIVITY, "Activity", Icon::Report, Activity::ALL.iter().map(|a| v(a.value(), a.label())).collect()),
        (FACET_GENDER, "Gender", Icon::User, vec![v("male", "Male"), v("female", "Female")]),
        (FACET_ANOINTED, "Anointed", Icon::Sparkles, vec![v("yes", "Anointed"), v("no", "Not anointed")]),
        (FACET_CONSENT, "Consent", Icon::Shield, vec![v("yes", "Consent recorded"), v("no", "No consent recorded")]),
        (FACET_EMERGENCY, "Emergency contact", Icon::Phone, vec![v("yes", "Has emergency contacts"), v("no", "No emergency contact")]),
        (FACET_LABELS, "Labels", Icon::Tag, {
            let mut l: Vec<(String, String)> = dir.labels.iter().map(|x| (x.id.to_string(), x.name.clone())).collect();
            l.push(v("none", "No labels"));
            l
        }),
    ]
}

/// The values a person has for a filter (for lists of other things about people, like absences).
pub fn person_values(dir: &Directory, p: &Person, facet: &str) -> Vec<String> {
    values(dir, p, facet)
}

/// The values a person has for a filter.
fn values(dir: &Directory, p: &Person, facet: &str) -> Vec<String> {
    let yes_no = |b: bool| vec![if b { "yes" } else { "no" }.to_string()];
    match facet {
        FACET_STATUS => vec![if p.moved_away.is_some() { "moved-away" } else { "current" }.into()],
        FACET_TYPE => vec![p.publisher_type.value().into()],
        FACET_APPOINTMENT => vec![p.appointment.map(|a| a.value()).unwrap_or("none").into()],
        FACET_ANOINTED => yes_no(p.anointed),
        FACET_GENDER => vec![p.gender.value().into()],
        FACET_CONSENT => yes_no(p.consent_date.is_some()),
        FACET_EMERGENCY => yes_no(!p.emergency_contacts.is_empty()),
        FACET_LABELS => {
            if p.labels.is_empty() { vec!["none".into()] } else { p.labels.iter().map(|l| l.to_string()).collect() }
        }
        FACET_ACTIVITY => dir.activity(p).map(|a| vec![a.value().to_string()]).unwrap_or_default(),
        FACET_GROUP => {
            let gs: Vec<String> = dir.groups_of(p.id).iter().map(|(g, _)| g.id.to_string()).collect();
            if gs.is_empty() { vec!["none".into()] } else { gs }
        }
        _ => Vec::new(),
    }
}

fn matches_query(p: &Person, tokens: &[String]) -> bool {
    let hay = fold(&format!("{} {}", p.first_name, p.last_name));
    tokens.iter().all(|t| hay.contains(t.as_str()))
}

fn values_with(dir: &Directory, extra: &[ExtraFacet], p: &Person, facet: &str) -> Vec<String> {
    match extra.iter().find(|e| e.def.0 == facet) {
        Some(e) => (e.values)(p),
        None => values(dir, p, facet),
    }
}

fn matches(dir: &Directory, extra: &[ExtraFacet], p: &Person, tokens: &[String], sel: &FilterSelection, skip: Option<&str>) -> bool {
    matches_query(p, tokens)
        && sel.iter().all(|(facet, chosen)| {
            Some(facet.as_str()) == skip || chosen.is_empty() || values_with(dir, extra, p, facet).iter().any(|v| chosen.contains(v))
        })
}

/// The people of `pool` that match the search and the filters, sorted by the name as shown.
pub fn filter_pool<'a>(
    dir: &Directory,
    pool: &[&'a Person],
    query: &str,
    sel: &FilterSelection,
    extra: &[ExtraFacet],
    prefs: &Preferences,
) -> Vec<&'a Person> {
    let t = tokens(query);
    let mut out: Vec<&Person> = pool.iter().copied().filter(|p| matches(dir, extra, p, &t, sel, None)).collect();
    out.sort_by_cached_key(|p| fold(&p.display_name(prefs)));
    out
}

/// Filters for `pool`: the extra ones first, then `defs`. Each value shows how many people it
/// gives with the other filters applied.
pub fn facets_for(dir: &Directory, pool: &[&Person], query: &str, sel: &FilterSelection, defs: Vec<FacetDef>, extra: &[ExtraFacet]) -> Vec<Facet> {
    let t = tokens(query);
    extra
        .iter()
        .map(|e| e.def.clone())
        .chain(defs)
        .map(|(id, label, icon, opts)| {
            let pool: Vec<&Person> = pool.iter().copied().filter(|p| matches(dir, extra, p, &t, sel, Some(id))).collect();
            let options = opts
                .into_iter()
                .map(|(value, label)| {
                    let count = pool.iter().filter(|p| values_with(dir, extra, p, id).contains(&value)).count();
                    FacetOption { value, label, count }
                })
                .collect();
            Facet { id: id.into(), label: label.into(), icon, options }
        })
        .collect()
}

fn tokens(query: &str) -> Vec<String> {
    fold(query).split(|c: char| c.is_whitespace() || c == ',').filter(|t| !t.is_empty()).map(str::to_string).collect()
}

/// The people that match, sorted by the name as shown.
pub fn filtered<'a>(dir: &'a Directory, query: &str, sel: &FilterSelection, prefs: &Preferences) -> Vec<&'a Person> {
    let pool: Vec<&Person> = dir.people.iter().collect();
    filter_pool(dir, &pool, query, sel, &[], prefs)
}

/// The filters of *All users* with their values and counts.
pub fn facets(dir: &Directory, query: &str, sel: &FilterSelection) -> Vec<Facet> {
    let pool: Vec<&Person> = dir.people.iter().collect();
    facets_for(dir, &pool, query, sel, standard_facets(dir, &ALL_FACETS), &[])
}

/// The list shows current members only, until the user changes the Status filter.
pub fn default_filters() -> FilterSelection {
    let mut s = FilterSelection::new();
    s.insert(FACET_STATUS.into(), std::iter::once("current".to_string()).collect());
    s
}

/// People who moved away, most recent first.
pub fn moved_away(dir: &Directory) -> Vec<&Person> {
    let mut out: Vec<&Person> = dir.people.iter().filter(|p| p.moved_away.is_some()).collect();
    out.sort_by(|a, b| b.moved_away.cmp(&a.moved_away));
    out
}

/// Human description of the active filters (export headers).
pub fn describe(dir: &Directory, query: &str, sel: &FilterSelection) -> String {
    let mut parts: Vec<String> = Vec::new();
    if !query.trim().is_empty() {
        parts.push(format!("Search: \"{}\"", query.trim()));
    }
    for (id, label, _, opts) in facet_defs(dir) {
        if let Some(chosen) = sel.get(id).filter(|c| !c.is_empty()) {
            let names: Vec<String> = opts.iter().filter(|(v, _)| chosen.contains(v)).map(|(_, l)| l.clone()).collect();
            parts.push(format!("{label}: {}", names.join(" or ")));
        }
    }
    if parts.is_empty() { "All users".into() } else { parts.join(" · ") }
}

/// Builds a selection (tests).
#[cfg(test)]
pub fn selection(pairs: &[(&str, &[&str])]) -> FilterSelection {
    pairs
        .iter()
        .map(|(k, vs)| (k.to_string(), vs.iter().map(|v| v.to_string()).collect::<std::collections::BTreeSet<_>>()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir() -> Directory {
        crate::mock::demo_directory(40)
    }

    #[test]
    fn filters_combine_or_inside_and_across() {
        let d = dir();
        let prefs = Preferences::default();
        let both = filtered(&d, "", &selection(&[(FACET_GENDER, &["male", "female"])]), &prefs).len();
        assert_eq!(both, d.people.len());
        let north = d.groups[0].id.to_string();
        let sel = selection(&[(FACET_GENDER, &["male", "female"]), (FACET_GROUP, &[north.as_str()]), (FACET_CONSENT, &["no"])]);
        for p in filtered(&d, "", &sel, &prefs) {
            assert!(d.groups[0].includes(p.id));
            assert!(p.consent_date.is_none());
        }
    }

    #[test]
    fn search_ignores_accents_case_and_order() {
        let d = dir();
        let prefs = Preferences::default();
        let a = filtered(&d, "alvarez jose", &FilterSelection::new(), &prefs);
        assert_eq!(a.len(), 1);
        assert_eq!(a[0].first_name, "José");
    }

    #[test]
    fn the_default_list_hides_people_who_moved_away() {
        let mut d = dir();
        d.move_away(7, "2026-11-01", true);
        let current = filtered(&d, "", &default_filters(), &Preferences::default());
        assert!(current.iter().all(|p| p.moved_away.is_none()));
    }
}
