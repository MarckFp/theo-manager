# Theo Manager: Requirements and Architecture Decisions

> **Status:** Draft v0.4 (2026-11-16). This is the source of truth for every architecture decision.
> Change it through a reviewed commit, and add a line to the [Decision log](#14-decision-log).
>
> **In one paragraph:** Theo Manager manages a Jehovah's Witnesses congregation: people, roles,
> meeting assignments, duties, territories (with maps), public talks, field service groups and
> reports. It is a free Rust app with **no backend**. One Dioxus codebase targets Web, Windows,
> macOS, Linux, iOS and Android. All data is encrypted on the device before it is stored
> (zero-knowledge, end-to-end). A congregation runs either **Local** (one user, one device,
> unlocked with the congregation password) or **Cloud** (many users with their own accounts). In
> Cloud mode, full-access members unlock with the congregation password. Publishers get only the
> **key domains** their role allows, through a post-quantum key grant (section 6.2). People who do
> not use a device are managed by others. One database wrapper, `theo-db`, hides the engine:
> SQLite on native, IndexedDB in the browser, or a stock SurrealDB server for Cloud. The code is
> **source-available on GitHub** in one repository (a Cargo workspace of crates). There are no
> services and no secrets in the repository.

Keywords: **MUST** = required, **SHOULD** = expected unless there is a written reason, **MAY** =
optional. Requirement IDs (`R-xxx`) are stable. Never renumber them; mark them `(removed)` instead.

---

## Contents

1. [Product scope](#1-product-scope)
2. [Non-negotiable principles](#2-non-negotiable-principles)
3. [Technology decisions](#3-technology-decisions)
4. [System architecture](#4-system-architecture)
5. [Storage, access control and joining](#5-storage-access-control-and-joining)
6. [Cryptography specification](#6-cryptography-specification)
7. [Security requirements and threat model](#7-security-requirements-and-threat-model)
8. [UI and design system](#8-ui-and-design-system)
9. [Platforms, performance and size budgets](#9-platforms-performance-and-size-budgets)
10. [Repository and dependency management](#10-repository-and-dependency-management)
11. [Engineering practices and public-repo hygiene](#11-engineering-practices-and-public-repo-hygiene)
12. [Roadmap and de-risking spikes](#12-roadmap-and-de-risking-spikes)
13. [Questions](#13-questions)
14. [Decision log](#14-decision-log)
15. [Glossary](#15-glossary)
16. [Ideas from similar apps (to review)](#16-ideas-from-similar-apps-to-review)

---

## 1. Product scope

### 1.1 Platform requirements

| ID | Requirement |
|---|---|
| R-001 | The app MUST be lightweight: small downloads, fast start, low memory use. |
| R-002 | The app MUST have **no custom backend**. No server code is written, deployed or run by us. The only server-side component allowed is a **stock SurrealDB server** in Cloud mode. Its configuration is declarative only (schema, access rules, permissions). The app applies it itself (R-228). |
| R-003 | The app MUST run on **Web, Windows, macOS, Linux, iOS and Android** from one codebase. |
| R-004 | The app MUST be a single-page application (SPA) on every platform. |
| R-005 | The app manages one or more **congregations**. The modules are listed in 1.3. Each module gets its own data-modeling session before it is built (phase 3). |

### 1.2 Usage modes

| Mode | Who uses it | How the user gets in | Where the data lives |
|---|---|---|---|
| **Local** | One user manages the whole congregation on one device | Congregation password only | SQLite (native) or IndexedDB (web) on that device |
| **Cloud** | Several users share one congregation, on any device | **Account** (username + account password) to sign in. Then either the **congregation password** (full-access members) or a **key grant** (members with limited roles, for example publishers) decrypts the data | One SurrealDB database per congregation, encrypted |

- A user creates a congregation in Local or Cloud mode at setup.
- A Local congregation can move to Cloud later (R-205).
- Other users join a Cloud congregation through an **invite link** (section 5.6):
  - **Full-access members** (for example the coordinator or secretary) sign up, then type the
    congregation password. They get it through another channel.
  - **Members with limited roles** (for example publishers) only sign up. Their keys come from the
    invite, so they never need the congregation password.
- People who do not use a device have no account. Users with the right permissions manage their
  data for them (R-018).
- One install MAY hold several congregations and switch between them (R-016).

### 1.3 Domain modules (first draft, refined during data modeling)

| Module | Covers |
|---|---|
| **M-01 People** | Publishers and other members: names, contact and emergency contacts, household, baptism, appointments and privileges (elder, ministerial servant, pioneer types), field service group, active/inactive status, unavailability periods |
| **M-02 Users and access** | Accounts, invites, memberships, roles, permissions (section 5.5) |
| **M-03 Midweek meeting** | Life and Ministry meeting schedule per week: chairman, prayers, all parts, student assignments with assistants, auxiliary classrooms, Congregation Bible Study conductor and reader. Reads the events of each week (M-12, decision E1): no meeting in an assembly or convention week, the circuit overseer's visit, the Memorial |
| **M-04 Weekend meeting** | Chairman, public talk (link to M-06), Watchtower Study conductor and reader, prayers. Reads the events of each week (M-12, decision E1) |
| **M-05 Duties** | Audio, video and stage (with Zoom attendants and hosts), attendants, roving microphones, cleaning (groups and rota), Kingdom Hall maintenance. Each has its own page under *Departments* (D-28) |
| **M-06 Public talks** | Talk catalog (number and title), local and visiting speakers, incoming schedule, outgoing talks (local speakers to other congregations), directory of other congregations and their coordinators |
| **M-07 Territories** | Territory cards, boundaries, check-out and return, coverage history, campaigns |
| **M-08 Public witnessing** | Locations, carts, time slots, shift sign-ups and assignments |
| **M-09 Field service groups** | Groups, overseers and assistants, members |
| **M-10 Field service reports** | Monthly report per publisher, pioneer hours, late or missing reports, congregation totals for the monthly report |
| **M-11 Meeting attendance** | Attendance count per meeting (in person and by video conference) |
| **M-12 Events** | Circuit overseer visit, assemblies and conventions, Memorial, and special weeks that change the normal schedule |
| **M-13 Schedule output** | Printable and exportable schedules (PDF or image) for the information board and messaging apps |
| **M-14 Elderly care** | Help for older ones: transport to meetings, visits, shopping; helpers with the *Elderly care* privilege (proposal: `docs/model/m14-elderly-care.md`) |

### 1.4 Cross-cutting domain requirements

| ID | Requirement |
|---|---|
| R-010 | **Eligibility rules** decide who can take which part or duty, based on privileges, gender, baptism, availability and enrolment. They live in `theo-core`. The UI only offers eligible people. |
| R-011 | An **assignment helper** SHOULD suggest people based on eligibility and history (last time assigned, part type), to spread assignments fairly. It never assigns without confirmation. |
| R-012 | The app MUST warn about conflicts: two assignments for one person at the same meeting, or an assignment during the person's unavailability. |
| R-013 | Assignments, reports and territory coverage keep their **history**. Deletes are soft (tombstone) until purged (R-021). |
| R-014 | Sharing with people who have no account works through **export** (PDF, image, print). Public links would need a backend, so there are none. |
| R-015 | **Notifications:** with no backend, there is no push and no e-mail. v1 has a "My assignments" view. Mobile and desktop builds MAY add local reminders, computed on the device (Q13). |
| R-016 | One install MAY hold several congregations (vaults), each with its own mode and password. |
| R-060 | **Device profiles:** in Local mode, a user still has a profile (username + password) on the device. The profile lists its congregations and protects its personal keys, like a Cloud account but without a server. After sign-in, the app shows the user's congregations; with only one, it skips the choice and goes straight to unlock. |
| R-061 | The dashboard layout (order and size of widgets) is saved per user and per congregation, inside the encrypted data. |
| R-062 | **Display preferences:** date format (YYYY-MM-DD, MM-DD-YYYY, DD-MM-YYYY), time format (24-hour, 12-hour), first day of the week (Monday, Sunday), UI language (detected from the device locale; a list of supported translations) and name format (First Last, or Last, First). Each congregation has **defaults**; each user MAY **override** any of them in their profile. Effective value = user override, else congregation default. All dates, times and names in the app MUST go through these formatters. |
| R-063 | The **landing page exists only in the browser build**. Installed apps (Windows, macOS, Linux, iOS, Android) open on the log-in screen and show no "Home" links. |
| R-064 | **Sub-menus.** Each of the 5 main sections has pages. Desktop: the sidebar expands sections in place (accordion). Tablet: the icon rail opens a flyout. Phone: the dock opens the section's **hub** (large tiles); to change page, tap the active dock item again (it returns to the hub) and choose another page. There are no page chips, so pages keep their space for content (2026-11-17). The model is one list (`theo-app/src/nav.rs`). |
| R-065 | **All users** (*Users › All users*): a list of cards with search and 8 filters (status, type, appointment, group, activity, gender, anointed, consent). The list starts with status "Current". Values inside one filter combine with OR, filters combine with AND, and each value shows its result count. A card opens the person page (edit, delete, family, activity, and an invite in Cloud mode). Export of the filtered list as PDF, DOCX or CSV. Data model and rules: [`docs/model/m01-people.md`](docs/model/m01-people.md) and [`docs/model/m09-field-service-groups.md`](docs/model/m09-field-service-groups.md). |
| R-067 | **Moved away:** a person who moves away is marked with a date and listed on *Users › Moved away* (which replaces the Invitations page; invites are sent from the person page). They are kept for 1 year, then purged with their history (R-021). A family can move away together. Details: `docs/model/m01-people.md`. |
| R-068 | **Group rosters:** a member with an account sees only the names of the people in their own field service group, through a roster record encrypted with a per-group key. All other people data needs a permission. Details: `docs/model/m09-field-service-groups.md`. |
| R-069 | **Field service groups** (*Ministry › Field service groups*): card list with search, export and an alert for publishers without a group; a group page with name, overseer and assistant (baptised brothers only, listed as elders, ministerial servants, then others; a warning when not appointed), members added and removed at once (only publishers without a group can be added), and the group's activity. Export covers all groups only. Leading a group gives no access in the app; only role permissions do. Details: `docs/model/m09-field-service-groups.md`. |
| R-070 | **Field service reports** (*Ministry › Field service reports*): every current publisher reports the previous month. Publishers say whether they took part; pioneers and auxiliary pioneers give hours (pioneers also credit hours, at most 50); everyone gives Bible studies and may add notes. Goals: 600 h per service year (regular and special pioneers, missionaries), 30 h or 15 h per month for auxiliary pioneers, with the 15-hour months set per service year. Missing reports get a warning from the 5th and are late from the 15th. Monthly auxiliary pioneering is planned per person, ahead of time or once the month has started. A month view with a month picker, statistics per service year (congregation, groups, pioneers' goals and pace), reports on the person page and on the dashboard. The activity status (M-01) is derived from the reports. Details: `docs/model/m10-field-service-reports.md`. |
| R-071 | **Families** (*Users › All users*, tab *Families*; it was a sub-menu until 2026-11-18): cards of all families with search and filters (status, number of members, group). The family page changes the head, lists the members with their relationship to the head, adds people who have no family, removes members, and deletes the family (members stay). Details: `docs/model/m01-people.md`. |
| R-072 | **Privileges** (*Users › Privileges*): 33 privileges in five sections (both meetings, midweek, weekend, duties, ministry and care), each with a rule (baptised brothers; brothers including Bible students; baptised publishers; everyone except visitors). Visitors have none. Two views: by person (dialog per person) and by privilege (tick who has it), with the people filters plus a privilege filter. Privileges a person no longer qualifies for are removed on save, with a warning first. Details: `docs/model/m01-people.md`, "Privileges". |
| R-073 | **Permissions** (*Users › Permissions*) belong to the link between an account and a person (the membership), so only people who **joined the app** can have them; e-mail or contact details never count. Per sub-menu: no access, read or write (write includes read). Plus: write the reports of their own group, read their own group, read all groups, and administrator (all permissions; always the creator of the congregation; only administrators change permissions). Local congregations: one user with every permission. Details: `docs/model/m02-users-and-access.md`. This replaces the role model of 5.5 for v1; saved permission sets may come later. |
| R-074 | **Backup and restore** (the congregation *Settings* (cog in the congregation switcher)): one encrypted file with all the data of a congregation (Argon2id + XChaCha20-Poly1305 with the congregation password, `theo-crypto`); restoring replaces all data on the device, keeping the storage mode and current password. Both are logged. Details: `docs/design/local-sharing.md`, "Backup". |
| R-075 | **Congregation settings** are not a menu section: a cog next to the open congregation in the congregation switcher (sidebar and phone top bar) opens them (`/app/settings`). They have their own permission level (no access, read, write). |
| R-076 | **Emergency contacts**: a person has zero, one or more (first and last name, telephone, mobile phone, address, address line 2, postal code, city, notes), on the person page. All users filters people with and without emergency contacts. Details: `docs/model/m01-people.md`, "Emergency contacts". |
| R-077 | **Labels**: the congregation defines labels (name and colour); a person has any number of them. They show on the person cards and filter All users, Privileges and Moved away. Details: `docs/model/m01-people.md`, "Labels". |
| R-078 | **Absences** (*Users › Absences*): who (a person or the whole congregation) is away from which day to which day, with notes shown only in the absence; adding and deleting offers the same for family members; the same filters as All users plus now, coming up, past. Schedules will not assign people while they are away. Details: `docs/model/m01-people.md`, "Absences". |
| R-081 | **The person page saves automatically** (existing people), like its cards. Changes that remove privileges, roles or values wait for *Apply*. |
| R-082 | **Meeting attendance** (*Meetings › Meeting attendance*): per month, the number of people at each meeting, week 1 to week 5, for the midweek and the weekend meeting (in person and by video conference together), with meetings, total and average per row (average = total ÷ meetings held). A list of the months of a service year; statistics (averages compared with the year before, highest attendance, weekend average against publishers, a line chart per month, months ranked, highlights, the yearly record) and export. Details: `docs/model/m11-meeting-attendance.md`. |
| R-083 | **Events** (*Meetings › Events*): regional convention, circuit assembly, Memorial, circuit overseer's visit or custom, with a title, a description and first and last day; presets fill in the usual title, length and absence. For the whole congregation or for chosen people (then only they and the administrators see it). Optionally the event creates absences on its days (whole congregation, or each chosen person), which change and go with the event. A list with search and filters, and a month calendar. **Later (with M-03 and M-04): the meeting schedules read the events** (no midweek meeting in an assembly week, the circuit overseer's visit, the Memorial). Details: `docs/model/m12-events.md`. |
| R-084 | **Meetings for field service** (*Ministry › Meetings for field service*): date, time, place and conductor, as a list or a calendar per month. Weekly presets (day, time, place) with who can conduct each one (people with the privilege "Meeting for field service"). *Fill month*: fill from the presets, fill and assign, assign the empty ones, clear the month. The automatic assignment uses only people with the privilege, without an absence, available for the preset and without another meeting that day, spreads the meetings over the week and the month, and leaves a meeting empty with a warning when nobody is available. The conductor list shows each person's availability (available, not at this time, already conducts that day, away), then the other baptised publishers. Planners **send** a month: until then (and for changes after a send) everyone else sees only the last sent version. Sending notifies only the conductors concerned (new, changed, cancelled). In a preset nobody is available until ticked. Details: `docs/model/m15-field-service-meetings.md`. |
| R-085 | **My assignments** (home page widget, first by default): the signed-in person's upcoming assignments (only what was sent) and, on top, their notices of new, changed and cancelled assignments with *Got it*. *All my assignments* opens a full-page timeline (coming up and past, grouped by month). Field service meetings now; meeting parts and duties join with their modules. |
| R-089 | **Public witnessing** (*Ministry › Public witnessing*): shifts with date, start, end, place and people, as a list or a calendar. Optional **shift supervisors** (on or off, and who can be one). Weekly presets say how many supervisors, brothers, sisters and anyone a shift needs, and who is available (nobody until ticked); presets can be switched off. Fill, fill and assign, assign the empty places, clear; the assignment respects the privilege, gender, supervisors, absences, availability and one shift a day, and spreads shifts over the week and month. Send a month: only the people concerned get a notice; their shifts appear in *My assignments*. Statistics for the last 6 or 12 months. Details: `docs/model/m16-public-witnessing.md`. |
| R-090 | **Installable web app (PWA)**: the web build on GitHub Pages has a manifest, icons (made from the one icon source) and a service worker that keeps the app's files, so browsers offer "Install" and the installed app opens offline. Installed, it opens on the log-in screen like the native apps. Done when publishing (`scripts/web-postbuild.py`); `dx serve` is unchanged. |
| R-092 | **Stay unlocked** (per device, *Me › Security*): lock when the app is closed or reloaded (default), or stay unlocked for 5 or 15 minutes, 1 or 8 hours after the last use. The device remembers only which congregation is open and until when. ⚠️ MOCK: only the demo comes back after a reload, because nothing else is stored on the device yet. Notices marked *Got it* are remembered per device the same way. |
| R-091 | **Biometric unlock (UI level)**: on the web and the installed web app, turning it on creates a passkey (WebAuthn, the device's fingerprint, face or PIN) and unlocking asks for it; the choice is remembered on that device. It only checks that it is the user: no key is unlocked yet. Where passkeys are not possible (an IP address, the native apps for now) the check is simulated. |
| R-086 | **Home widgets** can be removed while customizing (×) and added back (*Add widget*); *Reset layout* restores the default. |
| R-087 | Every month and year picker has a **Today** button (shown when another month or year is open) that goes back to the current one. |
| R-088 | **The creator is a person**: the congregation wizard asks for your name and gender and adds you to All users, linked to your account, as administrator. Your card is tagged *You*. Deleting yourself removes your access, except on a Local congregation or when you are the only administrator. Details: `docs/model/m02-users-and-access.md`, "You are a person too". |
| R-079 | **Custom tasks** (*Departments › Custom tasks*): any other task (name, description) and the people who do it. Details: `docs/model/m01-people.md`, "Custom tasks". |
| R-080 | **Sticky list controls**: on list pages, the tabs, the search and the filters stay at the top while the list scrolls (All users and its Families tab, Privileges, Permissions, Moved away, Absences, Field service groups and reports, Custom tasks). The *Meetings › Attendance* sub-menu is called *Meeting attendance*, so it is not confused with *Departments › Attendants*. |
| R-066 | **Exports run on the device and are not encrypted.** The export menu says so before the user picks a format. CSV cells that start with `=`, `+`, `-` or `@` are prefixed with `'` (formula injection). |
| R-017 | **Self-service** for members with an account (all optional, controlled by permissions): see their own assignments and schedules; add and edit their own **absences** (away or on holiday), which block new assignments in that period (R-012); submit their own field service report; ask for a change of their own contact details (an admin applies it). |
| R-018 | **Managed people:** every self-service action MUST also be possible for a user with the right permission, on behalf of a person who has no account. The UI shows who entered the data ("entered by the secretary"). |
| R-019 | An account is linked to exactly one person per congregation. A person has at most one account. Linking happens through the invite (5.6). |

### 1.5 Personal data protection

| ID | Requirement |
|---|---|
| R-020 | The data is **sensitive personal data**. In the EU, religious affiliation is a special category under GDPR Art. 9. The app MUST store only the fields the congregation needs (data minimisation). |
| R-021 | The app MUST support, per person: **export** of all their data, and **erasure** (a real purge of envelopes and history, not only a tombstone). It MUST support configurable **retention** rules, for example deleting field service reports older than N years. Congregations align these settings with the data protection guidance they follow (to confirm per congregation). |
| R-022 | Development, tests, demos, screenshots, issues and AI-assisted work MUST use **synthetic data only**. Real congregation data never goes into a repo, an issue, a log or a prompt. |

### 1.6 Importing meeting content from jw.org

The [jw.org terms of use](https://www.jw.org/en/terms-of-use/) set three limits:
- They forbid tools that "scrape data, HTML, images, or text" from the site.
- They allow "free, non-commercial applications designed to download electronic files such as
  EPUB, PDF, MP3, and MP4 files from public areas of this site".
- They forbid distributing publication content "with or as part of any software application
  (including uploading such materials to a server for use by a software application)".

So the app downloads the **official EPUB files** and reads them **on each device**. A shared cloud
copy for each language would break the third rule, so there is none.

| ID | Requirement |
|---|---|
| R-030 | The app MUST import the meeting workbook and the Watchtower study articles from the **official EPUB files** that jw.org offers for download. It parses them on the device in Rust (`zip` + `quick-xml`). |
| R-031 | The app MUST NOT scrape HTML from jw.org or wol.jw.org. |
| R-032 | The app MUST NOT upload publication content to any server, and there is no cache shared between congregations. |
| R-033 | In Cloud mode, the congregation database stores only the **schedule structure**: week, part type, duration, assignees and a **reference** to the source (for example `mwb/2027-01/week-3/part-4`). Each device shows titles and texts from **its own cached EPUB**. This is about 6 workbook issues and 12 Watchtower study issues per language per year, per device. (Q17) |
| R-034 | Downloads start only when a user imports, or when the user turns on a monthly check. EPUBs are cached on the device (files on native, IndexedDB on web). Requests identify the app with its User-Agent and respect HTTP caching. There are no repeated or parallel bulk requests. |
| R-035 | **Import from file** MUST always work: the user downloads the EPUB from jw.org and selects it in the app. This is the fallback when the web build cannot download directly because of browser CORS rules (spike S8). |
| R-036 | A congregation chooses one or more publication languages (jw.org language codes). |
| R-037 | Users can always edit parts by hand: special weeks, the circuit overseer visit, assemblies, or when no EPUB is available. |
| R-038 | The app MUST stay **free and non-commercial**, because the jw.org exception only covers such apps. |

### 1.7 Maps (territories)

| ID | Requirement |
|---|---|
| R-040 | The map engine MUST be pluggable. A `TileProvider` has a name, URL template, attribution, maximum zoom and an optional API key. API keys are runtime input, stored encrypted in the congregation settings, never in a repo (R-701). |
| R-041 | The default provider is **OpenStreetMap** standard tiles, used under the [OSMF Tile Usage Policy](https://operations.osmfoundation.org/policies/tiles/): visible attribution "© OpenStreetMap contributors", an app-specific User-Agent on native (`TheoManager/<version> (+<repo URL>)`), a valid Referer on web, HTTP caching honoured (at least 7 days), at most 2 parallel connections. |
| R-042 | The app MUST NOT offer "download this area for offline use" with OpenStreetMap tiles, because the policy forbids bulk downloads. Offline maps are possible only with a provider that allows them, for example a self-hosted **PMTiles** file. |
| R-043 | Other providers (for example a self-hosted tile server, PMTiles, or commercial providers with the user's own API key) MUST be selectable per congregation. |
| R-044 | Territory boundaries are polygons inside encrypted records. They are never sent to the tile provider. The app MUST import and export **GeoJSON** and **KML**. |
| R-045 | **Privacy notice:** viewing a map tells the tile provider which area is viewed, and from which IP address. The app shows this once, when an admin turns on maps. Maps can be turned off. |
| R-046 | The map is a Rust `MapView` component in `theo-ui`: a raster tile grid, an SVG layer for boundaries, pan and zoom with mouse and touch, and polygon drawing and editing. No JS map library (P1). Spike S9. **Built** (2026-11-20, `theo-ui/src/map.rs`): areas, lines and markers, pinch and wheel zoom. Exception to P1: the territory card PDF reads the tiles through a short canvas script (`platform::render_tiles`), the only way to read tile pixels inside the webview. |

---

## 2. Non-negotiable principles

| # | Principle | What it means in practice |
|---|---|---|
| P1 | **100% Rust** | All hand-written code is Rust. Generated glue (wasm-bindgen output, the Dioxus JS interpreter) is accepted. Styles are CSS (Tailwind CSS v4 + daisyUI 5). Hand-written JS needs an approved exception here. |
| P2 | **Private by design** | No telemetry, no analytics, no CDN fonts or scripts. Only **two** kinds of outside requests are allowed, and both are visible to the user: **map tiles** (1.7) and **jw.org EPUB downloads** (1.6). Neither ever carries congregation data. |
| P3 | **Zero-knowledge E2E encryption** | Plaintext and keys never leave the device. A server, a stolen disk or a database dump shows only ciphertext and the metadata listed in 7.3. |
| P4 | **Lose the password, lose the data** | There is no server-side recovery. Optional, user-held recovery is covered in R-311. |
| P5 | **Post-quantum ready** | All primitives are quantum-resistant, or hybrid classical + post-quantum (section 6.1). |
| P6 | **One look everywhere** | Every button, form, menu and dialog comes from one component library (`theo-ui`). |
| P7 | **One repository, no services** | One repository with a Cargo workspace of crates (D-25). Only `theo-app` produces a deployable artifact. |
| P8 | **Public code, zero secrets** | All code is public on GitHub. No repo ever holds a credential, key or `.env` file (section 11.2). |
| P9 | **Source-available, not redistributable** | Anyone may read, clone, fork and contribute. Nobody else may publish copies (10.4). The app is free. |

---

## 3. Technology decisions

### 3.1 UI framework: **Dioxus 0.7** (decision D-01)

| Criterion | **Dioxus 0.7** ✅ | Tauri 2 + Leptos | Tauri 2 + Yew | GPUI |
|---|---|---|---|---|
| Web | ✅ WASM | ✅ WASM | ✅ WASM | ❌ no production web target |
| Windows / macOS / Linux | ✅ (system webview) | ✅ (system webview) | ✅ | ✅ (native GPU) |
| iOS / Android | ✅ via `dx` CLI (tier 2) | ✅ Tauri mobile | ✅ Tauri mobile | ❌ |
| Where app logic runs on native | **Native Rust**, in the same process as the UI | WASM in the webview; native code only through IPC commands | Same as Leptos | Native |
| Build targets for one feature | 1 crate, platform chosen by feature flag | 2 (frontend WASM + Tauri core) + an IPC contract | 2 | 1 |
| Embedded DB on native | Direct native calls (SQLite) | Behind IPC (acts like a local "backend") | Behind IPC | Direct |
| CSS / daisyUI support | ✅ built-in Tailwind support in `dx` | ✅ | ✅ | ❌ (no CSS) |

**Why Dioxus:**
1. It is the only option where one Rust crate covers all six platforms with one toolchain (`dx`).
2. On native platforms, the app logic, crypto and database run as **native code**, not WASM. Argon2
   and SQLite run at full speed, and there is no IPC layer.
3. The web build uses WASM + IndexedDB, behind the same `theo-db` API.

**Rejected:**
- **GPUI:** see "Dioxus vs GPUI" below.
- **Yew:** heavier and less active than Leptos, with no advantage here.
- **Tauri + Leptos:** a strong second choice, with slightly smaller WASM. It needs two compile
  targets and an IPC contract, and it pushes native DB access behind IPC.

**Accepted risks:**
- Dioxus mobile is **tier 2**. Spike S1 builds the app shell on iOS and Android first.
- Dioxus is before 1.0 (0.8 is in alpha). We pin the minor version and upgrade all repos together.

#### Dioxus vs GPUI (re-checked 2026-11-16)

**What runs where.** Dioxus does **not** run the app in WebAssembly on native platforms:

| Platform | Dioxus: app logic (Rust) | Dioxus: drawing | GPUI: drawing |
|---|---|---|---|
| Windows, macOS, Linux | Native machine code | System webview (WebView2, WKWebView, WebKitGTK). The webview composites on the GPU. | Own GPU renderer (Metal, DirectX, Vulkan) |
| iOS, Android | Native machine code | System webview | ❌ Not supported upstream. Only a third-party `gpui-mobile` crate (0.1, early development) |
| Web | WASM | Browser DOM | ❌ Not supported upstream. Only a fork that needs WebGPU (Chrome or Edge only) |

**Where GPUI is really faster:** drawing very large, fast-changing screens, such as a code editor
with 100,000 lines at 120 fps. It also has a faster start and lower memory use than a webview.

**Why that does not matter much here:**
- A congregation has a few hundred people and a few thousand records per year.
- Our screens are forms, lists, schedule grids and one map. A webview draws these easily at 60 fps.
- The heavy work (Argon2, encryption, SQLite, search) already runs as native code in Dioxus.

**What GPUI would cost us:**
- No production web or mobile target. Three of our six platforms would rest on forks or pre-alpha
  crates.
- No CSS, so no Tailwind or daisyUI. We would build the whole design system ourselves.
- Accessibility (screen readers), text input methods and mobile keyboards are much less mature in
  custom renderers than in webviews (to confirm per platform).
- A hybrid (GPUI on desktop, Dioxus on web and mobile) means **two UI codebases**. That breaks R-003
  and the consistency rules in 8.4.

**Conclusion: stay with Dioxus.** The real costs of the webview are memory use (about 80–150 MB),
start time, and weaker WebKitGTK performance on Linux. R-104 and R-105 control them.

**Exit path, if performance ever becomes a problem:** **Dioxus Native**. It is the same Dioxus
components and CSS, drawn on the GPU (Blitz layout + Vello on wgpu), with no webview. It is
experimental today: the Blitz README calls it pre-alpha, and CSS support is incomplete. Because
`theo-ui` owns every component (8.4), switching the renderer later would affect `theo-ui` only, not
the screens.

| ID | Requirement |
|---|---|
| R-101 | The app MUST use Dioxus, pinned to one minor version (currently 0.7.x) across all repos. |
| R-102 | Platform-specific code MUST stay behind `cfg(target_arch = "wasm32")`, `cfg(target_os = ...)` or Cargo features, in adapter modules only. Domain code MUST be platform-agnostic. |
| R-103 | The app MUST NOT use Dioxus "fullstack" or server functions, because that would be a backend (R-002). |
| R-104 | **UI performance budget**, measured in spike S1 and then in CI on the gallery: scrolling a `DataTable` with 2,000 rows and switching weeks in a `ScheduleGrid` stay at **60 fps** on a mid-range Android phone and on Linux (WebKitGTK); input delay ≤ 50 ms. `DataTable`, `DataList` and `ScheduleGrid` MUST virtualize long lists (draw only the visible rows). |
| R-105 | **Renderer review:** if R-104 or the memory and start-time budgets (9.2) cannot be met in the webview, we evaluate Dioxus Native for the failing platform before anything else. The renderer is re-checked at every Dioxus minor upgrade. |

### 3.2 Styling: **Tailwind CSS v4 + daisyUI 5 (standalone, no Node.js)**, wrapped by `theo-ui` (D-02)

- daisyUI 5 runs with the **Tailwind standalone CLI**, without Node.js or npm. You download
  `daisyui.mjs` and load it with `@plugin`.
- daisyUI has the layout parts we need: `drawer` + `menu` for the desktop **sidebar** and `dock` for
  the mobile **dock**. It also has `btn`, `input`, `fieldset`, `modal` (native `<dialog>`),
  `dropdown`, `tabs`, `toast`, `table` and `calendar` styles.
- It is CSS only, with no JS runtime.
- A **custom daisyUI theme** (`theo-light`, `theo-dark`) gives the neutral, slick **shadcn-style**
  look.
- Tailwind v4 scans our `.rs` sources (`@source`) and keeps only the classes we use.

**Rejected:**
- **Dioxus Components / shadcn ports:** they mean copy-pasting component sources, and the ecosystem
  is younger.
- **Plain hand-written CSS:** too slow to build a consistent design system.
- **SASS/LESS:** no gain over Tailwind v4, which has native CSS variables, nesting and layers.

### 3.3 One database wrapper: **`theo-db`** (D-03, D-04, D-07)

The app talks to **one API**. How you open the database decides the engine:

```rust
// Same `Db` type and the same methods in every case.
let db = Db::open(DbConfig::LocalFile    { path }).await?;                       // native → SQLite
let db = Db::open(DbConfig::LocalBrowser { name }).await?;                       // web    → IndexedDB
let db = Db::open(DbConfig::Cloud        { url, namespace, database }).await?;   // all    → SurrealDB server
let db = Db::open(DbConfig::Memory).await?;                                      // tests
```

Every record is encrypted before it reaches `theo-db`, so **the database never queries plaintext**.
All search, filter and sort runs on the device after decryption. A congregation has a few hundred
people and a few thousand records per year, so in-memory search is fast. The database only needs
to store versioned, encrypted records reliably.

| Config | Platform | Engine | Implementation |
|---|---|---|---|
| `LocalFile` | Windows, macOS, Linux, iOS, Android | **SQLite** (WAL mode) | `rusqlite` with `bundled` |
| `LocalBrowser` | Web | **IndexedDB** | **Default:** direct adapter on `web-sys`/`rexie` (adds almost 0 KB). **Alternative:** SurrealDB embedded with the `kv-indxdb` engine. Spike S3 compares size and reliability, then we pick one. The `Db` API does not change. |
| `Cloud` | All | **SurrealDB server** over `wss://` | `surrealdb` SDK with **only** the remote WS protocol (no embedded engine), or a thin JSON-RPC client if the SDK breaks the size budget (S3) |
| `Memory` | All | In memory | For tests and the UI gallery |

**Why direct IndexedDB is the default for the web:** the SurrealDB embedded engine adds several MB
of WASM, and its IndexedDB engine has had stability problems. Its query language brings no value on
ciphertext. If S3 shows it is small and stable enough, we can switch, because the API hides the
engine.

**Why SurrealDB for Cloud:**
- Record-level auth (`DEFINE ACCESS ... TYPE RECORD`) and row permissions let the app connect
  directly and safely, with no API layer to write.
- Live queries push changes to other devices.
- It is open source and self-hostable. The server uses BSL 1.1, which allows self-hosting. We do
  not redistribute it.

**`theo-db` also owns the SurrealDB setup.** There is no separate cloud repo:
- `theo-db/surreal/migrations/*.surql`: schema, access rules, permissions, events.
- `Db::provision(...)` applies them (R-228).
- `theo-db/deploy/`: example `docker-compose.yml` and `Caddyfile` with placeholders only.

| ID | Requirement |
|---|---|
| R-201 | The mode MUST be chosen **at runtime per congregation** (Local or Cloud), at setup. |
| R-202 | Each engine MUST sit behind a **Cargo feature** (`sqlite`, `indexeddb`, `surreal-remote`, `memory`). A build only includes what it needs. Opening a config whose feature is off returns a clear error. |
| R-203 | All engines MUST pass the same **conformance test suite**. |
| R-204 | `theo-db` MUST only receive **encrypted envelopes** and the metadata in 5.1. This is enforced by types: `theo-db` does not depend on `theo-crypto` and cannot see plaintext types. |
| R-205 | Moving a congregation from Local to Cloud (or back) MUST work by exporting and importing envelopes. No re-encryption is needed. |
| R-206 | A future **Sync** mode (Local + Cloud, offline-first) SHOULD be possible without changing the envelope format. The `version` and tombstone fields exist for this. |

### 3.4 Cryptography stack (D-05)

Only well-known, pure-Rust, WASM-compatible crates. **No custom primitives.**

| Purpose | Algorithm | Crate (preferred → fallback) |
|---|---|---|
| Password → key | **Argon2id** (RFC 9106) | `argon2` (RustCrypto) |
| Key derivation | **HKDF-SHA-256** | `hkdf` + `sha2` |
| Data encryption (AEAD) | **XChaCha20-Poly1305**, 256-bit keys | `chacha20poly1305` |
| Hybrid post-quantum KEM | **X-Wing** (X25519 + **ML-KEM-768**, FIPS 203) | `libcrux-ml-kem` (formally verified) → RustCrypto `ml-kem` / `x-wing` |
| Hybrid post-quantum signatures | **ML-DSA-65** (FIPS 204) + **Ed25519** | `libcrux-ml-dsa` → RustCrypto `ml-dsa`; `ed25519-dalek` |
| Hashing / MAC | SHA-256, HMAC-SHA-256 | `sha2`, `hmac` |
| Randomness | OS CSPRNG / `crypto.getRandomValues` | `getrandom` (`wasm_js` backend on web) |
| Passphrase generation | EFF large wordlist (7,776 words) | wordlist compiled into `theo-crypto` |
| Password strength | zxcvbn | `zxcvbn` |
| Secret hygiene | zeroize on drop, constant-time compare | `zeroize`, `secrecy`, `subtle` |

The final choice between libcrux and RustCrypto for ML-KEM and ML-DSA depends on spike S4.

---

## 4. System architecture

```text
 ┌──────────────────────────────── theo-app (Dioxus, 1 crate, 6 platforms) ─────────────────────────┐
 │  Routes / screens / state (signals) / module UIs (M-01 … M-13)                                   │
 │        │ uses                         │ uses                                                     │
 │        ▼                              ▼                                                          │
 │  ┌──────────────┐            ┌─────────────────────────────────┐                                 │
 │  │   theo-ui    │            │            theo-core            │  domain models, eligibility,    │
 │  │ components + │            │  congregation/session lifecycle,│  permissions model, search,     │
 │  │ theme (CSS)  │            │  encrypt-before-store           │  conflict handling              │
 │  └──────────────┘            └──────┬────────────────┬─────────┘                                 │
 │                                     │ uses           │ uses (Db API, envelopes only)             │
 │                                     ▼                ▼                                           │
 │                              ┌──────────────┐  ┌──────────────────────────────────────────┐      │
 │                              │ theo-crypto  │  │ theo-db  (one API, engine by DbConfig)   │      │
 │                              │ KDF, AEAD,   │  │  sqlite │ indexeddb │ surreal-remote │ mem│      │
 │                              │ PQ KEM/sig   │  └────┬─────────┬───────────┬───────────────┘      │
 │                              └──────────────┘       │         │           │                      │
 └─────────────────────────────────────────────────────┼─────────┼───────────┼──────────────────────┘
                                                        ▼         ▼           ▼  wss:// (TLS 1.3, hybrid PQ key exchange)
                                                  SQLite file  IndexedDB   ┌──────────────────────────────┐
                                                  (native)     (browser)   │ Caddy (TLS, rate limits)     │
                                                                           │ SurrealDB (stock server)     │
                                                                           │ 1 database per congregation  │
                                                                           │ schema applied by the app    │
                                                                           └──────────────────────────────┘
```

**Write path:**
1. A UI event calls a `theo-core` use case.
2. The use case checks permissions and eligibility, and validates the object.
3. It serializes the object (CBOR) and pads it.
4. It encrypts it with the congregation key (AEAD, bound to the record identity).
5. It calls `Db::put(envelope)`. The engine stores the envelope.

**Read path:** `Db` returns envelopes. `theo-core` decrypts them, deserializes them and updates the
in-memory index. The UI signals then update.

| ID | Requirement |
|---|---|
| R-121 | `theo-core` and `theo-crypto` MUST NOT depend on Dioxus, `web-sys` or any async runtime. They MUST compile for `wasm32-unknown-unknown` and all native targets. |
| R-122 | Async APIs MUST NOT require `Send` futures on `wasm32`. |
| R-123 | Blocking work MUST NOT freeze the UI. On native, SQLite and Argon2 run on a blocking thread pool. On web, the UI shows a progress state during Argon2. A Web Worker MAY be used later. |
| R-124 | Decrypted data MUST stay in memory only. It is never written to disk, logs, caches or browser storage. |

---

## 5. Storage, access control and joining

### 5.1 What every engine stores (logical schema)

```text
vault_header                       (one per congregation; Cloud: readable only by full-access members)
  congregation_id   : 128-bit random
  format_version    : u16
  kdf               : { alg: "argon2id", m_kib, t, p, salt (16 B random) }
  wrapped_root      : envelope (root key under the congregation KEK)
  wrapped_recovery  : optional envelope (root key under the recovery key, R-311)

keyring                            (one row per key domain and version, 6.2)
  domain            : "schedule" | "people" | "reports" | "territories" | ... (plaintext)
  key_version       : u32
  public_keys       : X-Wing public key of the domain (for sealed submissions)
  wrapped_secret    : envelope (domain secret under the root key)

grant                              (Cloud only; readable only by its member)
  member            : account ID
  domain, key_version
  sealed_secret     : domain secret, encapsulated to the member's X-Wing public key

member_keys                        (Cloud only; public part readable by members, private part only by the owner)
  member            : account ID
  public_keys       : X-Wing + ML-DSA-65 + Ed25519 public keys
  wrapped_private   : envelope (private keys under the member's account KEK)

record
  congregation_id   : 128-bit
  collection        : module name in plaintext, e.g. "people", "midweek", "fs_report" (needed for server permissions, R-242)
  scope_id          : optional 128-bit random ID of a group or person (needed for "own"/"own group" rules, R-242)
  record_id         : 128-bit random (not UUIDv7: it leaks the creation time)
  version           : u64 (increments on every write; optimistic concurrency)
  updated_at        : i64, rounded to the minute (set by the server in Cloud mode)
  author            : account ID (Cloud only, set by the server)
  deleted           : bool (tombstone; the envelope is removed on delete)
  envelope          : bytes (section 6.4; says which domain and key version encrypted it)
```

Each collection belongs to exactly one key domain. The mapping is fixed in `theo-core` (for example
`fs_report` → `reports`).

### 5.2 `Db` API (in `theo-db`)

```rust
impl Db {
    pub async fn open(config: DbConfig) -> Result<Db, DbError>;
    pub async fn sign_up(&self, invite: InviteToken, username: &str, auth_key: AuthKey) -> Result<Session, DbError>; // Cloud only
    pub async fn sign_in(&self, username: &str, auth_key: AuthKey) -> Result<Session, DbError>;                      // Cloud only
    pub async fn provision(url: Url, owner: OwnerCredentials, congregation: NewCongregation) -> Result<(), DbError>; // Cloud only, R-228

    pub async fn read_header(&self) -> Result<Option<VaultHeader>, DbError>;
    pub async fn write_header(&self, header: &VaultHeader, expected_version: Option<u64>) -> Result<(), DbError>;
    pub async fn put(&self, rec: &EncryptedRecord, expected_version: Option<u64>) -> Result<u64, DbError>; // Err(Conflict) on mismatch
    pub async fn get(&self, collection: &Collection, id: RecordId) -> Result<Option<EncryptedRecord>, DbError>;
    pub async fn list(&self, collection: &Collection, since_version: u64) -> Result<Vec<EncryptedRecord>, DbError>;
    pub async fn delete(&self, collection: &Collection, id: RecordId, expected_version: u64) -> Result<(), DbError>; // tombstone
    pub async fn purge(&self, collection: &Collection, id: RecordId) -> Result<(), DbError>;                      // erasure, R-021
    pub fn subscribe(&self) -> Option<ChangeStream>; // Cloud: live query. Local: None.
}
```

The `Db` API speaks in envelopes and opaque keys only. `theo-core` builds the domain API on top.

### 5.3 Cloud mode on SurrealDB

| ID | Requirement |
|---|---|
| R-221 | **One SurrealDB database per congregation.** This gives strong isolation, simple permission rules, and easy export or deletion of a whole congregation. The schema is `SCHEMAFULL`, with the tables `account`, `member_keys`, `membership`, `role`, `invite`, `vault_header`, `keyring`, `grant`, `record` and `audit`. |
| R-222 | Sign-in MUST use `DEFINE ACCESS account ON DATABASE TYPE RECORD` with `SIGNUP` / `SIGNIN`. The value sent as "password" is the derived **auth key** (6.2), never the account password. The server stores `crypto::argon2::generate(auth_key)`. |
| R-223 | Every table MUST have `PERMISSIONS` based on the caller's active membership and role (section 5.5). Without a membership, a session can read nothing. |
| R-224 | Writes MUST use optimistic concurrency (`version` check). On a conflict, the client re-fetches, re-applies the change and retries. If a merge is not possible, the user decides. |
| R-225 | The server MUST sit behind a reverse proxy (Caddy). The proxy gives TLS 1.3 with hybrid post-quantum key exchange (`X25519MLKEM768`), rate limits on sign-in and sign-up, and request size limits. |
| R-226 | Schema changes MUST be versioned `.surql` migration files in `theo-db`. The app applies them in order and records the applied version in the database. |
| R-227 | Server-side `DEFINE EVENT`s write an **audit log** with metadata only: who changed which record, and when. |
| R-228 | **Provisioning from the app.** The first admin enters the server URL and the SurrealDB owner credentials once, in the app. The app creates the namespace and database and applies the migrations. It then **forgets the owner credentials**: they are never stored or logged. |

### 5.4 Search and querying

| ID | Requirement |
|---|---|
| R-231 | Search, filter and sort MUST run on the device, over decrypted data in memory. |
| R-232 | **Blind indexes** (HMAC-SHA-256 of normalized terms, with a key derived from the congregation key) MAY be added per field, on opt-in only. They leak equality between values, so each use MUST be documented. |

### 5.5 Roles and permissions

Access control has **two layers**, with different jobs:

| Layer | Protects against | Enforced by |
|---|---|---|
| **E2E encryption with key domains** | Server operator, database leaks, network attackers. Also members reading **domains** they hold no key for | Crypto in `theo-core` (6.2) |
| **Access control** (roles and permissions) | Members reading or changing **records** beyond their role *inside* a domain they hold, for example another group's reports | SurrealDB permissions (Cloud) + the UI (all modes) |

**Model** (first proposal; for v1 it is replaced by permissions per account, R-073 and `docs/model/m02-users-and-access.md`):
- A **permission** is `<module>.<action>` with a **scope**. Actions: `read`, `write`, `manage`.
  Scopes: `all`, `own_group`, `own`. Example: `fs_report.read@own_group` lets a field service
  group overseer read the reports of their group.
- A **role** is a named set of permissions. Admins can edit roles. The app ships **role templates**
  as a starting point: Admin, Coordinator, Secretary, Service overseer, Life and Ministry overseer,
  Public talk coordinator, Duties coordinator, Territory servant, Field service group overseer,
  Publisher.
- An **account** is a login. A **person** (M-01) is a member of the congregation. A membership
  links an account to a person and gives it roles. People without accounts are normal.
- **Privileges** (elder, ministerial servant, pioneer) are domain data on the person. They drive
  eligibility (R-010). They do not grant app permissions. Roles do.
- Each role also lists the **key domains** it can read. The app grants exactly those keys (6.2).
  The **Publisher** template, for example, reads `schedule` only. It can *submit* to `reports` and
  `people` with sealed records (R-327) without being able to read those domains.

| ID | Requirement |
|---|---|
| R-241 | In Cloud mode, the **server MUST enforce read and write permissions** per collection and scope. A member who knows the congregation password still cannot download collections that their role does not allow. |
| R-242 | To make R-241 possible, records carry `collection` in plaintext and an optional opaque `scope_id` (5.1). This is a deliberate metadata trade-off (7.3). Names and content stay encrypted. |
| R-243 | The server sets `author` and `updated_at` itself (`VALUE $auth.id`, `time::now()`), so clients cannot fake them. |
| R-244 | In Local mode, the single user has full rights. Roles still exist in the data, ready for a move to Cloud, but are not enforced. |
| R-245 | The UI hides or disables actions that the user's permissions do not allow. It uses the same permission model from `theo-core` (one source of truth). |
| R-246 | **Removing a member, or taking away a role:** the admin revokes the membership or role, and the server denies access at once. The admin's app then **rotates the key domains** that member held. New data uses the new key version, which is granted only to the remaining holders. If the member had full access, the app also asks for a **new congregation password**. The UI explains that data already on the removed person's device cannot be called back. |
| R-247 | **Cryptographic read separation is part of v1** (D-12). Each key domain has its own key. A member receives only the domain keys of their roles. A member without the `reports` key cannot decrypt reports, even if the server leaks them. |
| R-248 | Changes to keys (grants, rotation) happen on the device of a full-access member, because only such a device holds the keys. The app MUST do them automatically when an admin changes roles. If the admin is offline, the change waits until their app is online. |

### 5.6 Joining a Cloud congregation

There are two kinds of invite. Both start the same way.

```text
Admin (in app)                          New user                                   SurrealDB
──────────────                          ────────                                   ─────────
1. Create invite: pick the person (M-01),
   the roles, expiry (7 days), single use.
   App makes a random 256-bit token T.
   Member invite only: app wraps the role's
   domain keys under HKDF(T) = "invite bundle". ─────────────────────────────────► stores SHA-256(T) + bundle
2. Send the link privately:
   https://<app-host>/join#s=<server>&ns=<ns>&db=<db>&t=<T>
                                        3. Opens the link (web, or the native app
                                           through universal/app links).
                                        4. Picks username + account password.
                                           App derives auth_key and creates
                                           the member's PQ keypairs.     ─────────► SIGNUP checks T: exists, not expired,
                                                                                     not used. Creates account + membership
                                                                                     linked to the person. Marks T used.
MEMBER INVITE (for example publishers)
                                        5. App unwraps the bundle with T, turns it
                                           into grants for the member's own keys. ──► stores grants, deletes the bundle
                                           Done: no congregation password needed.

FULL-ACCESS INVITE (for example the coordinator or secretary)
3b. Tell the congregation password
    in person (never in the link).
                                        5. Types the congregation password.
                                           App unwraps the root key and grants
                                           itself all domain keys.         ─────────► stores grants
```

| ID | Requirement |
|---|---|
| R-251 | Invite data MUST be in the URL **fragment** (`#...`), so web hosts never see it in their logs. |
| R-252 | The invite link MUST NOT contain the congregation password or the root key. A member-invite link unlocks only the domain keys of that invite's roles, so it MUST be sent privately (in person, or in a private message to that person). |
| R-253 | Invites MUST be single-use and expire (default 7 days). The server stores only the token hash. An admin can revoke an open invite, and the bundle is then deleted. |
| R-254 | An admin MAY require approval. The invite then has no bundle, and the grants are made when an admin approves (R-248). |
| R-255 | If a member sees "invite already used", the app tells them to contact an admin. The admin revokes that membership and rotates the affected domains (R-246). |
| R-256 | The congregation password works only for memberships with **full access**. The server lets only those memberships read `vault_header` (R-223). |
| R-257 | A member who forgets their **account password** loses only their account, not data. An admin sends a new invite for the same person. |

---

## 6. Cryptography specification

### 6.1 Where "post-quantum" applies

- **Symmetric crypto is already quantum-resistant.** Grover's algorithm only halves the security
  of a key. A 256-bit XChaCha20 key still gives about 128-bit security against a quantum attacker.
  Argon2id, HKDF and SHA-256 are in the same position.
- **Public-key crypto is what quantum computers break** (X25519, Ed25519, RSA). Wherever we use
  public-key crypto, we use **hybrid** constructions: classical + NIST PQC (ML-KEM-768,
  ML-DSA-65). Security then holds as long as *either* part is unbroken.
- Post-quantum public-key crypto is used **from v1 (phase 2)**:
  - **X-Wing** (X25519 + ML-KEM-768) gives key domains to members (grants).
  - It also lets members submit sealed records to domains they cannot read (R-327).
  - Phase 4 adds hybrid **signatures** (ML-DSA-65 + Ed25519).

### 6.2 Secrets and key hierarchy

| Secret | Who knows it | Used for | What the server sees |
|---|---|---|---|
| **Congregation password** | The Local user; in Cloud, only **full-access** members (shared in person) | Unlocks the **root key**, which unlocks every key domain | Only the salt and the wrapped root key |
| **Account password** | One user (Cloud only) | Sign-in; unlocks the member's personal keys, and through their grants, their domain keys | Only `argon2(auth_key)` and the wrapped personal keys |

```text
CONGREGATION ROOT  (Local and Cloud)
congregation password ──Argon2id(salt_c: 16 B random)──► congregation_kek   (never stored)
   └── unwraps ──► root_key  (32 B random)
                     └── unwraps the KEYRING: one entry per key domain and key version
                           domain "schedule"    meetings, duties, public talks, public witnessing, calendar
                           domain "people"      contact details, addresses, privileges, notes
                           domain "reports"     field service reports, attendance
                           domain "territories" territory cards, boundaries, assignments
                           (final list in the M-02 modeling session)
                         each domain = 32 B secret ──HKDF──► record_key, blind_index_key, X-Wing keypair

ACCOUNT  (Cloud only, one per user)
account password ──Argon2id(salt_a)──► account_master   (never stored)
   ├──HKDF "theo/v1/auth"────────► auth_key     → SurrealDB SIGNIN (server stores argon2(auth_key))
   └──HKDF "theo/v1/account-kek"─► account_kek  → unwraps the member's personal keys
                                                  (X-Wing, ML-DSA-65 + Ed25519)
salt_a = SHA-256("theo/v1/auth-salt" ‖ server_origin ‖ namespace ‖ database ‖ normalized_username)

GRANTS  (Cloud)
for each domain that the member's roles can read:
   domain secret ──X-Wing encapsulate to member's public key──► grant row (only that member can read it)
```

- **Day-to-day sign-in** in Cloud mode needs only the account password, on any device.
  - It unwraps the personal keys, then the grants, then the member's domain keys.
  - Full-access members also hold grants for every domain. They need the congregation password only
    to join, or to recover.
- HKDF is one-way. The server knows `auth_key`, but it cannot compute `account_kek` from it.
- `salt_a` is deterministic, because the client needs it before sign-in. Bitwarden uses the same
  approach. `salt_c` is random and stored in `vault_header`.
- **Changing the congregation password** re-wraps `root_key` only.
- **Rotating a domain** creates a new key version. New writes use it. Old records MAY be
  re-encrypted in the background.

| ID | Requirement |
|---|---|
| R-316 | Every key domain MUST be versioned. Readers keep old versions, so they can still read old records. |
| R-317 | Because the server stores the wrapped personal keys, a malicious operator could try account passwords offline. Argon2id plus R-315 makes that expensive. The account password protects only that member's domains. |

| ID | Requirement |
|---|---|
| R-314 | The app MUST **generate** the congregation password by default, as a 6-word passphrase from the EFF wordlist (about 77 bits). The admin MAY choose their own. It MUST have at least 6 characters, an uppercase letter, a lowercase letter, a number and a symbol (decided 2026-11-17); the strength meter is shown as advice. ⚠️ Six characters are weak against offline guessing if encrypted data leaks; a minimum of 10–12 characters is recommended (to decide). **Why:** the server stores the salt and the wrapped key, so a malicious operator could try passwords offline. Argon2id plus a strong passphrase makes that infeasible. |
| R-315 | Account passwords MUST reach zxcvbn score 3 or more. |

### 6.3 Argon2id parameters

| ID | Requirement |
|---|---|
| R-301 | Default: the RFC 9106 second recommended option, **m = 64 MiB, t = 3, p = 4**. Spike S4 checks this on web (single-threaded WASM) and on low-end Android. Target unlock time ≤ 3 s. Parameters MUST NOT go below the OWASP minimum (m = 19 MiB, t = 2, p = 1). |
| R-302 | Parameters MUST be stored with each salt (`vault_header`; account parameters are fixed per `format_version`), so they can be raised later. |

### 6.4 Encrypted envelope format (versioned, algorithm-agile)

```text
envelope  = magic "TH" (2 B) | format (1 B, =0x01) | suite (1 B) | domain_id (1 B) | key_version (4 B)
            | [suite 0x02 only: recipients] | nonce (24 B) | ciphertext ‖ tag (16 B)
suite 0x01 = domain-key encryption (XChaCha20-Poly1305, with the domain's record key)
suite 0x02 = sealed:      random content key, X-Wing-encapsulated to each recipient
                          (the domain's public key + the author's own public key)
AAD       = "theo/v1/record" ‖ congregation_id ‖ collection ‖ scope_id ‖ record_id ‖ version
            ‖ suite ‖ domain_id ‖ key_version
plaintext = pad_padme( CBOR(schema_version, payload) )
```

| ID | Requirement |
|---|---|
| R-303 | Every encryption MUST use a fresh random 24-byte nonce. |
| R-304 | The AAD MUST bind the ciphertext to its congregation, collection, scope, record ID, version, key domain and key version. The server cannot then move records between rows, collections, scopes, versions or domains without detection. |
| R-305 | Plaintext SHOULD be padded (Padmé) to hide exact sizes. |
| R-306 | Serialization MUST be self-describing and able to evolve: CBOR (`minicbor` with indexed fields). |
| R-307 | Unknown `format` or `suite` values MUST fail closed, with a clear error. |
| R-308 | Envelope parsing MUST be fuzz-tested (`cargo-fuzz`). |

### 6.5 Key management on the device

| ID | Requirement |
|---|---|
| R-309 | Key material MUST be held in `Zeroizing`/`SecretBox` types, and wiped on lock, logout and drop. |
| R-310 | **Auto-lock** after a configurable idle time (default 5 min), and when a mobile app goes to the background (default on). Locking drops all keys and all decrypted data. |
| R-311 | **Optional recovery kit.** At setup, the admin MAY create a 256-bit recovery key, shown once as 24 words or a QR code. It wraps `root_key` a second time. Only the user holds it. It is off by default, and the UI explains the trade-off. |
| R-312 | **Biometric unlock** (replaces "remember on this device", 2026-11-16). After a password unlock, the user MAY turn on biometric unlock per congregation and per device. A key that unwraps the congregation's keys is kept in secure hardware and released only after a biometric check: **Web:** WebAuthn passkey with the PRF extension (the browser stores only a key wrapped by the PRF output; nothing usable without the authenticator). **Windows:** Windows Hello. **macOS / iOS:** Keychain item with biometry access control (Touch ID, Face ID). **Android:** Keystore key bound to BiometricPrompt. **Linux:** not available; password only. Off by default. Turning it off, changing the congregation password or rotating keys deletes the stored key. |
| R-313 | The app MUST show a clear, unskippable warning at setup: no one can recover a lost congregation password. |

### 6.6 Post-quantum features

| ID | Phase | Requirement |
|---|---|---|
| R-321 | 2 | **Key domains and grants** (R-247): domain secrets are encapsulated with X-Wing to each member's public key. (Was phase 4 in v0.2.) |
| R-322 | 2 | **Sign in anywhere with the account password:** this follows from R-321. (Was phase 4 in v0.2.) |
| R-323 | 4 | **Signed records and manifest:** hybrid ML-DSA-65 + Ed25519 signatures prove who wrote what. They also detect a server that deletes records or rolls them back, which AEAD alone does not detect. |
| R-327 | 2 | **Sealed submissions** (suite 0x02): a member can write a record into a domain they cannot read. Examples: their own field service report into `reports`, their own absence, or a change request for their own contact details into `people`. Readers of the domain and the author can decrypt it. Nobody else can. |

---

## 7. Security requirements and threat model

### 7.1 Attackers we defend against

| Attacker | Protection |
|---|---|
| Cloud server operator, or a leaked database dump | Only ciphertext plus the metadata in 7.3. The auth key cannot decrypt anything. |
| Network attacker | TLS 1.3 with hybrid PQ key exchange. On top of that, content is E2E encrypted. |
| A member trying to see more than their role allows | No key for domains outside their roles (R-247). Server-side permissions inside a domain (R-241). |
| A removed member | Membership revoked on the server, and their domains rotated (R-246). |
| Stolen or lost device, while locked | Data at rest is encrypted. Keys exist only after unlock. |
| Malicious server tampering | AEAD with AAD (R-304). Rollback and deletion detection comes in phase 4 (R-323). |
| "Harvest now, decrypt later" quantum attacker | 256-bit symmetric keys and hybrid PQ public-key crypto. |

### 7.2 Out of scope (stated honestly)

- **Malware on an unlocked device.** No app can defend against a compromised OS.
- **Record-level rules inside a domain.** Key domains separate members cryptographically. Finer
  rules *inside* a domain still rely on the server applying its permissions (R-241). Examples are
  "own group only" or "own record only". A malicious server operator could break those finer rules
  for a member who already holds that domain's key. Splitting a domain further (for example one
  `reports` key per field service group) is possible later, if needed.
- **A member's own copies.** A removed member keeps whatever they already decrypted on their device.
- **Web delivery risk.** The web build is only as trustworthy as its host. A hacked host could serve
  modified WASM that captures passwords. Mitigations: strict CSP, Subresource Integrity, and
  reproducible builds with published hashes, which a public repo makes possible. The docs MUST
  point users to the native apps for the highest assurance.
- **Weak passwords.** Reduced by R-314 and R-315.

### 7.3 Metadata that the storage layer can see

- Congregation ID, record IDs, versions, coarse timestamps, padded sizes, tombstones.
- **Collection names**, and so the number of records per module (for example how many field service
  reports exist in a month).
- **Scope IDs** (random, not names). Key domain and key version of each record.
- Cloud only: account usernames, memberships with their roles, which member holds which domain
  (grants), authorship of each change (audit log), invite hashes.
- Outside services: the **tile provider** sees which map areas are viewed (R-045). **jw.org** sees
  which publication issues and languages are downloaded. Neither sees congregation data.

Usernames SHOULD NOT be real names. The UI suggests a nickname or e-mail-like handle, and the real
name stays in the encrypted person record.

### 7.4 Security requirements for the app

| ID | Requirement |
|---|---|
| R-331 | The web build MUST ship a strict CSP: `default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self'; base-uri 'none'; form-action 'none'`. **`img-src`:** `'self' data: blob:` plus the tile hosts. The public build allows `https:`, because the provider is configurable (R-043). **`connect-src`:** `'self'`, the jw.org download hosts (S8), and the SurrealDB server. The public build cannot know the user's server in advance, so it allows `wss:`. A self-hosted build (the `theo-db/deploy` Caddy example) lists exact hosts. Spike S5 records any extra exception that Dioxus needs. |
| R-332 | Hosts that can set headers MUST send CSP as a header, plus HSTS, `X-Content-Type-Options: nosniff`, `Referrer-Policy: no-referrer`, `frame-ancestors 'none'` and a deny-all `Permissions-Policy`. On GitHub Pages, which cannot set headers, the CSP goes in a `<meta>` tag. Map tile images override the referrer policy with `referrerpolicy="strict-origin"`, because the OSM policy requires a Referer (R-041). |
| R-333 | Logs (`tracing`) MUST NEVER contain secrets, plaintext, usernames or record content. A log redaction test enforces this. |
| R-334 | All repos MUST run `cargo-deny` (advisories, licences, banned crates, duplicate versions) and `cargo-audit` in CI. |
| R-335 | `theo-crypto` MUST use `#![forbid(unsafe_code)]` and test every primitive against official test vectors (KATs). |
| R-336 | A third-party crypto review SHOULD happen before a 1.0 release. |
| R-337 | Release artifacts MUST be signed, with build provenance (GitHub artifact attestations). Builds SHOULD be reproducible. |

---

## 8. UI and design system

### 8.1 Layout

| Screen width | Navigation | Notes |
|---|---|---|
| `< 768px` (phones) | **Bottom dock** (daisyUI `dock`, max 5 items: for example Home, Meetings, Duties, Field service, More) + top app bar | Respect `env(safe-area-inset-*)`. Touch targets ≥ 44×44 px. |
| `768–1023px` (tablets) | **Collapsed sidebar** (icon rail) that expands as a drawer | |
| `≥ 1024px` (desktop) | **Full sidebar** (daisyUI `drawer` + `menu`), collapsible, grouped by module | |

| ID | Requirement |
|---|---|
| R-401 | One `AppShell` component in `theo-ui` MUST own this responsive behaviour. Screens never build their own navigation. |
| R-402 | Mobile-first CSS: base styles target phones, and `md:` / `lg:` add the larger layouts. |
| R-403 | All screens MUST work from 320px to 2560px wide, with no horizontal scrolling. Wide tables (schedules) switch to a card or list layout on phones. |
| R-404 | Navigation entries MUST follow permissions (R-245). A user only sees the modules they can use. |
| R-405 | A member with only the Publisher role gets a **simple home screen**: my next assignments, my absences, my report, this week's schedules. Large text and few choices, so less experienced users can use it. |

### 8.2 Component library (`theo-ui`)

| ID | Requirement |
|---|---|
| R-411 | All UI building blocks MUST come from `theo-ui` as typed Dioxus components with props (enums for variants and sizes, not free-form class strings). |
| R-412 | `theo-app` MUST NOT use raw Tailwind/daisyUI classes or inline styles. A CI check fails on `class:` or `style:` in `theo-app/src`, with no allowlist (8.4, R-431). |
| R-413 | v1 component set. **Layout:** `AppShell`, `Sidebar`, `Dock`, `TopBar`, `Page`, `PageHeader`, `Stack`, `Grid`, `Card`, `Divider`. **Inputs:** `Button`, `IconButton`, `Input`, `PasswordInput` (with strength meter), `PassphraseDisplay`, `TextArea`, `Select`, `Combobox` (search people), `Checkbox`, `Switch`, `Radio`, `DatePicker`, `WeekPicker`, `Field` (label + help + error), `Form`. **Feedback:** `Alert`, `Toast`, `Badge`, `Spinner`, `Skeleton`, `Progress`, `EmptyState`. **Overlay:** `Modal`, `ConfirmDialog`, `Drawer`, `Dropdown`, `Menu`, `Tooltip`. **Data:** `Table`, `List`, `Tabs`, `Avatar`, `Calendar`, `ScheduleGrid` (week × part × person), `MapView` (R-046), `FileDrop` (EPUB, GeoJSON, KML import). **Patterns:** the page templates and pattern components in 8.4. |
| R-414 | `theo-ui` MUST ship a **gallery** example app that shows every component in every variant, in light and dark themes. |
| R-415 | Themes: `theo-light`, `theo-dark` and "system", through daisyUI theme variables. Design tokens (colours, radius, spacing, font sizes) are defined once in `theo-ui/assets/theme.css`. |
| R-416 | Fonts: the **system font stack** only. No web fonts. |
| R-417 | Icons: **Lucide** (ISC licence) as Rust SVG components (for example `dioxus-free-icons` with the `lucide` feature). Only the icons we use are compiled in. |
| R-418 | Accessibility: **WCAG 2.2 AA**. Keyboard navigation, visible focus, ARIA roles on custom widgets, contrast ≥ 4.5:1, `prefers-reduced-motion`, `prefers-color-scheme`. |
| R-419 | Form validation lives in `theo-core` (one source of rules). `Field` shows errors the same way everywhere. |
| R-420 | **Print styles:** schedule pages MUST print cleanly on A4 and Letter with `@media print` (M-13). PDF export is decided in spike S6. |

### 8.3 CSS toolchain

- The Tailwind CSS v4 **standalone CLI** and `daisyui.mjs` are pinned by version and checksum. No
  Node.js or npm.
- `dx` builds `tailwind.css` automatically. `theo-ui` exports its CSS as a Dioxus `asset!()`, so the
  app includes it once.
- `@source` points at the `theo-ui` `.rs` files only, because `theo-app` uses no classes (R-412).
  Unused classes are removed.

### 8.4 UI consistency rules (D-16)

**Goal:** a menu, form, button, list, dialog or message looks and behaves **the same on every page
and in every module**. Screens put together `theo-ui` components; they never style or rebuild
them. If a screen needs something new, it is added to `theo-ui` first, and every screen gets it.

**One component per pattern.** Each UI pattern has exactly one component. Screens MUST use it:

| Pattern | The only component allowed | What it fixes for every screen |
|---|---|---|
| Page layout | `ListPage`, `DetailPage`, `FormPage`, `SchedulePage`, `DashboardPage` | Header, title, breadcrumbs, primary action place, content width, padding |
| Navigation | `AppShell` (sidebar / rail / dock) | Order, icons, active state, permission filtering (R-404) |
| Actions on a record or row | `ActionMenu` | Same trigger (⋮), same position, same order, destructive actions last and in red |
| Several selected rows | `BulkActionBar` | Same place and behaviour in every list |
| Forms | `Form` + `Field` + input components | Layout, labels, help, errors, required mark, buttons, saving state |
| Lists and tables | `DataTable` / `DataList` | Sorting, filtering, search, paging, row actions, card layout on phones |
| Confirmation | `ConfirmDialog` | Text pattern, button order, destructive style |
| Messages | `Toast` (results), `Alert` (page-level), `Field` error (input-level) | Position, duration, colours, icons |
| Data states | `Skeleton`, `EmptyState`, `ErrorState`, `LockedState` | Loading, empty, error with retry, locked/offline look the same everywhere |
| People and dates | `PersonChip`, `PersonPicker`, `DateText`, `WeekText`, `DurationText` | One way to show and pick people, dates, weeks and durations |

**Requirements:**

| ID | Requirement |
|---|---|
| R-421 | Every screen MUST be built from one of the **page templates** above. Templates have named slots (header, filters, content, aside, footer actions). Screens fill slots; they do not change the layout. |
| R-422 | **Actions vocabulary:** `theo-ui` has one `Action` catalogue (`Save`, `Cancel`, `Edit`, `Delete`, `Add`, `Publish`, `Export`, `Print`, …). Each action has one label (i18n key), one icon and one style. Screens use catalogue actions, so "Delete" is always the same word, icon, colour and place. |
| R-423 | **Button hierarchy:** at most **one primary** button per screen or dialog. Then secondary and ghost buttons. Destructive actions always use the destructive style and always open a `ConfirmDialog`. |
| R-424 | **Button order and place:** the same everywhere. Larger screens: actions at the bottom right of forms and dialogs, primary last, with a minimum width. Phones: actions full width; on **edit pages** the title sits in a sticky bar with a back arrow (the way to cancel) and Save is a round floating button above the dock (2026-11-17). `theo-ui` decides this (`Page` with `back`, `Form` with `floating_save`), not the screen. |
| R-425 | **Form behaviour** is owned by `Form`: label above the input, help text below, error below in the error colour, `*` for required fields, validation on leaving a field and on submit, focus moves to the first error, Enter submits, buttons disabled with a spinner while saving, and a warning before leaving with unsaved changes. |
| R-426 | **Feedback:** success is a `Toast` (same corner, same duration). Errors in an input are shown in its `Field`. Page errors use `Alert` with a retry action. Native browser `alert()`/`confirm()` are never used. |
| R-427 | **Data display:** dates, times, weeks, durations, numbers and names MUST use the shared formatters and components (locale-aware). A person is always shown with `PersonChip` (same name order, same avatar style). |
| R-428 | **Icons:** one icon per concept, through an `Icon` enum in `theo-ui` (`Icon::Edit`, `Icon::Absence`, …). `theo-app` MUST NOT import icon crates directly. |
| R-429 | **Texts:** shared labels use common i18n keys (`common.save`, `common.cancel`). A screen MUST NOT define its own text for a shared action. |
| R-430 | **Closed components:** `theo-ui` components MUST NOT accept a free `class` or `style` prop. Variants are enums. A one-off override is impossible by design. |
| R-431 | **No local UI in `theo-app`:** in `rsx!`, `theo-app` MUST NOT use `class:`, `style:` or raw interactive or layout HTML elements (`button`, `input`, `select`, `textarea`, `form`, `a`, `table`, `dialog`, `ul`, `div` with layout). It uses only `theo-ui` components. |
| R-432 | **New patterns go to `theo-ui` first.** A pull request that needs a new component or variant adds it to `theo-ui` with a gallery entry (R-414), docs and an accessibility check. Then `theo-app` uses it. |
| R-433 | **Spacing and sizes** come only from design tokens (R-415). Components use the tokens; screens never set spacing. |
| R-434 | **Enforcement in CI:** (1) a lint step (an `xtask` in `theo-app` that parses the `rsx!` sources) fails on anything R-431 forbids; (2) **visual regression tests**: the `theo-ui` gallery and key screens are screenshotted at 375, 768 and 1440 px in light and dark themes, and compared with approved images; (3) accessibility checks on the gallery. |
| R-436 | **Lists are cards, not tables** (`RecordCard` in a grid), so they work on phones. Filters use `FilterBar`: search, then one menu per filter on tablets and desktops, or one "Filters" sheet on phones (the button shows how many filters are changed; there are no chips under it). Overlays use `Dialog` (a bottom sheet on phones, centred on larger screens). Menus (`MenuButton`) are dropdowns on larger screens and bottom sheets on phones. Detail pages use `Split`: the form on the left, related records (`RowList`) on the right; choosing several records uses `PickList` in a `Dialog`. |
| R-437 | **Web basics:** the page has `lang` (following the user's language), a meta description, a `main` landmark on every page, `robots.txt`, and a source map for each JavaScript file (added by `scripts/web-postbuild.py`). |
| R-435 | The pull-request template has a short UI checklist: page template used, catalogue actions used, all data states handled (loading, empty, error), checked on phone and desktop. |

---

## 9. Platforms, performance and size budgets

### 9.1 Platform matrix

| Platform | Renderer | Local engine | Min. version (to confirm in S1) | Package |
|---|---|---|---|---|
| Web | Browser DOM (WASM) | IndexedDB | Last 2 versions of Chrome, Firefox, Safari, Edge | Static files (GitHub Pages, or self-hosted with Caddy) |
| Windows | WebView2 | SQLite | Windows 10 1809+ | MSI / MSIX |
| macOS | WKWebView | SQLite | macOS 12+ | `.app` / DMG (universal) |
| Linux | WebKitGTK | SQLite | Current Ubuntu LTS, Fedora | AppImage, `.deb`, Flatpak |
| iOS | WKWebView | SQLite | iOS 16+ | App Store / TestFlight |
| Android | Android WebView | SQLite | Android 8 (API 26)+ | APK / AAB (arm64-v8a, x86_64) |

### 9.2 Budgets (CI fails when a budget is exceeded; calibrate after spike S1)

| Metric | Budget |
|---|---|
| Web: main `.wasm` (brotli) | ≤ 800 KB (stretch: 500 KB) |
| Web: CSS (brotli) | ≤ 40 KB |
| Web: first render, mid-range phone on 4G | ≤ 2 s |
| Desktop installer | ≤ 15 MB |
| Android APK per ABI | ≤ 15 MB |
| iOS app download | ≤ 20 MB |
| Unlock (Argon2id), mid-range device | ≤ 1 s native, ≤ 3 s web |
| Cold start to lock screen (desktop) | ≤ 1 s |
| Decrypt and index a full congregation (300 people, 5 years of history) | ≤ 1 s native, ≤ 2 s web |
| Idle memory (desktop, unlocked, same data set) | ≤ 150 MB |

### 9.3 Release profile (all crates)

```toml
[profile.release]
opt-level = "z"        # size first; spike S1 compares "s" against "z"
lto = "fat"
codegen-units = 1
panic = "abort"
strip = true

# Web: strip would remove the custom sections wasm-bindgen needs (found 2026-11-16).
[profile.wasm-release]
inherits = "release"
strip = false
```

The web release also runs `wasm-opt -Oz` (done by `dx bundle`). Hosts serve brotli pre-compressed
assets.

---

## 10. Repository and dependency management

### 10.1 One repository, several crates (D-25)

Everything lives in **one public repository**, `theo-manager`, with one Cargo workspace. The crates
keep the boundaries below, so audit-critical code stays apart from fast-moving feature code, but
they share one history, one CI and one release. (Until 2026-11-17 the plan was one repository per
crate, D-06; it was replaced because one person maintains the project and changes usually touch
several crates at once.)

| Crate | Type | Contents | Depends on | Status |
|---|---|---|---|---|
| `theo-crypto` | lib | KDF, AEAD, envelopes, hybrid PQ KEM and signatures, key hierarchy, passphrases, zeroization | external crates only | Planned |
| `theo-core` | lib | Domain models for M-01 to M-13, eligibility, permission model, key domains and grants, use cases, session and lock lifecycle, search, conflict handling, jw.org EPUB parser (parsing only, no network) | `theo-crypto`, `theo-db` | Planned |
| `theo-db` | lib | **The single database wrapper**: `Db` API, engines (`sqlite`, `indexeddb`, `surreal-remote`, `memory`), conformance suite, SurrealDB migrations (`surreal/`), deployment examples (`deploy/`) | external crates only | Planned |
| `theo-ui` | lib | Dioxus components, `AppShell`, theme CSS, icons, gallery example | `dioxus` only (no domain knowledge) | In use |
| `theo-app` | bin | Routes, module screens, wiring, network fetches (EPUB downloads, map tiles), `Dioxus.toml`, platform configs | `theo-core`, `theo-db`, `theo-ui` | In use (with in-memory mocks of the planned crates) |

```text
theo-app ──► theo-ui
    ├──────► theo-core ──► theo-crypto
    │            └───────► theo-db
    └──────► theo-db      (only to choose DbConfig at startup)
```

**Rules of the graph:**
- No cycles.
- `theo-db` never depends on `theo-crypto` or `theo-core`. It handles envelopes only (R-204).
- `theo-ui` never depends on `theo-core`. It shows data, it does not own it.

**Why there is no `theo-cloud` crate:** the SurrealDB schema belongs to the `surreal-remote`
engine. Both change together, so they live together in `theo-db`. The app applies the schema
itself (R-228), so there are no deployment scripts to maintain.

### 10.2 Versions and dependencies

| ID | Requirement |
|---|---|
| R-501 | The app follows **SemVer**. A release is a GitHub release with a tag `vX.Y.Z` on `main`; the release workflow builds and publishes it (R-516). |
| R-502 | Crates depend on each other by **path** inside the workspace. Publishing to crates.io MAY come later. |
| R-503 | (removed: `cargo-semver-checks` was only needed for separately released crates.) |
| R-504 | The workspace commits its `Cargo.lock`. |
| R-505 | Dependency updates are automated with Dependabot (`.github/dependabot.yml`). Shared crates (`dioxus*`) are grouped. |
| R-506 | `rust-toolchain.toml` pins one stable Rust version for local builds and CI. That version is also the MSRV (minimum supported Rust version). |
| R-516 | **Release workflow** (`.github/workflows/release.yml`): publishing a release from a tag on `main` builds Windows (x64, x86, ARM64), macOS (x64, ARM64), Linux (x64, ARM64), Android and iOS packages, attaches them to the release, and deploys the web build to GitHub Pages. Android packages are signed with the release key from the `release` environment (how-to: `docs/release-signing.md`); desktop and iOS signing come later (R-704). The app is called "TheoManager" on every platform (the binary is `theo-manager`). |
| R-517 | **CI** (`.github/workflows/ci.yml`) on every pull request and push to `main`: clippy (native and WebAssembly), the desktop build, all tests including the UI rules, an up-to-date stylesheet check, and a secret scan. |

### 10.3 Repository layout

```text
theo-manager/
├── Cargo.toml            ← workspace and build profiles
├── rust-toolchain.toml
├── REQUIREMENTS.md
├── docs/model/           ← one data-model file per module (M-01 … M-14)
├── docs/design/          ← design proposals (for example sharing Local congregations)
├── scripts/              ← release helpers (web post-build)
├── .github/              ← CI, release, Dependabot, issue and PR templates
├── theo-ui/              ← design system
└── theo-app/             ← the app
```

### 10.4 Licence (D-13, accepted)

**Goal:** anyone may read, clone and fork the code, and contribute. Nobody else may publish a copy:
no app-store copies, no public websites, no binaries.

No standard licence matches this exactly:

| Licence | Read, clone, fork | Change (for pull requests) | Publish copies | Fit |
|---|---|---|---|---|
| MIT / Apache-2.0 / AGPL-3.0 | ✅ | ✅ | ✅ allowed | ❌ allows copies |
| PolyForm Noncommercial 1.0.0 | ✅ | ✅ | ✅ allowed (noncommercial) | ❌ allows copies |
| **PolyForm Strict 1.0.0** | ✅ (use) | ❌ forbidden | ❌ forbidden | ⚠️ closest; needs an addendum |

**Proposal:** **PolyForm Strict 1.0.0 + a short addendum** in `LICENSE`. The addendum grants three
extra permissions:
1. Making changes **only** to prepare contributions to this project, or to test the code privately.
2. Use by congregations and their members for their religious activities is a permitted purpose.
   (PolyForm lists "charitable organization" but does not name religious organizations, so we say
   it explicitly.)
3. A congregation MAY host the **unmodified official web build** for its own members.

| ID | Requirement |
|---|---|
| R-511 | The repository MUST carry `LICENSE.md` (PolyForm Strict 1.0.0 + addendum, in place since 2026-11-17). |
| R-512 | Contributions need a **CLA** (Contributor Licence Agreement, through the CLA Assistant GitHub app). Without it, the project could not ship contributed code under its own terms. |
| R-513 | The name and logo are reserved in a `TRADEMARKS.md`. A fake copy of an E2E app could steal passwords, and the reserved name makes takedown requests to app stores easy. |
| R-514 | Third-party licences of all dependencies MUST be collected into the app's "About → Licences" screen (`cargo-about`). `cargo-deny` allows only licences compatible with distribution in a closed binary (MIT, Apache-2.0, BSD, ISC, Zlib, Unicode, MPL-2.0 file-level). |
| R-515 | A lawyer SHOULD review the addendum before the first public release (to confirm). |

**Consequences:**
- The project is **source-available, not "open source"** in the OSI sense.
- F-Droid will not list it.
- Some contributors avoid non-OSI projects.
- GitHub's terms of service still let anyone fork on GitHub.
- The app stays **free**, which the jw.org terms also require (R-038).

---

## 11. Engineering practices and public-repo hygiene

### 11.1 Quality

| ID | Requirement |
|---|---|
| R-601 | Every repo's CI runs `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test`, `cargo-deny`, and a `wasm32-unknown-unknown` build (all libs except platform-only code). |
| R-602 | `theo-db` runs the **conformance suite** against every engine: SQLite on Linux, macOS and Windows; IndexedDB with `wasm-bindgen-test` in headless Chrome and Firefox; SurrealDB against a throwaway server container started by CI. |
| R-603 | `theo-crypto`: KATs, property tests (`proptest`: "decrypt(encrypt(x)) = x", "any tampering fails"), fuzz targets, and an Argon2 benchmark per platform. |
| R-604 | `theo-core`: unit tests for every eligibility and permission rule, and a **synthetic congregation generator** (`fake` crate) used by tests, the gallery and demos (R-022). |
| R-605 | `theo-app` CI builds all 6 platforms on every release tag. Public repos get free macOS runners for the iOS and macOS builds. CI enforces the size budgets (9.2). |
| R-606 | End-to-end tests in Rust: WebDriver (`fantoccini` or `thirtyfour`) against the web build. They cover setup, unlock, invite and join, CRUD, lock, and the layouts at 375, 768 and 1440 px. |
| R-607 | Error handling: `thiserror` in libraries. No `unwrap()`/`expect()` outside tests (clippy `unwrap_used` and `expect_used` = deny in libs). User-facing errors never show internals. |
| R-608 | Every public item in library crates is documented (`#![warn(missing_docs)]`). Each repo has a README with purpose, features and examples. |
| R-609 | Commits follow Conventional Commits, so `release-plz` can write changelogs. |
| R-610 | i18n from the start: every UI string goes through a translation layer. The library and languages are open (Q7). |

### 11.2 Public repository hygiene (no secrets, ever)

| ID | Requirement |
|---|---|
| R-701 | No repo MAY contain a secret: no API keys, passwords, tokens, private keys, DB credentials, signing certificates or `.env` files. **The app needs no build-time secrets at all.** Server URLs and credentials are runtime input from the user. |
| R-702 | Every repo's `.gitignore` covers `.env*`, `*.pem`, `*.key`, `*.p12`, `*.jks`, `*.keystore`, `*.mobileprovision`, `*.sqlite*`, `*.db` and local exports. |
| R-703 | GitHub **secret scanning** with **push protection**, and Dependabot alerts, are on for every repo. CI runs `gitleaks` on every pull request. |
| R-704 | Release signing material (Apple, Android, Windows) lives only in GitHub Actions **environment secrets** of a protected `release` environment, with a required reviewer. Provenance uses keyless GitHub artifact attestations. |
| R-705 | Deployment examples in `theo-db/deploy/` use placeholders (`<CHANGE_ME>`). The SurrealDB owner password is set on the host at deploy time (Docker secret or environment variable), never committed. |
| R-706 | Test fixtures, screenshots and demo data are synthetic only (R-022). |
| R-707 | `main` is protected: pull request + green CI required. Signed commits are recommended. |
| R-708 | Each repo has a `SECURITY.md` and GitHub private vulnerability reporting turned on. |
| R-709 | Each repo has a `LICENSE` file (10.4). |

---

## 12. Roadmap and de-risking spikes

### Phase 0: Spikes (each one is time-boxed, and its outcome goes into the Decision log)

| Spike | Question to answer | Exit criterion |
|---|---|---|
| **S1** | Do Dioxus 0.7 + Tailwind/daisyUI and `AppShell` (sidebar/dock) work on all 6 platforms? | Builds, runs and looks the same on each one. Sizes measured against 9.2. |
| **S2** | Does `rusqlite` (bundled) cross-compile for iOS and Android? | Conformance suite green on each platform. |
| **S3** | Web local engine: direct IndexedDB vs SurrealDB embedded (`kv-indxdb`). Also the WASM size of the remote-only `surrealdb` SDK. | Decision for `LocalBrowser`. SDK kept if it adds ≤ 250 KB brotli; otherwise a thin JSON-RPC client. |
| **S4** | Argon2id timing (web, low-end Android). Do libcrux ML-KEM/ML-DSA build on `wasm32` and mobile? | Final R-301 parameters and PQ crate choice. |
| **S5** | Does Dioxus web run under the CSP in R-331? | List of needed exceptions (ideally none). |
| **S6** | Schedule export: browser print CSS vs a Rust PDF crate (for example `printpdf`) in WASM | Chosen approach for M-13, with its size cost. |
| **S7** | SurrealDB permissions for role + scope rules (R-241), and full-access-only `vault_header` (R-256) | A working prototype of `fs_report.read@own_group`, with acceptable query speed. |
| **S8** | jw.org EPUB import: how the app finds the EPUB URL for an issue and language (the public download links), whether the web build can download it (CORS), and the WASM size of `zip` + `quick-xml` | Working import on native; direct download or file-only import on web (R-035); parser size. |
| **S9** | Rust `MapView`: OSM raster tiles, boundary polygons, pan/zoom on touch, drawing; OSM policy rules (R-041) | A smooth map on a mid-range phone, with no JS map library. |
| **S10** | Key domains end to end: invite bundle → grants → sealed submission → domain rotation (R-246, R-321, R-327) | Working flow with 3 roles, with ML-KEM in WASM inside the size budget. |

### Later phases

| Phase | Scope |
|---|---|
| **1. Foundation (Local)** | `theo-crypto` v0.1, `theo-db` (sqlite, indexeddb, memory), `theo-core` congregation lifecycle (create, unlock, lock, change password, export/import) with the keyring (key domains exist in Local too), `theo-ui` v1 + gallery, `theo-app` shell, setup and settings. |
| **2. Cloud** | `surreal-remote` engine, migrations and provisioning (R-228), accounts and personal PQ keys, both invite types (5.6), grants and sealed submissions (R-321, R-327), roles and permissions (5.5), domain rotation (R-246), live updates, conflict handling, Local → Cloud move. |
| **3. Modules** | Proposed order, each one starting with a data-modeling session: M-01 People → M-09 Field service groups → M-02 roles UI → M-03 Midweek (with jw.org import) → M-04 Weekend + M-06 Public talks → M-05 Duties → M-13 Schedule output → M-10 Reports → M-11 Attendance → M-07 Territories (with maps) → M-08 Public witnessing → M-12 Calendar. |
| **4. Advanced security** | Signed records (R-323), offline-first Sync mode (R-206), finer key domains if needed, external crypto review. |

---

## 13. Questions

### 13.1 Answered

| # | Question | Answer (2026-11-13) |
|---|---|---|
| Q1 | What does Theo Manager manage? | A Jehovah's Witnesses congregation: modules M-01 to M-13 (1.3). Each module is modeled later. |
| Q2 | Where does the code live? | **Public GitHub** repositories. No credentials in code or `.env` (11.2). |
| Q3 | Single user or shared? | Both. Local: one user, congregation password. Cloud: account sign-in + shared congregation password, and new users join through a link (5.6). |
| Q9 | Licence? | Allow clone and fork, block publishing copies. **Proposed:** PolyForm Strict 1.0.0 + addendum (10.4). Needs your confirmation (Q18). |
| Q11 | Cryptographic separation of sensitive modules? | **Yes**, it follows from Q15: publishers get accounts, so they must not hold the keys to all data. Key domains are in v1 (D-12). Needs your confirmation (Q19). |
| Q12 | Territory maps? | **OpenStreetMap** by default, with pluggable providers (1.7). |
| Q14 | Import from jw.org? | **Yes**, by downloading the official EPUB files on each device. No HTML scraping and no shared cloud copy, because the jw.org terms forbid both (1.6). |
| Q18 | Licence? | **PolyForm Strict 1.0.0 + addendum** in `LICENSE.md` (2026-11-17). CLA and trademark notice still to add (R-512, R-513). |
| Q15 | Accounts for publishers? | **Both:** publishers MAY have accounts for self-service (assignments, absences, reports). People without a device are managed by users with permissions (R-017, R-018). |

### 13.2 Open

| # | Question | Why it matters | Default if nobody decides |
|---|---|---|---|
| Q4 | Should Cloud mode keep an encrypted offline cache on the device? | Using the app without a connection (for example in the Kingdom Hall). | No cache in v1, Sync in phase 4 |
| Q5 | Offer the optional recovery kit (R-311), or strictly no recovery? | UX vs "lose the password, lose the data". | Offer it, off by default |
| Q6 | Allow one small hand-written service worker (JS) so the web app works offline as a PWA? | It breaks P1 by about 30 lines of JS. | No PWA in v1 |
| Q7 | Which UI languages? | i18n library and right-to-left support. Many congregations are foreign-language. | English + Spanish, i18n-ready |
| Q8 | Self-host SurrealDB, or use Surreal Cloud? | Ops effort vs control. | Self-host (docker-compose + Caddy) |
| Q10 | Final product name and crate prefix (`theo-`)? | Repo names, bundle IDs, GitHub organization name, trademark (R-513). | `theo-*` |
| Q13 | Are a "My assignments" view, local reminders and calendar (`.ics`) export enough, given that push and e-mail need a backend? | Expectations for notifications. | Yes |
| Q16 | Where is the official web build hosted? | Header-based CSP (R-332). | GitHub Pages + `<meta>` CSP; Caddy example for self-hosting |
| Q17 | In Cloud mode, should the congregation database store only references to workbook parts, with each device reading titles from its own EPUB (R-033)? Or also store the part titles? | Storing titles is easier, but it may count as "uploading such materials to a server" under the jw.org terms. | References only |
| Q19 | Confirm **key domains** (D-12)? This changes the Q3 answer slightly. Only full-access members use the congregation password. Publishers join by invite only, and day-to-day sign-in needs only the account password. | It is the core of the Cloud security model. | Key domains |
| Q20 | Which ideas from section 16 go into scope? | Scope of phase 3. | Decide per module, in its modeling session |

---

## 14. Decision log

| ID | Date | Decision | Status |
|---|---|---|---|
| D-01 | 2026-11-13 | UI framework: Dioxus 0.7 (over Tauri+Leptos, Tauri+Yew, GPUI) | Proposed, confirm after S1 |
| D-02 | 2026-11-13 | Styling: Tailwind CSS v4 + daisyUI 5 standalone, wrapped by typed `theo-ui` components | Proposed |
| D-03 | 2026-11-13 | No backend. Each congregation is Local or Cloud (Sync later) | Accepted |
| D-04 | 2026-11-13 | Engines: SQLite (native local), IndexedDB (web local; direct or SurrealDB `kv-indxdb`, decided by S3), SurrealDB server over WSS (Cloud) | Proposed, confirm after S2/S3 |
| D-05 | 2026-11-13 | Crypto: Argon2id → HKDF → XChaCha20-Poly1305. Hybrid PQ: X-Wing (ML-KEM-768) + ML-DSA-65/Ed25519 | Proposed, crate choice after S4 |
| D-06 | 2026-11-13 | Polyrepo with a meta repo and `.cargo/config.toml` `[patch]` for local development | Replaced by D-25 |
| D-07 | 2026-11-13 | **One database wrapper `theo-db`**: engine chosen by `DbConfig`. It owns the SurrealDB schema too. `theo-cloud` removed; `theo-store` renamed to `theo-db` | Accepted |
| D-08 | 2026-11-13 | Two secrets: per-user account password (Cloud sign-in) and a shared congregation password (decryption). Refined by D-12 | Accepted |
| D-09 | 2026-11-13 | One SurrealDB database per congregation. The app provisions it with owner credentials that it does not keep | Proposed, confirm in S7 |
| D-10 | 2026-11-13 | Two-layer access control: E2E encryption against the server, server-side role/scope permissions between members | Proposed, confirm in S7 |
| D-11 | 2026-11-13 | Public GitHub repositories, zero secrets, keyless provenance | Accepted |
| D-12 | 2026-11-13 | **Key domains** in v1. The congregation password unlocks a root key, which unlocks one key per domain. Members get only their roles' domains through X-Wing grants. Publishers submit sealed records. Day-to-day Cloud sign-in uses the account password only | Proposed (Q19), confirm in S10 |
| D-13 | 2026-11-13 | Licence: source-available, PolyForm Strict 1.0.0 + addendum, CLA, reserved trademark | Accepted 2026-11-17 (`LICENSE.md`); CLA and trademark notice pending |
| D-14 | 2026-11-13 | Maps: pluggable tile providers, OpenStreetMap by default under the OSMF policy, Rust `MapView` | Accepted, confirm in S9 |
| D-15 | 2026-11-13 | jw.org: official EPUB downloads parsed on each device. No HTML scraping, no shared cloud cache | Accepted (terms of use), details in S8 |
| D-16 | 2026-11-16 | Strict UI consistency: one component per pattern, page templates, an action catalogue, closed components, no local UI in `theo-app`, enforced by a lint and visual regression tests (8.4) | Accepted |
| D-17 | 2026-11-16 | D-01 re-checked against GPUI: stay with Dioxus (webview renderer). GPUI has no production web or mobile target and no CSS. Exit path: Dioxus Native. Guarded by R-104 and R-105 | Accepted |
| D-18 | 2026-11-16 | App flow (scaffold): landing → log in / sign up → choose congregation (dropdown with local/cloud icon) → congregation password or biometrics → `/app`. Sign-up continues to the congregation wizard. Accounts also exist in Local mode, as a profile on the device that lists its congregations (R-060) | Accepted |
| D-19 | 2026-11-16 | Navigation: sidebar (Home, Users, Meetings, Ministry, Congregation, then the user at the bottom) with the congregation switcher on top; icon rail on tablets; dock on phones with Home, Users, Meetings, Ministry and Congregation, and the user's avatar in the top bar (dock limit of 5). Animated dock, sidebar indicator and page transitions | Accepted |
| D-21 | 2026-11-17 | i18n with `dioxus-i18n` (Fluent). The language switch works now; texts stay English until translations are written. Device language detected with `sys-locale` | Accepted |
| D-22 | 2026-11-17 | Sub-navigation as in R-064. The first list of pages per section is a proposal, confirmed per module in its modeling session | Proposed |
| D-23 | 2026-11-17 | Data model M-01 (persons, families) and M-09 (field service groups) drafted in `docs/model/`. One identifier per record (random `record_id`); the UI shows no number. One family per person. Moved away kept 1 year (R-067). Members see only their own group's names (R-068). No password on the person: accounts link through memberships (R-019). Activity status is derived, never stored | Accepted (reviewed 2026-11-17) |
| D-24 | 2026-11-17 | Exports (CSV, DOCX, PDF) written by hand in `theo-app`, without extra crates (bundle budget). The PDF uses built-in Helvetica (Western European characters only) until spike S6 picks a font-embedding approach | Proposed |
| D-27 | 2026-11-18 | **Sharing Local congregations** without a server: one signed, encrypted change log, carried by encrypted packages (all platforms) or nearby peer-to-peer sync (installed apps; QR pairing for web). Read permissions = which key domains a device receives; write permissions = signed changes checked against an administrator-signed access list. Proposal: `docs/design/local-sharing.md` | Accepted 2026-11-18 (sharing later; backup built) |
| D-28 | 2026-11-18 | Main sections: Home, Users, Meetings, Ministry, **Departments** (audio, video and stage; attendants; microphones; cleaning; maintenance). *Congregation* is removed: *Events* moved to *Meetings*, and the congregation *Settings* open from a cog next to the open congregation in the switcher (`/app/settings`). The dock keeps 5 items with Home. Replaces the Congregation part of D-26 | Accepted |
| D-26 | 2026-11-18 | Navigation regrouped. *Ministry*: field service reports and groups first. *Congregation*: Events, Audio, video and stage, Attendants, Microphones, Cleaning, Maintenance, then Settings. Import and export and the activity log become parts of *Settings*. "Other congregations" is removed from the menu (to decide where it lives with public talks, M-06) | Accepted |
| D-29 | 2026-11-18 | *Users*: *Families* becomes a tab of *All users*; new sub-menu *Absences*. *Departments*: new sub-menu *Custom tasks*. *Meetings › Attendance* renamed *Meeting attendance*. Emergency contacts are fields inside the person (M-01 P5). Labels (R-077). One app icon source, `theo-app/icons/icon.svg`; platform icons are generated in the release workflow (`scripts/icons.sh`) | Accepted |
| D-30 | 2026-11-19 | Charts: series colours come from two theme tokens (`--chart-1`, `--chart-2`) with their own light and dark steps, checked with a palette validator (colour-blind separation, lightness, contrast). Line charts start at zero, have a legend for two series, end labels, and a tooltip on hover and keyboard focus; a table with the same numbers is on the page | Accepted |
| D-31 | 2026-11-19 | One month calendar component in `theo-ui` (`Calendar`) for every module that shows things over days (events now; schedules and absences later): a month grid with bars for things that last several days on larger screens, and a compact grid with dots plus a list on phones | Accepted |
| D-32 | 2026-11-19 | Field service meetings keep the **sent version** next to the current one: planners work on a draft month and send it; a change after a send is visible to others only after the next send. Preset availability lists who **can** conduct each preset: nobody is available until ticked. Sending notifies only the people concerned, in *My assignments* on the home page | Accepted |
| D-25 | 2026-11-17 | **One repository** (`theo-manager`) with a Cargo workspace instead of one repository per crate. GitHub Actions for CI and releases; the web app on GitHub Pages | Accepted |
| D-20 | 2026-11-16 | `/app` home is a user-arranged dashboard: widgets reorder and resize by dragging (pointer events, works with touch). Layout saved per congregation (R-061) | Accepted |

---

## 15. Glossary

| Term | Meaning |
|---|---|
| **Account** | A Cloud login (username + account password). It is not the same as a person. |
| **AEAD** | Authenticated Encryption with Associated Data. It encrypts and detects tampering. The AAD is extra data that is authenticated but not encrypted. |
| **Argon2id** | A memory-hard password hashing function. It makes guessing passwords expensive. |
| **Blind index** | A keyed hash of a value. It allows equality search on encrypted data, but leaks which values are equal. |
| **CLA** | Contributor Licence Agreement: contributors give the project the right to ship their code. |
| **Full access** | A membership that may use the congregation password and holds every key domain. |
| **Grant** | A key domain's secret, encapsulated to one member's public key, so only that member can open it. |
| **Key domain** | A group of collections that share one encryption key, for example `reports`. A member can read a domain only if they hold a grant for it. |
| **Root key** | The random 32-byte key that the congregation password unlocks. It unlocks every key domain. |
| **Sealed record** | A record written into a key domain by someone who cannot read that domain (envelope suite 0x02). |
| **E2E / zero-knowledge** | Only the users' devices can decrypt. Servers store data they cannot read. |
| **Envelope** | Our binary format for one encrypted record (6.4). |
| **HKDF** | A function that derives several independent keys from one key. |
| **Invite** | A single-use, expiring link that lets a new account join a Cloud congregation. |
| **KAT** | Known-Answer Test: official test vectors for a crypto algorithm. |
| **KEK** | Key-Encryption Key: a key that only encrypts other keys. |
| **KEM** | Key Encapsulation Mechanism: a public-key way to share a secret key. |
| **Membership** | The link between an account, a person and the roles that the account has in a congregation. |
| **ML-KEM / ML-DSA** | The NIST post-quantum standards for key exchange (FIPS 203) and signatures (FIPS 204). |
| **Padmé** | A padding scheme that hides the exact length of a message, at low overhead. |
| **PQC** | Post-Quantum Cryptography: algorithms believed secure against quantum computers. |
| **Scope** | Which records a permission covers: `all`, `own_group` or `own`. Not the same as a key domain. |
| **X-Wing** | A hybrid KEM that combines X25519 and ML-KEM-768. It is secure if either part holds. |

---

## 16. Ideas from similar apps (to review)

We looked at the public websites and documentation of **Hourglass**, **NW Scheduler** and
**Organized** (2026-11-13). We take ideas only, never code, texts or designs. Organized is
MIT-licensed, but we do not copy from it either. Nothing in this section is in scope until you
confirm it (Q20).

### 16.1 What they do (short)

| | Hourglass | NW Scheduler | Organized |
|---|---|---|---|
| Platforms | Web, iOS, Android | Windows desktop + publisher app (iOS, Android) | Installable web app (PWA) |
| Data | Cloud (AWS); E2E is **optional**, and must be off to export | Local-first; optional E2E sync (Argon2id + AES-256) | Local-first; E2E sync with an access code + master key |
| Workbook import | Not found | Automatic for English; EPUB by hand for other languages | jw.org button or EPUB/JWPUB file |
| Publisher self-service | Accept or decline assignments, reports, reminders | Own assignments, away periods (also for family), reports, own territories | Own assignments, reports, auxiliary pioneer application |
| Territories | Drawing, maps, S-13, address visits | Drawing, KML (Google My Maps) import, S-13, printed cards | Not built yet |
| Notifications | In-app; e-mail/SMS/WhatsApp for public witnessing | Push reminders: first + final, 1 h to 14 days before | None yet |
| Export | PDF, DOCX, CSV, JSON | Word, PDF, e-mail | PDF made offline, one slip per assignment |
| Known complaints | Little documentation; no custom schedule types | Sync overwrites good data with incomplete data | Autofill skips eligible students; no AV/attendant schedules yet |

### 16.2 Proposed improvements for Theo Manager (each needs your yes or no)

| # | Idea | Seen in | Fit with our design |
|---|---|---|---|
| I-01 | **Encryption always on, export always possible** | Contrast with Hourglass | Already true: export runs on the device after decryption. |
| I-02 | **No "last sync wins" overwrites:** merge field by field, and show a clear conflict screen when two people changed the same field | Lesson from the NW Scheduler complaints | Extends R-224. The main reliability risk for Cloud mode. |
| I-03 | **Draft, then publish** schedules: members see a schedule only after it is published | Organized | Fits: a "published" flag plus a server read rule. |
| I-04 | **Explain the assignment helper:** "why this person" and "why not that person" | Organized (users' requests) | Extends R-011. Pure `theo-core` logic. |
| I-05 | **Warnings that do not block** (same day, recent part) vs **absences that do block** | Hourglass, Organized | Refines R-012. |
| I-06 | **Accept or decline** an assigned part from the member's app | Hourglass | A sealed reply (R-327). The scheduler sees the status. |
| I-07 | **Family delegates:** a family head manages absences and reports for family members | Hourglass, NW Scheduler, Organized | Extends R-018: a delegate links accounts to several people. |
| I-08 | **Calendar export (`.ics`)** of my assignments, plus local reminders (first + final) | None of the three seem to have `.ics` | A real advantage, and it needs no backend (Q13). |
| I-09 | **One slip per assignment** (PDF or image) to send by messenger app | Organized | Fits M-13. Made on the device. |
| I-10 | **Custom duty and schedule types** from day one (for example sign-language interpreting or extra cleaning tasks) | Hourglass and Organized users' requests | Shapes the M-05 model: duty types are data, not code. |
| I-11 | **KML, GeoJSON and Territory Helper import** for territories | NW Scheduler | Extends R-044. |
| I-12 | **Import from other apps** (CSV, JSON exports of other tools) to make switching easy | Hourglass imports from other tools | Local parsing. Each format is a small parser in `theo-core`. |
| I-13 | **"Copy from another language"** when a workbook language is not available yet | NW Scheduler | Fits R-036 and R-037. |
| I-14 | **Device and session list** with "sign out this device" | Organized | Server-side session revocation, then domain rotation if needed. |
| I-15 | Public witnessing for **several congregations together** | Hourglass | **Defer.** It needs sharing across congregations, which is hard with E2E. |

## References

- Dioxus: https://dioxuslabs.com, https://github.com/DioxusLabs/dioxus/releases
- daisyUI without Node.js: https://daisyui.com/docs/install/standalone/
- SurrealDB Rust SDK: https://surrealdb.com/docs/sdk/rust/setup
- RFC 9106 (Argon2): https://www.rfc-editor.org/rfc/rfc9106
- FIPS 203 (ML-KEM), FIPS 204 (ML-DSA): https://csrc.nist.gov/pubs/fips/203/final, https://csrc.nist.gov/pubs/fips/204/final
- X-Wing KEM: https://datatracker.ietf.org/doc/draft-connolly-cfrg-xwing-kem/
- OWASP Password Storage Cheat Sheet: https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html
- EFF diceware wordlists: https://www.eff.org/dice
- jw.org Terms of Use: https://www.jw.org/en/terms-of-use/
- OSMF Tile Usage Policy: https://operations.osmfoundation.org/policies/tiles/
- PMTiles: https://docs.protomaps.com/pmtiles/
- PolyForm Strict 1.0.0: https://polyformproject.org/licenses/strict/1.0.0
- PolyForm Noncommercial 1.0.0: https://polyformproject.org/licenses/noncommercial/1.0.0
- Hourglass: https://www.hourglass-app.com/en/features, https://www.hourglass-app.com/en/security
- NW Scheduler: https://nwscheduler.com/frequently-asked-questions/, https://nwscheduler.com/jw-scheduler-security-and-privacy/
- Organized: https://guide.organized-app.com, https://github.com/sws2apps/organized-app
