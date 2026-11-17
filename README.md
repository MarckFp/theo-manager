# Theo Manager

A private app to organise a congregation of Jehovah's Witnesses: people and families, field service
groups, meeting schedules, duties, territories, public talks, public witnessing and field service
reports. One Rust codebase runs on the web, Windows, macOS, Linux, Android and iOS.

> **Status: early development.** The screens and navigation work, but data lives **in memory** and
> is **not encrypted yet**. Use the built-in demo (invented data only). Do not enter real personal
> data until encryption and storage are released.

## Why it is different

| | |
|---|---|
| **No backend** | The app is the whole product. A congregation is either **Local** (data only on the device) or **Cloud** (a SurrealDB database the congregation hosts itself). |
| **End-to-end encrypted** | Only members' devices can read the data. A server stores data it cannot read (zero knowledge). Lose the congregation password, lose the data: there is no reset. |
| **Post-quantum** | Key exchange and signatures use hybrid post-quantum algorithms (X-Wing with ML-KEM-768, ML-DSA-65 with Ed25519), so recorded data stays safe later too. |
| **Least access** | Each member gets only the keys of the data their role needs. Publishers see their own assignments; only users with permissions see everyone's data. |
| **Small and fast** | Rust compiled to WebAssembly and native code, no JavaScript framework, a strict size budget. |
| **Consistent UI** | Every screen is built from one design system, `theo-ui`. A test fails the build if a screen writes its own markup or styles. |

The full design, with every decision and its reasons, is in [`REQUIREMENTS.md`](REQUIREMENTS.md).

## What works today

- Landing page (web only), sign up and log in, the congregation wizard (Local or Cloud), unlock
  with the congregation password or biometrics (simulated).
- A dashboard with widgets that you can reorder and resize.
- Sidebar on desktop, icon rail on tablets, dock on phones; light and dark themes.
- Display preferences (date and time format, first day of the week, name order, language) per
  congregation, with personal overrides.
- **Users:** all users with search and filters, the person page (family, activity, invite), people
  who moved away (kept for one year), field service groups, export to PDF, Word and CSV.

The demo account has invented people only (`Try the demo` on the log-in page).

## Stack

| Part | Choice |
|---|---|
| Language | Rust (edition 2024) |
| UI | [Dioxus](https://dioxuslabs.com) 0.7: web (WebAssembly), desktop (system webview) and mobile |
| Styles | Tailwind CSS v4 + daisyUI 5, built with the standalone CLI (no Node.js) |
| Icons | Lucide, through `dioxus-free-icons` |
| Translations | `dioxus-i18n` (Fluent) |
| Storage (planned) | SQLite on devices, IndexedDB in the browser, SurrealDB for Cloud congregations, behind one wrapper |
| Cryptography (planned) | Argon2id, HKDF, XChaCha20-Poly1305, X-Wing (ML-KEM-768 + X25519), ML-DSA-65 + Ed25519 |

## Repository layout

One repository, one Cargo workspace:

| Path | What it is |
|---|---|
| [`theo-ui/`](theo-ui) | The design system: components, page templates, icons, the stylesheet. |
| [`theo-app/`](theo-app) | The app: routes, screens and wiring for all platforms. |
| [`docs/model/`](docs/model) | The data model, one file per module. |
| [`REQUIREMENTS.md`](REQUIREMENTS.md) | Requirements, architecture, security model and the decision log. |
| `scripts/` | Build helpers used by the release workflow. |
| `.github/` | CI, release workflow, issue and pull request templates. |

Planned crates (`theo-core`, `theo-crypto`, `theo-db`) join the workspace when they are written.

## Run it

You need Rust (the version in `rust-toolchain.toml`) and the Dioxus CLI:

```sh
cargo install dioxus-cli --version 0.7.10 --locked
```

Then, from `theo-app/`:

```sh
dx serve --platform web --release   # browser, http://127.0.0.1:8080
dx serve --platform desktop         # native window
cargo test --workspace              # unit tests and the UI rules
```

The debug web build is large and slow to load; use `--release` to see the real size. After you
change a component or `theo-ui/css/input.css`, rebuild the stylesheet with
`theo-ui/scripts/build-css.sh` (or `.ps1` on Windows) and commit `theo-ui/assets/theo-ui.css`.

## Releases

Publishing a GitHub release with a tag like `v0.2.0` on `main` starts
[`.github/workflows/release.yml`](.github/workflows/release.yml). It builds the app for Windows,
macOS, Linux (x64 and ARM64), Android and iOS, attaches the packages to the release, and deploys
the web app to GitHub Pages. Packages are not signed yet.

## Contributing

Contributions are welcome. Read [`CONTRIBUTING.md`](CONTRIBUTING.md) first: it explains the UI
rules, how to run the checks, and why demo data must be invented. Report security problems
privately, as described in [`SECURITY.md`](SECURITY.md).

## Licence

Source-available, not open source: you may read, clone, fork and contribute, but not publish
copies or builds. See [`LICENSE.md`](LICENSE.md) (PolyForm Strict 1.0.0 with an addendum for
contributions and congregations).

Theo Manager is an independent project. It is not made, endorsed or supported by Jehovah's
Witnesses or the Watch Tower Bible and Tract Society.
