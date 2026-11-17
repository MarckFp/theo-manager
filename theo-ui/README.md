# theo-ui

The design system of Theo Manager. Every button, form, menu, list, dialog and page layout of the
app comes from this crate, so they look and behave the same everywhere
(`REQUIREMENTS.md` sections 8.2 and 8.4).

## What is in it

| Area | Components |
|---|---|
| Root | `ThemeRoot` (stylesheet, theme, toasts), `Theme` |
| Page templates | `AppShell`, `Page`, `AuthLayout`, `WizardLayout`, `PublicLayout` |
| Navigation | `AppShell` (sidebar, icon rail, dock), `NavHub`, `WorkspaceSwitcher`, `Popover`, `MenuButton`, `Segmented` |
| Actions | `Action` catalogue, `ActionButton`, `Button`, `IconButton`, `TextLink` |
| Forms | `Form`, `Field`, `TextInput`, `TextArea`, `PasswordInput`, `RuleChecklist`, `Select` (with sections), `Switch`, `Checkbox`, `ChipToggle`, `ChoiceCards` |
| Feedback | `Alert`, `Badge`, `Spinner`, `Skeleton`, `EmptyState`, toasts (`use_toaster`, close themselves), `Dialog` (bottom sheet on phones), `ConfirmDialog` |
| Data | `Card`, `RecordCard` + `Tag`, `RowList`, `PickList`, `FilterBar`, `ItemList`, `CheckList`, `Avatar`, `StatTile`, `BarChart`, `Sparkline`, `MonthStrip`, `Dashboard` |
| Landing | `Hero`, `Section`, `FeatureCard`, `StepCard`, `CompareCard`, `Illustration` |
| Layout | `Stack`, `Cluster`, `Grid`, `Split`, `Container`, `Spacer`, `Divider`, `Heading`, `Text`, `Code` |

Rules:

- Components are **closed**: no `class` or `style` props. Variants are enums.
- Icons only through the `Icon` enum. Action labels and icons only through the `Action` enum.
- A new pattern is added here first, with an entry in the gallery, before the app uses it.

## Styles

Styles are Tailwind CSS v4 + daisyUI 5, compiled **without Node.js**:

```sh
./scripts/build-css.ps1          # Windows (add -Watch while developing)
./scripts/build-css.sh           # macOS / Linux (add --watch)
```

The script downloads the pinned Tailwind standalone CLI into `tools/` (not committed) and writes
`assets/theo-ui.css`, which is committed. Run it after you change a component or
`css/input.css`. daisyUI 5.8.1 is vendored in `css/vendor/` (MIT licence).

## Gallery

```sh
dx serve --example gallery --platform web --features web
```

## Status

Scaffolding. Not yet published: `theo-app` uses it as a path dependency (`../theo-ui`).
