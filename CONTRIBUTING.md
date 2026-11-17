# Contributing to Theo Manager

Thank you for helping. This page says how to set up the project, which rules every change follows,
and what a pull request needs.

## Before you start

- For a bug, open an issue with the steps to reproduce it.
- For a new feature, open an issue first, so we can agree on it before you write code. Most
  modules get a data-model session first (`docs/model/`).
- Security problems go to a private report, not an issue (see [`SECURITY.md`](SECURITY.md)).

## Set up

```sh
cargo install dioxus-cli --version 0.7.10 --locked
cd theo-app
dx serve --platform web --release
```

## Rules for every change

| Rule | Why |
|---|---|
| **Invented data only.** No real names, phone numbers or reports in code, tests, screenshots, issues or AI prompts (REQUIREMENTS.md R-022). | The app handles sensitive personal and religious data. |
| **No secrets** in the repository: no keys, tokens, passwords or `.env` files (R-701). | The repository is public. CI runs a secret scan. |
| **UI only from `theo-ui`.** Screens in `theo-app` never use `class:`, `style:` or raw HTML elements. A new pattern goes into `theo-ui` first (R-431, R-432). | Every screen looks and behaves the same. `cargo test` fails otherwise. |
| **Same actions everywhere.** Buttons come from the `Action` catalogue (R-422). | "Delete" is always the same word, icon, colour and place. |
| **Lists are cards, not tables** (R-436). | Tables do not work on phones. |
| **Docs change with the code.** Update `REQUIREMENTS.md`, `docs/model/` or a README when behaviour changes. | The docs are the specification. |

## Checks

Run these before you open a pull request. CI runs them too.

```sh
cargo clippy --workspace --all-targets -- -D warnings -A unreachable-code
cargo test --workspace
```

If you changed a component or `theo-ui/css/input.css`, rebuild the stylesheet and commit it:

```sh
./theo-ui/scripts/build-css.sh     # or .\theo-ui\scripts\build-css.ps1 on Windows
```

## Pull requests

- One topic per pull request, with a short description of what changes for users.
- Check the UI on a phone size and a desktop size, in light and dark themes.
- Fill in the checklist of the pull request template.

## Licence of contributions

The project is source-available under [`LICENSE.md`](LICENSE.md). You may change the code to
prepare a contribution. By opening a pull request, you agree that your contribution is shipped
under the project's licence. A Contributor Licence Agreement (CLA) check will be added before the
first release (REQUIREMENTS.md R-512).
