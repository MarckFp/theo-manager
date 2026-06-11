use dioxus::prelude::*;
use dioxus_i18n::t;
use surrealdb::types::RecordId;

use crate::database::{use_crypto, use_db};
use crate::models::privilege::{UserPrivileges, UserPrivilegesData, PRIV_TOTAL};
use crate::models::user::{Appointment, Gender, User, UserType};
use crate::pages::app::user::{format_name, effective_name_format};
use crate::models::congregation::{Congregation, NameFormat};

// ── Helper ────────────────────────────────────────────────────────────────────

fn normalize(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'á' | 'à' | 'ä' | 'â' | 'ã' | 'Á' | 'À' | 'Ä' | 'Â' | 'Ã' => 'a',
            'é' | 'è' | 'ë' | 'ê' | 'É' | 'È' | 'Ë' | 'Ê' => 'e',
            'í' | 'ì' | 'ï' | 'î' | 'Í' | 'Ì' | 'Ï' | 'Î' => 'i',
            'ó' | 'ò' | 'ö' | 'ô' | 'õ' | 'Ó' | 'Ò' | 'Ö' | 'Ô' | 'Õ' => 'o',
            'ú' | 'ù' | 'ü' | 'û' | 'Ú' | 'Ù' | 'Ü' | 'Û' => 'u',
            'ñ' | 'Ñ' => 'n',
            'ç' | 'Ç' => 'c',
            other => other.to_ascii_lowercase(),
        })
        .collect()
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

// ── Privilege flags (form state mirrors UserPrivileges booleans) ──────────────

#[derive(Clone, Default)]
struct PrivilegeFlags {
    // Midweek Meeting
    weekday_pray: bool,
    weekday_chairman: bool,
    aux_chairman: bool,
    treasures: bool,
    spiritual_gems: bool,
    bible_reading: bool,
    field_ministry_discussion: bool,
    starting_conversation: bool,
    following_up: bool,
    making_disciples: bool,
    assistant: bool,
    student_talk: bool,
    living_as_christians: bool,
    congregation_bible_study: bool,
    congregation_bible_study_reader: bool,
    // Weekend Meeting
    weekend_pray: bool,
    weekend_chairman: bool,
    watchtower_conductor: bool,
    public_talks: bool,
    public_talks_away: bool,
    // Departments
    stage: bool,
    audio: bool,
    video: bool,
    microphones: bool,
    attendant: bool,
    zoom_attendant: bool,
    // Other
    hospitality: bool,
    interpreter: bool,
    field_service_meeting: bool,
    public_witnessing: bool,
    cleaning: bool,
    maintenance: bool,
    territory: bool,
}

impl PrivilegeFlags {
    fn from_privileges(p: &UserPrivileges) -> Self {
        Self {
            weekday_pray: p.weekday_pray,
            weekday_chairman: p.weekday_chairman,
            aux_chairman: p.aux_chairman,
            treasures: p.treasures,
            spiritual_gems: p.spiritual_gems,
            bible_reading: p.bible_reading,
            field_ministry_discussion: p.field_ministry_discussion,
            starting_conversation: p.starting_conversation,
            following_up: p.following_up,
            making_disciples: p.making_disciples,
            assistant: p.assistant,
            student_talk: p.student_talk,
            living_as_christians: p.living_as_christians,
            congregation_bible_study: p.congregation_bible_study,
            congregation_bible_study_reader: p.congregation_bible_study_reader,
            weekend_pray: p.weekend_pray,
            weekend_chairman: p.weekend_chairman,
            watchtower_conductor: p.watchtower_conductor,
            public_talks: p.public_talks,
            public_talks_away: p.public_talks_away,
            stage: p.stage,
            audio: p.audio,
            video: p.video,
            microphones: p.microphones,
            attendant: p.attendant,
            zoom_attendant: p.zoom_attendant,
            hospitality: p.hospitality,
            interpreter: p.interpreter,
            field_service_meeting: p.field_service_meeting,
            public_witnessing: p.public_witnessing,
            cleaning: p.cleaning,
            maintenance: p.maintenance,
            territory: p.territory,
        }
    }

    fn count_enabled(&self) -> usize {
        [
            self.weekday_pray, self.weekday_chairman, self.aux_chairman,
            self.treasures, self.spiritual_gems, self.bible_reading,
            self.field_ministry_discussion, self.starting_conversation,
            self.following_up, self.making_disciples, self.assistant,
            self.student_talk, self.living_as_christians,
            self.congregation_bible_study, self.congregation_bible_study_reader,
            self.weekend_pray, self.weekend_chairman, self.watchtower_conductor,
            self.public_talks, self.public_talks_away,
            self.stage, self.audio, self.video, self.microphones,
            self.attendant, self.zoom_attendant,
            self.hospitality, self.interpreter, self.field_service_meeting,
            self.public_witnessing, self.cleaning, self.maintenance, self.territory,
        ]
        .iter()
        .filter(|&&b| b)
        .count()
    }

    fn into_data(self, publisher: RecordId) -> UserPrivilegesData {
        UserPrivilegesData {
            publisher,
            weekday_pray: self.weekday_pray,
            weekday_chairman: self.weekday_chairman,
            aux_chairman: self.aux_chairman,
            treasures: self.treasures,
            spiritual_gems: self.spiritual_gems,
            bible_reading: self.bible_reading,
            field_ministry_discussion: self.field_ministry_discussion,
            starting_conversation: self.starting_conversation,
            following_up: self.following_up,
            making_disciples: self.making_disciples,
            assistant: self.assistant,
            student_talk: self.student_talk,
            living_as_christians: self.living_as_christians,
            congregation_bible_study: self.congregation_bible_study,
            congregation_bible_study_reader: self.congregation_bible_study_reader,
            weekend_pray: self.weekend_pray,
            weekend_chairman: self.weekend_chairman,
            watchtower_conductor: self.watchtower_conductor,
            public_talks: self.public_talks,
            public_talks_away: self.public_talks_away,
            stage: self.stage,
            audio: self.audio,
            video: self.video,
            microphones: self.microphones,
            attendant: self.attendant,
            zoom_attendant: self.zoom_attendant,
            hospitality: self.hospitality,
            interpreter: self.interpreter,
            field_service_meeting: self.field_service_meeting,
            public_witnessing: self.public_witnessing,
            cleaning: self.cleaning,
            maintenance: self.maintenance,
            territory: self.territory,
        }
    }
}

// ── Edit target (passed as prop to modal) ─────────────────────────────────────

#[derive(Clone, PartialEq)]
struct EditTarget {
    user: User,
    existing: Option<UserPrivileges>,
}

// ── Privilege field filter ────────────────────────────────────────────────────

#[derive(Clone, PartialEq, Default)]
enum PrivFieldFilter {
    #[default]
    All,
    WeekdayPray, WeekdayChairman, AuxChairman,
    Treasures, SpiritualGems, BibleReading,
    FieldMinistryDiscussion, StartingConversation, FollowingUp,
    MakingDisciples, Assistant, StudentTalk,
    LivingAsChristians, CongregationBibleStudy, CongregationBibleStudyReader,
    WeekendPray, WeekendChairman, WatchtowerConductor,
    PublicTalks, PublicTalksAway,
    Stage, Audio, Video, Microphones, Attendant, ZoomAttendant,
    Hospitality, Interpreter, FieldServiceMeeting,
    PublicWitnessing, Cleaning, Maintenance, Territory,
}

impl PrivFieldFilter {
    fn from_str(s: &str) -> Self {
        match s {
            "weekday_pray" => Self::WeekdayPray,
            "weekday_chairman" => Self::WeekdayChairman,
            "aux_chairman" => Self::AuxChairman,
            "treasures" => Self::Treasures,
            "spiritual_gems" => Self::SpiritualGems,
            "bible_reading" => Self::BibleReading,
            "field_ministry_discussion" => Self::FieldMinistryDiscussion,
            "starting_conversation" => Self::StartingConversation,
            "following_up" => Self::FollowingUp,
            "making_disciples" => Self::MakingDisciples,
            "assistant" => Self::Assistant,
            "student_talk" => Self::StudentTalk,
            "living_as_christians" => Self::LivingAsChristians,
            "congregation_bible_study" => Self::CongregationBibleStudy,
            "congregation_bible_study_reader" => Self::CongregationBibleStudyReader,
            "weekend_pray" => Self::WeekendPray,
            "weekend_chairman" => Self::WeekendChairman,
            "watchtower_conductor" => Self::WatchtowerConductor,
            "public_talks" => Self::PublicTalks,
            "public_talks_away" => Self::PublicTalksAway,
            "stage" => Self::Stage,
            "audio" => Self::Audio,
            "video" => Self::Video,
            "microphones" => Self::Microphones,
            "attendant" => Self::Attendant,
            "zoom_attendant" => Self::ZoomAttendant,
            "hospitality" => Self::Hospitality,
            "interpreter" => Self::Interpreter,
            "field_service_meeting" => Self::FieldServiceMeeting,
            "public_witnessing" => Self::PublicWitnessing,
            "cleaning" => Self::Cleaning,
            "maintenance" => Self::Maintenance,
            "territory" => Self::Territory,
            _ => Self::All,
        }
    }
}

fn has_privilege(p: &UserPrivileges, field: &PrivFieldFilter) -> bool {
    match field {
        PrivFieldFilter::All => true,
        PrivFieldFilter::WeekdayPray => p.weekday_pray,
        PrivFieldFilter::WeekdayChairman => p.weekday_chairman,
        PrivFieldFilter::AuxChairman => p.aux_chairman,
        PrivFieldFilter::Treasures => p.treasures,
        PrivFieldFilter::SpiritualGems => p.spiritual_gems,
        PrivFieldFilter::BibleReading => p.bible_reading,
        PrivFieldFilter::FieldMinistryDiscussion => p.field_ministry_discussion,
        PrivFieldFilter::StartingConversation => p.starting_conversation,
        PrivFieldFilter::FollowingUp => p.following_up,
        PrivFieldFilter::MakingDisciples => p.making_disciples,
        PrivFieldFilter::Assistant => p.assistant,
        PrivFieldFilter::StudentTalk => p.student_talk,
        PrivFieldFilter::LivingAsChristians => p.living_as_christians,
        PrivFieldFilter::CongregationBibleStudy => p.congregation_bible_study,
        PrivFieldFilter::CongregationBibleStudyReader => p.congregation_bible_study_reader,
        PrivFieldFilter::WeekendPray => p.weekend_pray,
        PrivFieldFilter::WeekendChairman => p.weekend_chairman,
        PrivFieldFilter::WatchtowerConductor => p.watchtower_conductor,
        PrivFieldFilter::PublicTalks => p.public_talks,
        PrivFieldFilter::PublicTalksAway => p.public_talks_away,
        PrivFieldFilter::Stage => p.stage,
        PrivFieldFilter::Audio => p.audio,
        PrivFieldFilter::Video => p.video,
        PrivFieldFilter::Microphones => p.microphones,
        PrivFieldFilter::Attendant => p.attendant,
        PrivFieldFilter::ZoomAttendant => p.zoom_attendant,
        PrivFieldFilter::Hospitality => p.hospitality,
        PrivFieldFilter::Interpreter => p.interpreter,
        PrivFieldFilter::FieldServiceMeeting => p.field_service_meeting,
        PrivFieldFilter::PublicWitnessing => p.public_witnessing,
        PrivFieldFilter::Cleaning => p.cleaning,
        PrivFieldFilter::Maintenance => p.maintenance,
        PrivFieldFilter::Territory => p.territory,
    }
}

// ── Filter state ──────────────────────────────────────────────────────────────

#[derive(Clone, PartialEq, Default)]
struct Filters {
    name: String,
    gender: Option<Gender>,
    user_type: Option<UserType>,
    appointment: Option<Option<Appointment>>, // None=All, Some(None)=no appt, Some(Some(x))=specific
    privilege: PrivFieldFilter,
}

// ── Main page ─────────────────────────────────────────────────────────────────

#[component]
pub fn AppPrivileges() -> Element {
    let db_signal = use_db();
    let crypto_signal = use_crypto();
    let congregation_res = use_context::<Resource<Option<Congregation>>>();
    let uid = db_signal.read().congregation_uid.clone().unwrap_or_default();

    let mut name_fmt = use_signal(|| NameFormat::FirstLast);
    {
        let uid = uid.clone();
        use_effect(move || {
            let uid = uid.clone();
            let cong_snap = congregation_res.read().clone();
            let db_opt = db_signal.read().db.clone();
            spawn(async move {
                let prefs = crate::pages::app::user_settings::load_prefs(&uid, db_opt).await;
                let cong_ref = cong_snap.as_ref().and_then(|o| o.as_ref());
                name_fmt.set(effective_name_format(
                    cong_ref,
                    prefs.name_format.as_deref().unwrap_or(""),
                ));
            });
        });
    }

    let mut users_res = use_resource(move || async move {
        let Some(db) = db_signal.read().db.clone() else { return vec![] };
        let crypto = crypto_signal.read().clone();
        User::all(&db, &crypto).await.unwrap_or_default()
    });

    let mut privs_res = use_resource(move || async move {
        let Some(db) = db_signal.read().db.clone() else { return vec![] };
        UserPrivileges::all(&db).await.unwrap_or_default()
    });

    let mut bootstrapped = use_signal(|| false);
    use_effect(move || {
        if *bootstrapped.peek() { return; }
        bootstrapped.set(true);
        users_res.restart();
        privs_res.restart();
    });

    let mut filters = use_signal(Filters::default);
    let mut edit_open = use_signal(|| false);
    let mut edit_target: Signal<Option<EditTarget>> = use_signal(|| None);

    let is_loading = users_res.read().is_none() || privs_res.read().is_none();

    // Build privilege map: user_rid_str -> UserPrivileges
    let priv_map = use_memo(move || {
        let privs = privs_res().unwrap_or_default();
        privs
            .into_iter()
            .filter_map(|p| Some((rid_str(&p.publisher), p)))
            .collect::<std::collections::HashMap<_, _>>()
    });

    // Filtered + sorted user list
    let filtered = use_memo(move || {
        let users = users_res().unwrap_or_default();
        let f = filters();
        let norm = normalize(&f.name);
        let pm = priv_map();

        let mut result: Vec<User> = users
            .into_iter()
            .filter(|u| {
                if !norm.is_empty() {
                    let full = normalize(&format!("{} {}", u.first_name, u.last_name));
                    if !full.contains(&norm) { return false; }
                }
                if let Some(ref g) = f.gender {
                    if &u.gender != g { return false; }
                }
                if let Some(ref ut) = f.user_type {
                    if &u.user_type != ut { return false; }
                }
                match &f.appointment {
                    None => {}
                    Some(None) => { if u.appointment.is_some() { return false; } }
                    Some(Some(a)) => {
                        if u.appointment.as_ref() != Some(a) { return false; }
                    }
                }
                if !matches!(f.privilege, PrivFieldFilter::All) {
                    let uid_s = u.id.as_ref().map(rid_str).unwrap_or_default();
                    match pm.get(&uid_s) {
                        None => return false,
                        Some(p) => { if !has_privilege(p, &f.privilege) { return false; } }
                    }
                }
                true
            })
            .collect();

        result.sort_by(|a, b| {
            let ka = normalize(&format!("{} {}", a.last_name, a.first_name));
            let kb = normalize(&format!("{} {}", b.last_name, b.first_name));
            ka.cmp(&kb)
        });
        result
    });

    rsx! {
        div { class: "space-y-5 w-full pb-10",
            h1 { class: "text-2xl font-bold text-gray-900", {t!("page-privileges")} }

            // ── Filters ───────────────────────────────────────────────────
            div { class: "bg-white rounded-xl border border-gray-200 p-4 space-y-3",
                input {
                    r#type: "text",
                    class: "w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500 placeholder-gray-400",
                    placeholder: t!("priv-filter-placeholder"),
                    value: filters.read().name.clone(),
                    oninput: move |e| filters.write().name = e.value(),
                }
                div { class: "grid grid-cols-2 sm:grid-cols-3 gap-2",
                    // Gender
                    select {
                        class: "px-2 py-1.5 text-sm border border-gray-200 rounded-lg bg-white focus:outline-none focus:ring-2 focus:ring-primary-500",
                        onchange: move |e| {
                            filters.write().gender = match e.value().as_str() {
                                "male" => Some(Gender::Male),
                                "female" => Some(Gender::Female),
                                _ => None,
                            };
                        },
                        option { value: "", {t!("user-filter-all")} }
                        option { value: "male", {t!("user-gender-male")} }
                        option { value: "female", {t!("user-gender-female")} }
                    }
                    // User type
                    select {
                        class: "px-2 py-1.5 text-sm border border-gray-200 rounded-lg bg-white focus:outline-none focus:ring-2 focus:ring-primary-500",
                        onchange: move |e| {
                            filters.write().user_type = match e.value().as_str() {
                                "student" => Some(UserType::Student),
                                "publisher" => Some(UserType::Publisher),
                                "baptized" => Some(UserType::BaptizedPublisher),
                                "cont_aux" => Some(UserType::ContinuousAuxiliaryPioneer),
                                "regular" => Some(UserType::RegularPioneer),
                                "special" => Some(UserType::SpecialPioneer),
                                "missionary" => Some(UserType::Missionary),
                                _ => None,
                            };
                        },
                        option { value: "", {t!("user-filter-all")} }
                        option { value: "student", {t!("user-type-student")} }
                        option { value: "publisher", {t!("user-type-publisher")} }
                        option { value: "baptized", {t!("user-type-baptized")} }
                        option { value: "cont_aux", {t!("user-type-cont-aux-pioneer")} }
                        option { value: "regular", {t!("user-type-regular-pioneer")} }
                        option { value: "special", {t!("user-type-special-pioneer")} }
                        option { value: "missionary", {t!("user-type-missionary")} }
                    }
                    // Appointment
                    select {
                        class: "px-2 py-1.5 text-sm border border-gray-200 rounded-lg bg-white focus:outline-none focus:ring-2 focus:ring-primary-500",
                        onchange: move |e| {
                            filters.write().appointment = match e.value().as_str() {
                                "none" => Some(None),
                                "elder" => Some(Some(Appointment::Elder)),
                                "ms" => Some(Some(Appointment::MinisterialServant)),
                                _ => None,
                            };
                        },
                        option { value: "", {t!("user-filter-all")} }
                        option { value: "none", {t!("user-appointment-none")} }
                        option { value: "elder", {t!("user-appointment-elder")} }
                        option { value: "ms", {t!("user-appointment-ms")} }
                    }
                }
                // Privilege filter (full width)
                select {
                    class: "w-full px-2 py-1.5 text-sm border border-gray-200 rounded-lg bg-white focus:outline-none focus:ring-2 focus:ring-primary-500",
                    onchange: move |e| {
                        filters.write().privilege = PrivFieldFilter::from_str(e.value().as_str());
                    },
                    option { value: "", {t!("priv-filter-privilege")} }
                    optgroup { label: t!("priv-cat-midweek"),
                        option { value: "weekday_pray", {t!("priv-weekday-pray")} }
                        option { value: "weekday_chairman", {t!("priv-weekday-chairman")} }
                        option { value: "aux_chairman", {t!("priv-aux-chairman")} }
                        option { value: "treasures", {t!("priv-treasures")} }
                        option { value: "spiritual_gems", {t!("priv-spiritual-gems")} }
                        option { value: "bible_reading", {t!("priv-bible-reading")} }
                        option { value: "field_ministry_discussion",
                            {t!("priv-field-ministry-discussion")}
                        }
                        option { value: "starting_conversation", {t!("priv-starting-conversation")} }
                        option { value: "following_up", {t!("priv-following-up")} }
                        option { value: "making_disciples", {t!("priv-making-disciples")} }
                        option { value: "assistant", {t!("priv-assistant")} }
                        option { value: "student_talk", {t!("priv-student-talk")} }
                        option { value: "living_as_christians", {t!("priv-living-as-christians")} }
                        option { value: "congregation_bible_study",
                            {t!("priv-congregation-bible-study")}
                        }
                        option { value: "congregation_bible_study_reader",
                            {t!("priv-congregation-bible-study-reader")}
                        }
                    }
                    optgroup { label: t!("priv-cat-weekend"),
                        option { value: "weekend_pray", {t!("priv-weekend-pray")} }
                        option { value: "weekend_chairman", {t!("priv-weekend-chairman")} }
                        option { value: "watchtower_conductor", {t!("priv-watchtower-conductor")} }
                        option { value: "public_talks", {t!("priv-public-talks")} }
                        option { value: "public_talks_away", {t!("priv-public-talks-away")} }
                    }
                    optgroup { label: t!("priv-cat-platform"),
                        option { value: "stage", {t!("priv-stage")} }
                        option { value: "audio", {t!("priv-audio")} }
                        option { value: "video", {t!("priv-video")} }
                        option { value: "microphones", {t!("priv-microphones")} }
                        option { value: "attendant", {t!("priv-attendant")} }
                        option { value: "zoom_attendant", {t!("priv-zoom-attendant")} }
                        option { value: "territory", {t!("priv-territory")} }
                        option { value: "cleaning", {t!("priv-cleaning")} }
                        option { value: "maintenance", {t!("priv-maintenance")} }
                    }
                    optgroup { label: t!("priv-cat-other"),
                        option { value: "hospitality", {t!("priv-hospitality")} }
                        option { value: "interpreter", {t!("priv-interpreter")} }
                        option { value: "field_service_meeting", {t!("priv-field-service-meeting")} }
                        option { value: "public_witnessing", {t!("priv-public-witnessing")} }
                    }
                }
            }

            // ── User list ─────────────────────────────────────────────────
            if is_loading {
                div { class: "flex justify-center py-20 text-gray-400",
                    span { class: "text-sm animate-pulse", {t!("priv-loading")} }
                }
            } else if filtered.read().is_empty() {
                div { class: "bg-white rounded-xl border border-gray-200 px-6 py-14 text-center",
                    p { class: "text-4xl mb-3", "🔑" }
                    p { class: "font-medium text-gray-600", {t!("priv-no-users")} }
                }
            } else {
                div { class: "space-y-2",
                    for user in filtered.read().clone() {
                        {
                            let uid_str = user.id.as_ref().map(rid_str).unwrap_or_default();
                            let privs = priv_map.read().get(&uid_str).cloned();
                            let count = privs.as_ref().map(|p| p.count_enabled()).unwrap_or(0);
                            let u_edit = user.clone();
                            let p_edit = privs.clone();
                            rsx! {
                                UserPrivilegeCard {
                                    user,
                                    privilege_count: count,
                                    name_fmt: name_fmt.read().clone(),
                                    on_edit: move |_| {
                                        edit_target
                                            .set(
                                                Some(EditTarget {
                                                    user: u_edit.clone(),
                                                    existing: p_edit.clone(),
                                                }),
                                            );
                                        edit_open.set(true);
                                    },
                                }
                            }
                        }
                    }
                }
            }
        }

        // ── Privilege edit modal ───────────────────────────────────────────
        if let Some(target) = edit_target.read().clone() {
            PrivilegeEditModal {
                target,
                open: edit_open,
                on_close: move |_| edit_open.set(false),
                on_saved: move |_| {
                    privs_res.restart();
                    edit_open.set(false);
                },
            }
        }
    }
}

// ── UserPrivilegeCard ─────────────────────────────────────────────────────────

#[component]
fn UserPrivilegeCard(
    user: User,
    privilege_count: usize,
    name_fmt: NameFormat,
    on_edit: Callback<()>,
) -> Element {
    let display_name = format_name(&user.first_name, &user.last_name, &name_fmt);

    let type_label = match &user.user_type {
        UserType::Publisher => t!("user-type-publisher"),
        UserType::BaptizedPublisher => t!("user-type-baptized"),
        UserType::ContinuousAuxiliaryPioneer => t!("user-type-cont-aux-pioneer"),
        UserType::RegularPioneer => t!("user-type-regular-pioneer"),
        UserType::SpecialPioneer => t!("user-type-special-pioneer"),
        UserType::Missionary => t!("user-type-missionary"),
        UserType::Student => t!("user-type-student"),
    };
    let appointment_label = user.appointment.as_ref().map(|a| match a {
        Appointment::Elder => t!("user-appointment-elder"),
        Appointment::MinisterialServant => t!("user-appointment-ms"),
    });
    let count_cls = if privilege_count > 0 {
        "inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium bg-primary-100 text-primary-700"
    } else {
        "inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium bg-gray-100 text-gray-500"
    };
    let count_label = format!("{}/{}", privilege_count, PRIV_TOTAL);

    rsx! {
        div { class: "bg-white rounded-xl border border-gray-200 px-4 py-3 flex items-center justify-between gap-3",
            // Left: identity
            div { class: "flex-1 min-w-0",
                div { class: "flex flex-wrap items-center gap-1.5",
                    span { class: "text-sm font-semibold text-gray-900 truncate", "{display_name}" }
                    span { class: "inline-flex px-1.5 py-0.5 rounded-full text-xs bg-blue-100 text-blue-700",
                        "{type_label}"
                    }
                    if let Some(al) = appointment_label {
                        span { class: "inline-flex px-1.5 py-0.5 rounded-full text-xs bg-amber-100 text-amber-700",
                            "{al}"
                        }
                    }
                }
            }
            // Right: count badge + edit button
            div { class: "flex items-center gap-2 shrink-0",
                span { class: count_cls, "{count_label}" }
                button {
                    class: "px-3 py-1.5 text-xs border border-gray-200 rounded-lg text-gray-600 hover:bg-gray-50 hover:border-primary-300 transition-colors font-medium",
                    onclick: move |_| on_edit.call(()),
                    {t!("priv-btn-edit")}
                }
            }
        }
    }
}

// ── PrivilegeEditModal ────────────────────────────────────────────────────────

#[component]
fn PrivilegeEditModal(
    target: EditTarget,
    open: Signal<bool>,
    on_close: Callback<()>,
    on_saved: Callback<()>,
) -> Element {
    let db_signal = use_db();
    let mut flags = use_signal(PrivilegeFlags::default);
    let mut submitting = use_signal(|| false);
    let mut save_error: Signal<Option<String>> = use_signal(|| None);

    let existing_for_effect = target.existing.clone();
    use_effect(move || {
        if *open.read() {
            match &existing_for_effect {
                Some(p) => flags.set(PrivilegeFlags::from_privileges(p)),
                None => flags.set(PrivilegeFlags::default()),
            }
            save_error.set(None);
        }
    });

    let existing_id = target.existing.as_ref().and_then(|p| p.id.clone());
    let pub_id = target
        .user
        .id
        .clone()
        .unwrap_or_else(|| RecordId::parse_simple("user:unknown").unwrap());

    let on_submit = use_callback(move |_: Event<MouseData>| {
        let fd = flags.read().clone();
        let data = fd.into_data(pub_id.clone());
        let eid = existing_id.clone();
        submitting.set(true);
        save_error.set(None);
        spawn(async move {
            let Some(db) = db_signal.read().db.clone() else {
                submitting.set(false);
                return;
            };
            let result = if let Some(rid) = eid {
                UserPrivileges::update(&db, rid, data).await.map(|_| ())
            } else {
                UserPrivileges::create(&db, data).await.map(|_| ())
            };
            submitting.set(false);
            match result {
                Ok(_) => on_saved.call(()),
                Err(e) => save_error.set(Some(e.to_string())),
            }
        });
    });

    let user = &target.user;
    let display_name = format!("{} {}", user.first_name, user.last_name);
    let f = flags.read().clone();
    let is_open = *open.read();
    let sub = *submitting.read();

    let overlay_cls = if is_open {
        "fixed inset-0 z-50 flex items-end lg:items-center justify-center lg:p-4 bg-black/40 transition-opacity duration-300"
    } else {
        "fixed inset-0 z-50 flex items-end lg:items-center justify-center lg:p-4 bg-black/40 transition-opacity duration-300 opacity-0 pointer-events-none"
    };
    let panel_cls = if is_open {
        "relative bg-white w-full rounded-t-2xl lg:rounded-2xl shadow-2xl flex flex-col max-h-[92vh] lg:max-h-[90vh] lg:max-w-2xl overflow-hidden transition-all duration-300 translate-y-0 lg:scale-100"
    } else {
        "relative bg-white w-full rounded-t-2xl lg:rounded-2xl shadow-2xl flex flex-col max-h-[92vh] lg:max-h-[90vh] lg:max-w-2xl overflow-hidden transition-all duration-300 translate-y-full lg:translate-y-0 lg:scale-95 opacity-0"
    };

    rsx! {
        div { class: overlay_cls, onclick: move |_| on_close.call(()),
            div { class: panel_cls, onclick: move |e| e.stop_propagation(),

                // Drag handle (mobile)
                div { class: "lg:hidden shrink-0 flex justify-center pt-3 pb-2",
                    div { class: "w-10 h-1 bg-gray-300 rounded-full" }
                }

                // Header
                div { class: "shrink-0 flex items-center justify-between px-4 lg:px-6 pb-3 lg:pt-5 lg:pb-4 border-b border-gray-100",
                    div {
                        h2 { class: "text-base lg:text-lg font-semibold text-gray-900",
                            "{display_name}"
                        }
                        p { class: "text-xs text-gray-500 mt-0.5",
                            {t!("priv-edit-title")}
                            " — "
                            {
                                let cnt = f.count_enabled();
                                format!("{}/{}", cnt, PRIV_TOTAL)
                            }
                        }
                    }
                    button {
                        class: "ml-4 p-1.5 text-gray-400 hover:text-gray-600 rounded hover:bg-gray-100 transition-colors",
                        onclick: move |_| on_close.call(()),
                        "✕"
                    }
                }

                // Scrollable body
                div { class: "flex-1 overflow-y-auto px-4 lg:px-6 py-4 space-y-5",

                    if let Some(err) = save_error.read().clone() {
                        div { class: "bg-red-50 border border-red-200 text-red-700 text-sm px-3 py-2 rounded-lg",
                            "{err}"
                        }
                    }

                    // ── Midweek Meeting ───────────────────────────────────
                    PrivCategorySection { title: t!("priv-cat-midweek"),
                        // Treasures from God's Word
                        PrivSubSection {
                            title: t!("priv-sub-treasures"),
                            color: "rgb(60, 127, 139)".to_string(),
                            PrivSwitch {
                                label: t!("priv-weekday-pray"),
                                checked: f.weekday_pray,
                                on_change: move |v| flags.write().weekday_pray = v,
                            }
                            PrivSwitch {
                                label: t!("priv-weekday-chairman"),
                                checked: f.weekday_chairman,
                                on_change: move |v| flags.write().weekday_chairman = v,
                            }
                            PrivSwitch {
                                label: t!("priv-aux-chairman"),
                                checked: f.aux_chairman,
                                on_change: move |v| flags.write().aux_chairman = v,
                            }
                            PrivSwitch {
                                label: t!("priv-treasures"),
                                checked: f.treasures,
                                on_change: move |v| flags.write().treasures = v,
                            }
                            PrivSwitch {
                                label: t!("priv-spiritual-gems"),
                                checked: f.spiritual_gems,
                                on_change: move |v| flags.write().spiritual_gems = v,
                            }
                            PrivSwitch {
                                label: t!("priv-bible-reading"),
                                checked: f.bible_reading,
                                on_change: move |v| flags.write().bible_reading = v,
                            }
                        }
                        // Apply Yourself to the Ministry
                        PrivSubSection {
                            title: t!("priv-sub-field-ministry"),
                            color: "rgb(155, 109, 23)".to_string(),
                            PrivSwitch {
                                label: t!("priv-field-ministry-discussion"),
                                checked: f.field_ministry_discussion,
                                on_change: move |v| flags.write().field_ministry_discussion = v,
                            }
                            PrivSwitch {
                                label: t!("priv-starting-conversation"),
                                checked: f.starting_conversation,
                                on_change: move |v| flags.write().starting_conversation = v,
                            }
                            PrivSwitch {
                                label: t!("priv-following-up"),
                                checked: f.following_up,
                                on_change: move |v| flags.write().following_up = v,
                            }
                            PrivSwitch {
                                label: t!("priv-making-disciples"),
                                checked: f.making_disciples,
                                on_change: move |v| flags.write().making_disciples = v,
                            }
                            PrivSwitch {
                                label: t!("priv-assistant"),
                                checked: f.assistant,
                                on_change: move |v| flags.write().assistant = v,
                            }
                            PrivSwitch {
                                label: t!("priv-student-talk"),
                                checked: f.student_talk,
                                on_change: move |v| flags.write().student_talk = v,
                            }
                        }
                        // Living as Christians
                        PrivSubSection {
                            title: t!("priv-sub-christians"),
                            color: "rgb(148, 41, 38)".to_string(),
                            PrivSwitch {
                                label: t!("priv-living-as-christians"),
                                checked: f.living_as_christians,
                                on_change: move |v| flags.write().living_as_christians = v,
                            }
                            PrivSwitch {
                                label: t!("priv-congregation-bible-study"),
                                checked: f.congregation_bible_study,
                                on_change: move |v| flags.write().congregation_bible_study = v,
                            }
                            PrivSwitch {
                                label: t!("priv-congregation-bible-study-reader"),
                                checked: f.congregation_bible_study_reader,
                                on_change: move |v| flags.write().congregation_bible_study_reader = v,
                            }
                        }
                    }

                    // ── Weekend Meeting ───────────────────────────────────
                    PrivCategorySection { title: t!("priv-cat-weekend"),
                        div { class: "grid grid-cols-2 sm:grid-cols-3 gap-x-4 gap-y-0.5",
                            PrivSwitch {
                                label: t!("priv-weekend-pray"),
                                checked: f.weekend_pray,
                                on_change: move |v| flags.write().weekend_pray = v,
                            }
                            PrivSwitch {
                                label: t!("priv-weekend-chairman"),
                                checked: f.weekend_chairman,
                                on_change: move |v| flags.write().weekend_chairman = v,
                            }
                            PrivSwitch {
                                label: t!("priv-watchtower-conductor"),
                                checked: f.watchtower_conductor,
                                on_change: move |v| flags.write().watchtower_conductor = v,
                            }
                            PrivSwitch {
                                label: t!("priv-public-talks"),
                                checked: f.public_talks,
                                on_change: move |v| flags.write().public_talks = v,
                            }
                            PrivSwitch {
                                label: t!("priv-public-talks-away"),
                                checked: f.public_talks_away,
                                on_change: move |v| flags.write().public_talks_away = v,
                            }
                        }
                    }

                    // ── Departments ───────────────────────────────────────
                    PrivCategorySection { title: t!("priv-cat-platform"),
                        div { class: "grid grid-cols-2 sm:grid-cols-3 gap-x-4 gap-y-0.5",
                            PrivSwitch {
                                label: t!("priv-stage"),
                                checked: f.stage,
                                on_change: move |v| flags.write().stage = v,
                            }
                            PrivSwitch {
                                label: t!("priv-audio"),
                                checked: f.audio,
                                on_change: move |v| flags.write().audio = v,
                            }
                            PrivSwitch {
                                label: t!("priv-video"),
                                checked: f.video,
                                on_change: move |v| flags.write().video = v,
                            }
                            PrivSwitch {
                                label: t!("priv-microphones"),
                                checked: f.microphones,
                                on_change: move |v| flags.write().microphones = v,
                            }
                            PrivSwitch {
                                label: t!("priv-attendant"),
                                checked: f.attendant,
                                on_change: move |v| flags.write().attendant = v,
                            }
                            PrivSwitch {
                                label: t!("priv-zoom-attendant"),
                                checked: f.zoom_attendant,
                                on_change: move |v| flags.write().zoom_attendant = v,
                            }
                            PrivSwitch {
                                label: t!("priv-territory"),
                                checked: f.territory,
                                on_change: move |v| flags.write().territory = v,
                            }
                            PrivSwitch {
                                label: t!("priv-cleaning"),
                                checked: f.cleaning,
                                on_change: move |v| flags.write().cleaning = v,
                            }
                            PrivSwitch {
                                label: t!("priv-maintenance"),
                                checked: f.maintenance,
                                on_change: move |v| flags.write().maintenance = v,
                            }
                        }
                    }

                    // ── Other ─────────────────────────────────────────────
                    PrivCategorySection { title: t!("priv-cat-other"),
                        div { class: "grid grid-cols-2 sm:grid-cols-3 gap-x-4 gap-y-0.5",
                            PrivSwitch {
                                label: t!("priv-hospitality"),
                                checked: f.hospitality,
                                on_change: move |v| flags.write().hospitality = v,
                            }
                            PrivSwitch {
                                label: t!("priv-interpreter"),
                                checked: f.interpreter,
                                on_change: move |v| flags.write().interpreter = v,
                            }
                            PrivSwitch {
                                label: t!("priv-field-service-meeting"),
                                checked: f.field_service_meeting,
                                on_change: move |v| flags.write().field_service_meeting = v,
                            }
                            PrivSwitch {
                                label: t!("priv-public-witnessing"),
                                checked: f.public_witnessing,
                                on_change: move |v| flags.write().public_witnessing = v,
                            }
                        }
                    }

                    div { class: "h-2" }
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
                        class: "flex-1 lg:flex-none px-4 lg:px-5 py-2.5 lg:py-2 text-sm bg-primary-600 text-white rounded-xl hover:bg-primary-700 disabled:opacity-50 transition-colors font-medium",
                        disabled: sub,
                        onclick: move |e| on_submit.call(e),
                        {if sub { t!("btn-connecting") } else { t!("btn-save") }}
                    }
                }
            }
        }
    }
}

// ── PrivCategorySection ───────────────────────────────────────────────────────

#[component]
fn PrivCategorySection(title: String, children: Element) -> Element {
    rsx! {
        div { class: "space-y-3",
            h3 { class: "text-xs font-semibold text-gray-400 uppercase tracking-wider px-0.5",
                "{title}"
            }
            {children}
        }
    }
}

// ── PrivSubSection ────────────────────────────────────────────────────────────

#[component]
fn PrivSubSection(title: String, color: String, children: Element) -> Element {
    rsx! {
        div { class: "space-y-1",
            p {
                class: "text-xs font-semibold uppercase tracking-wide px-0.5",
                style: "color: {color}",
                "{title}"
            }
            div { class: "grid grid-cols-2 sm:grid-cols-3 gap-x-4 gap-y-0.5 pl-2", {children} }
        }
    }
}

// ── PrivSwitch ────────────────────────────────────────────────────────────────

#[component]
fn PrivSwitch(label: String, checked: bool, on_change: Callback<bool>) -> Element {
    rsx! {
        div { class: "flex items-center gap-2 py-1.5 select-none",
            button {
                r#type: "button",
                class: if checked { "relative inline-flex h-5 w-9 shrink-0 rounded-full bg-primary-600 transition-colors duration-200 cursor-pointer focus:outline-none" } else { "relative inline-flex h-5 w-9 shrink-0 rounded-full bg-gray-300 transition-colors duration-200 cursor-pointer focus:outline-none" },
                onclick: move |_| on_change.call(!checked),
                span { class: if checked { "inline-block h-4 w-4 translate-x-4 transform rounded-full bg-white shadow transition-transform duration-200" } else { "inline-block h-4 w-4 translate-x-0.5 transform rounded-full bg-white shadow transition-transform duration-200" } }
            }
            span {
                class: "text-sm text-gray-700 leading-tight cursor-pointer",
                onclick: move |_| on_change.call(!checked),
                "{label}"
            }
        }
    }
}
