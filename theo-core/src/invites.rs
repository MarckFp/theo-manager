//! Invite links (REQUIREMENTS.md 5.6): the invite's token.
//!
//! ```text
//! https://<app address>/join#t=<token>
//! ```
//!
//! The server is the app's own Cloud service (D-49), so the link does not name it. Links from
//! builds before D-49 also carry `s=<server>`; it is read so the app can refuse a link for another
//! server. Everything is in the **fragment** (`#...`), which browsers never send to the web host,
//! so it never reaches its logs (R-251). The link holds no congregation password and no root key
//! (R-252): the token opens only the invite's bundle, once.

use theo_crypto::DomainId;

use crate::access::Access;

/// How long an invite works (R-253).
pub const INVITE_DAYS: u32 = 7;

/// What an invite link holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InviteLink {
    /// The token (64 hex digits, `theo_crypto::invite::InviteToken::to_text`).
    pub token: String,
    /// The server a link from an older build names (`s=`); None in current links.
    pub server: Option<String>,
}

fn decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = if bytes[i] == b'%' { s.get(i + 1..i + 3).and_then(|h| u8::from_str_radix(h, 16).ok()) } else { None };
        match hex {
            Some(b) => {
                out.push(b);
                i += 3;
            }
            None => {
                out.push(bytes[i]);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

impl InviteLink {
    /// A link for `token`.
    pub fn new(token: &str) -> Self {
        InviteLink { token: token.to_string(), server: None }
    }

    /// The link to send. `app` is the app's address (on the web, the page's origin); the
    /// installed apps accept the same link pasted on their join screen.
    pub fn to_url(&self, app: &str) -> String {
        format!("{}/join#t={}", app.trim_end_matches('/'), self.token)
    }

    /// Reads a link, or just its fragment. None without a valid token.
    pub fn parse(text: &str) -> Option<InviteLink> {
        let text = text.trim();
        let fragment = text.split_once('#').map_or(text, |(_, f)| f);
        let (mut s, mut t) = (None, None);
        for pair in fragment.split('&') {
            let Some((k, v)) = pair.split_once('=') else { continue };
            match k {
                "s" => s = Some(decode(v)),
                "t" => t = Some(decode(v)),
                _ => {}
            }
        }
        let token = t.filter(|t| t.len() == 64 && t.chars().all(|c| c.is_ascii_hexdigit()))?;
        Some(InviteLink { token, server: s.filter(|u| !u.is_empty()) })
    }
}

/// The key domains an account with `access` gets (6.2, R-247). Administrators get every domain
/// (they join with the congregation password instead, a full-access invite).
///
/// ⚠️ Every member gets **schedule** and **people**: the app needs names, the member's own person
/// and their permissions, which are all in the people domain today. So a member can decrypt the
/// people domain (contact details too) until it is split (M-02 modelling, to decide).
pub fn domains_for(access: &Access) -> Vec<DomainId> {
    if access.admin {
        return vec![DomainId::SCHEDULE, DomainId::PEOPLE, DomainId::REPORTS, DomainId::TERRITORIES];
    }
    let mut out = vec![DomainId::SCHEDULE, DomainId::PEOPLE];
    let reads = |page: &str| access.level(page) != crate::access::Level::None;
    if reads("ministry/reports") || reads("meetings/attendance") || access.own_group_reports || access.own_group_read || access.all_groups_read {
        out.push(DomainId::REPORTS);
    }
    if reads("ministry/territories") {
        out.push(DomainId::TERRITORIES);
    }
    out
}

/// What every member writes, whatever their permissions (when they hold the domain's key): their
/// own absences, ministry log, goals and field service reports. ⚠️ The server checks collections, not owners: a member could
/// change another person's record in these collections, until records carry their owner (R-242
/// `scope_id`, with sealed submissions, R-327).
pub const OWN_DATA: &[&str] = &["absences", "ministry_log", "ministry_carried", "personal_goals", "reports"];

/// The collections each page's **write** permission covers (M-02 pages, `access::PAGES`).
pub const PAGE_WRITES: &[(&str, &[&str])] = &[
    ("users/all", &["people", "families", "labels"]),
    ("users/privileges", &["people"]),
    ("users/absences", &["absences"]),
    ("users/elderly-care", &["people"]),
    ("users/moved-away", &["people", "families"]),
    ("meetings/midweek", &["midweek_weeks", "midweek_settings", "notices"]),
    ("meetings/weekend", &["weekend_weeks", "weekend_settings", "notices"]),
    ("meetings/talks", &["talk_settings", "other_congregations", "visiting_speakers", "speaker_talks", "notices"]),
    ("meetings/outgoing", &["outgoing_talks", "other_congregations", "visiting_speakers", "notices"]),
    ("meetings/attendance", &["attendance"]),
    ("meetings/events", &["events"]),
    ("ministry/reports", &["reports", "aux_short_months", "aux_plans"]),
    ("ministry/groups", &["groups"]),
    ("ministry/territories", &["territories", "territory_assignments", "territory_settings", "territory_categories", "territory_labels", "assignment_labels"]),
    ("ministry/witnessing", &["witnessing", "shift_presets", "shifts", "notices"]),
    ("ministry/service-meetings", &["service_meetings", "service_presets", "notices"]),
    ("departments/audio-video", &["duty_types", "duty_meetings", "notices"]),
    ("departments/attendants", &["duty_types", "duty_meetings", "notices"]),
    ("departments/microphones", &["duty_types", "duty_meetings", "notices"]),
    ("departments/cleaning", &["cleaning", "notices"]),
    ("departments/maintenance", &["maintenance_tasks"]),
    ("departments/custom-tasks", &["custom_tasks"]),
    ("settings", &[crate::collections::DETAILS]),
];

/// The collections an account with `access` may write: what the server lets its membership
/// write (R-241). Administrators write everything (their role on the server says so), so this is
/// for members. Permissions, the owner and the activity log stay with administrators.
pub fn writes_for(access: &Access) -> Vec<String> {
    let mut out: std::collections::BTreeSet<&str> = OWN_DATA.iter().copied().collect();
    if access.admin {
        out.extend(crate::collections::COLLECTIONS.iter().map(|c| c.name));
    }
    for (page, collections) in PAGE_WRITES {
        if access.level(page) == crate::access::Level::Write {
            out.extend(collections.iter().copied());
        }
    }
    if access.own_group_reports {
        out.insert("reports");
    }
    // Only what they also hold the key of: a publisher's own ministry log and reports are in the
    // reports domain, which they cannot read; they send those with sealed submissions (R-327,
    // to do), not by writing the collection.
    let keys = domains_for(access);
    out.into_iter().filter(|c| crate::collections::domain_of(c).is_some_and(|d| keys.contains(&d))).map(String::from).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn token() -> String {
        "ab".repeat(32)
    }

    #[test]
    fn links_round_trip_and_keep_everything_in_the_fragment() {
        let link = InviteLink::new(&token());
        let url = link.to_url("https://app.example.org/");
        assert_eq!(url, format!("https://app.example.org/join#t={}", token()));
        assert_eq!(InviteLink::parse(&url), Some(link.clone()));
        // Pasted without the address, or with spaces around it.
        assert_eq!(InviteLink::parse(&format!("  t={} ", token())), Some(link));
        // A link from an older build names its server.
        let old = InviteLink::parse(&format!("https://x/join#s=https://db.example.org&t={}", token()));
        assert_eq!(old.and_then(|l| l.server).as_deref(), Some("https://db.example.org"));

        assert_eq!(InviteLink::parse("https://app.example.org/join#t=abc"), None);
        assert_eq!(InviteLink::parse("https://app.example.org/join#s=https://x"), None);
    }

    #[test]
    fn writes_follow_the_permissions_and_need_their_keys() {
        let preset = |id: &str| crate::access::preset(id).map(|p| (p.access)()).unwrap_or_default();
        let publisher = writes_for(&preset("publisher"));
        assert!(publisher.contains(&"absences".to_string()) && !publisher.contains(&"people".to_string()));
        let lmm = writes_for(&preset("lmm-overseer"));
        assert!(lmm.contains(&"midweek_weeks".to_string()) && !lmm.contains(&"weekend_weeks".to_string()));
        assert!(!lmm.iter().any(|c| c == "access" || c == "log" || c == "owner"));
        // Every page and collection named here exists, and a member holds the key of what they write.
        for (page, collections) in PAGE_WRITES {
            assert!(*page == "settings" || crate::access::PAGES.iter().any(|(p, _)| p == page), "{page}");
            for c in *collections {
                assert!(crate::collections::domain_of(c).is_some(), "{c}");
            }
        }
        for id in ["publisher", "secretary", "service-overseer", "group-overseer", "lmm-overseer", "talk-coordinator", "duties-coordinator"] {
            let a = preset(id);
            let keys = domains_for(&a);
            for c in writes_for(&a) {
                assert!(crate::collections::domain_of(&c).is_some_and(|d| keys.contains(&d)), "{id} writes {c} without its key");
            }
        }
    }

    #[test]
    fn domains_follow_the_permissions() {
        let preset = |id: &str| crate::access::preset(id).map(|p| (p.access)()).unwrap_or_default();
        assert_eq!(domains_for(&Access::administrator()).len(), 4);
        assert_eq!(domains_for(&preset("publisher")), vec![DomainId::SCHEDULE, DomainId::PEOPLE]);
        assert!(domains_for(&preset("secretary")).contains(&DomainId::REPORTS));
        assert!(domains_for(&preset("service-overseer")).contains(&DomainId::TERRITORIES));
        assert!(!domains_for(&preset("lmm-overseer")).contains(&DomainId::TERRITORIES));
    }
}
