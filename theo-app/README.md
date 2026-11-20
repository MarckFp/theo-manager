# theo-app

The Theo Manager app: routes, screens and wiring for Web, Windows, macOS, Linux, iOS and Android
(one Dioxus 0.7 codebase). All UI comes from [`theo-ui`](../theo-ui); this crate never writes
markup or styles (`REQUIREMENTS.md` 8.4). `tests/ui_rules.rs` fails the build if it does.

> **Status: scaffold.** Screens and navigation are real; data is a **mock** kept in memory
> (`src/state.rs`, `src/mock.rs`). Nothing is encrypted or saved yet. `theo-core`, `theo-crypto`
> and `theo-db` replace the mock in phase 1.

## Run it

Needs Rust (stable) and the Dioxus CLI 0.7: `cargo install dioxus-cli --version 0.7.10 --locked`.

```sh
dx serve --platform web          # http://127.0.0.1:8080
dx serve --platform desktop      # native window (WebView2 / WKWebView / WebKitGTK)
dx serve --platform android      # needs the Android SDK + NDK
dx serve --platform ios          # needs macOS + Xcode
cargo test                       # unit tests + UI rules
```

The debug web build is large (about 70 MB of WebAssembly) and loads slowly. Use
`dx serve --platform web --release` to see the real size and speed.

## Try the flows

1. **Home** (`/`, web only) or **Log in** (installed apps): click **Try the demo**. This creates the account `demo` with two
   congregations: *Riverside* (local) and *Hillcrest* (cloud).
2. **Unlock** (`/unlock`): choose the congregation in the dropdown (icon = local or cloud) and type
   `demo`. Turn on "Use … next time" to try biometric unlock (simulated).
3. **App** (`/app`): dashboard. **Customize** → drag widgets by the handle, resize them from the
   corner. Sidebar on desktop, icon rail on tablets, dock on phones.
4. The congregation switcher (top of the sidebar, or the top bar on phones) changes congregation
   (this locks the current one) or starts **Create congregation**.
5. **Sign up** creates a new account and goes straight to the 5-step congregation wizard.
6. **Me** (avatar): theme (light, dark, system), biometric unlock, lock, sign out.
7. **Users › All users**: invented people in *Riverside* (32 current, 3 who moved away) and
   *Hillcrest* (14). Mix filters, open a card, change the type to see what saving will clear, add a
   relative, mark someone as moved away, export. *Hillcrest* is a Cloud congregation, so the person
   page also has the invite.
8. **Ministry › Field service groups**: four groups; *Town centre* has no assistant yet and
   one publisher has no group, so you see the warnings. Add members, change the overseer.
9. **Users › Moved away**: people who moved away, with the date they are removed. One demo person
   moved away more than a year ago, so the app removes him when you unlock *Riverside*.
10. **Ministry › Field service reports**: 14 months of invented reports. Change the month (arrows,
   or tap the month to jump), open a card to enter a report, see the statistics and set the
   15-hour months in **Goals**.

## Routes

| Path | Screen | Guard |
|---|---|---|
| `/` | Landing (**web only**; installed apps go to `/login`) | Signed in → `/app` |
| `/login`, `/signup` | Account ("Home" link on the web only; "Try the demo" everywhere) | Signed in → unlock or wizard |
| `/unlock` | Choose and unlock a congregation | Needs sign-in; no congregation → wizard |
| `/congregations/new` | Congregation wizard | Needs sign-in |
| `/app` | Dashboard | Needs sign-in and an unlocked congregation |
| `/app/users/all` | All users: tabs People and Families, search, filters, cards, labels, export | Same |
| `/app/users/all/new`, `/app/users/all/<number>` | Person page: new or edit, labels, family, emergency contacts, activity, invite (Cloud) | Same |
| `/app/users/families/<id>` | Family page (from the Families tab) | Same |
| `/app/users/absences` | Absences: who is away when | Same |
| `/app/departments/custom-tasks` | Custom tasks and who does them | Same |
| `/app/meetings/events` | Events: list with filters, or a month calendar; add and change events | Same |
| `/app/meetings/attendance` | Meeting attendance: months of a service year, statistics, export | Same |
| `/app/meetings/attendance/<YYYY-MM>` | One month: 5 weeks × midweek and weekend meeting | Same |
| `/app/ministry/witnessing/presets`, `/app/ministry/service-meetings/presets` | Presets of public witnessing and of meetings for field service | Same |
| `/app/ministry/witnessing` | Public witnessing: shifts (list, calendar), presets and supervisors, fill, assign, send, statistics | Same |
| `/app/ministry/service-meetings` | Meetings for field service: list, calendar, presets, fill, assign and send | Same |
| `/app/ministry/groups` | Field service groups: search, cards, export | Same |
| `/app/ministry/groups/new`, `/app/ministry/groups/<id>` | Group page: name, overseer, assistant, members, activity | Same |
| `/app/ministry/reports` | Field service reports: month, statistics, goals | Same |
| `/app/users/moved-away` | People who moved away (kept 1 year) | Same |
| `/app/<section>` | Section hub: large tiles for its pages | Same |
| `/app/<section>/<page>` | A page (placeholders until phase 3; `/app/settings` and the Users, Ministry pages are real) | Same |
| `/app/me` | Profile: preferences, appearance, security | Same |

Sections and pages are defined in one place: `src/nav.rs`.

## Preferences

Date format, time format, first day of the week, language and name format. Congregations have
defaults (`/app/settings`, the cog next to the open congregation in the switcher); users can override each one (`/app/me`). See
`src/prefs.rs`. Languages are wired through `dioxus-i18n` (`locales/en.ftl`); every language uses
the English texts until translations are written.

## People and exports

`src/people.rs` is an in-memory stand-in for M-01 People and M-09 Field service groups. It follows
the data model in [`docs/model/`](../docs/model); the real types go to `theo-core`. `src/export.rs`
writes CSV, DOCX and PDF without extra crates. Exported files are not encrypted (R-066).

## Biometric unlock

`src/platform.rs` says which method each platform uses (WebAuthn passkey with PRF on the web,
Windows Hello, Touch ID / Face ID, Android BiometricPrompt; none on Linux). In the scaffold, the
prompt is simulated.
