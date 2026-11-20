# M-07 Territories

The congregation's territories: each has a map, the addresses that must not be called on, and
who has it. The territory servant assigns territories to publishers, takes them back when they
are completed, and sees which ones are late. The registry keeps every assignment, like the S-13.

Status: first version, built on 2026-11-20.

## Records

### Territory (`territory`)

| Field | Type | Rules |
|---|---|---|
| `title` | text | Required. **Unique** in the congregation (case and spaces ignored). Usually a number ("12") or a name ("Old town 3"). Lists sort numbers as numbers: 2 before 10. |
| `category` | `Option<category>` | One category of the congregation, for example *Business*, *Residential* or *Rural*. |
| `labels` | set of labels | Labels of the congregation, for example *Has an elevator* or *Difficult access*. |
| `shapes` | list of shapes | The map: **areas** (closed boundaries; several are allowed), **lines** (open, for limits like "only this side of the street") and **markers** (a point with a short title, for example "Gate code at the intercom", and a colour: blue, green, yellow, red or grey). An area keeps at least 3 corners, a line 2. |
| `view` | `Option<center + zoom>` | The view the map opens with. None: the map fits the shapes. |
| `description` | `Option<text>` | Anything publishers should know. |
| `do_not_calls` | list | Addresses that must **not** be called on: `address` (required), `note` and `date` (optional). Shown in red at the top of the territory page and on the card. |
| `url` | `Option<text>` | A link, for example to the territory in another app. Starts with `https://` or `http://`. |

Deleting a territory deletes its registry lines.

### Category and label of territories

The same as the labels of people (M-01): `name` (unique, required) and `color`. They are separate
from the labels of people. Deleting one takes it off every territory.

### Assignment label

Like a category (`name`, `color`), for assignments. A new congregation starts with *Memorial
campaign*, *Regional convention campaign* and *Special campaign*; they can be renamed or deleted.

### Assignment (`territory_assignment`): one line of the registry

| Field | Type | Rules |
|---|---|---|
| `territory` | territory | Required. |
| `person` | person | Required. Not a Bible student or visitor, and not moved away. |
| `assigned` | date | Required. Not in the future. |
| `completed` | `Option<date>` | When it came back worked. Not before `assigned`, not in the future. None: the person has it now. |
| `labels` | set of assignment labels | For example *Memorial campaign*. Chosen when assigning, completing or correcting. |

**One person at a time**: a territory has at most one assignment without `completed`. Two
assignments of the same territory never overlap.

## Settings

| Setting | Rules |
|---|---|
| Late after | 1 to 730 **days**, default **120**. Shown also in months (120 days: about 4 months; 70: about 2.3). A territory is **late** when it has been assigned for longer. |
| Map images | Off for a new congregation. Turning them on shows the privacy notice (R-045). Off: the shapes are drawn on a plain background. |
| Map provider | OpenStreetMap (default), OpenTopoMap or Esri satellite. Only providers that need no API key (decided 2026-11-20). |
| Starting point | Where a new territory's map opens when there is nothing to fit yet. |
| Categories, labels, assignment labels | See above. |

## Derived values

| Value | Rule |
|---|---|
| Available | No assignment without `completed`. |
| Last completed | The newest `completed` of the territory. |
| Late | Assigned now, and `assigned` + *late after* months ≤ today. |
| Average time assigned | The days from `assigned` to `completed` of the assignments completed in the period. |
| Covered | Different territories completed in the period, out of all. |

## Screens

*Ministry › Territories* (`/app/ministry/territories`). Header: *Settings* (write), *Export*,
*Add territory* (write). Search (title, description, do-not-call addresses, markers, who has it)
and filters (status, category, label) apply to every list and to the exports.

- **Assignments** (the default tab):
  The parts are **Assigned** (first), **Available** and **Registry**.
  - **Available**: the territories nobody has, the longest since completed first (never completed
    before all). Tapping one assigns it: publisher (with how many they already have) and date
    (today).
  - **Assigned**: who has which territory since when, **late ones first** in red with a warning
    above the list. Tapping one marks it *completed* (date, today by default) or corrects it.
  - **Registry**: one column per territory with its title and last completed date, then every
    assignment **newest first** (publisher, assigned – completed). Blue: assigned now; red: late.
    Tapping a cell corrects or deletes it; *Add a past assignment* copies the paper registry.
- **Territories**: a list of cards (category, labels, do-not-calls, who has it, "No area on the
  map"), or **one map** with every territory (green available, blue assigned, red late); tapping
  one opens it.
- **Statistics** (last 12 months, 24 months or all): territories, late, average time assigned,
  covered; territories per category and per label; not worked for the longest time;
  assignments per label; per publisher (completed in the period and how many they have now).

**A territory** (`/app/ministry/territories/:id`): do-not-calls in red at the top, category and
labels, the map (other territories in grey), details, who has it with *Completed…* or
*Assign…*, and the history. *Directions* (opens the device's map app at the middle of the
territory: Google Maps in web browsers and the desktop app, the default map app in the
Android app, Apple Maps in the iPhone and iPad app), *Card (PDF)* and *Edit*. The target button on the map saves the
current view as the default view (write).

**Editor** (`/app/ministry/territories/new`, `…/:id/edit`): title, category, labels (with
*Categories* and *Labels* to manage them), description, link; the map editor (draw areas and
lines, add markers, move or add corners, delete, set the default view, import **GeoJSON or KML**,
export GeoJSON); the do-not-call list.

**The map** (everywhere): a full-screen button (also in the editor, so a phone does not scroll the
page by mistake); a saved view that is too close for a small screen gives way to one that fits
the territory. In the editor, tapping a marker opens it (title, colour, delete); tapping a corner
selects it and the minus button deletes it; on a computer, right-click deletes a corner (and,
while drawing, takes back the last point). On touch screens the marker dialog opens a moment
after the tap, so the tap does not close it at once. *Delete* asks first.

**Settings** (`/app/ministry/territories/settings`): late after, map images and provider (with
a preview where the starting point is set), categories and labels.

**Home**: *My territories* lists the territories I have now (late ones marked); tapping one opens
its page. *Territories* shows how many are assigned and late.

## Exports

| Export | Content |
|---|---|
| Territory cards (PDF) | One A4 page per territory shown (or the one open): title, category, labels, the map with the tiles and the shapes drawn on top, the provider's attribution, the do-not-calls in a red box, the description, the link, and the print date. Without map images (off, or offline) the shapes are drawn on a plain background. |
| Registry (PDF, Word, CSV) | Per territory, newest first: publisher, assigned, completed (or "Still assigned"), days. |
| GeoJSON (one territory) | Areas as polygons, lines, markers as points with their title. |

The card's map image is drawn by a short canvas script (the only way to read tile pixels inside
the webview, exception to P1 noted in R-046). Tiles are read once per card; no area is
downloaded in bulk (R-042).

## Permissions

The page *Ministry › Territories* (M-02): **write** manages territories, assignments and
settings; **read** sees everything without changing it. A publisher without access sees only the
territories they have now (from *My territories*), with the map, the do-not-calls and the card.

## Open questions

1. Should a publisher be able to mark their own territory as completed (a request the territory
   servant confirms)?
2. Campaigns (for example the Memorial invitation): a separate "worked for the campaign" date per
   territory, outside the S-13 coverage?
