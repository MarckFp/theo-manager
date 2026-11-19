# M-15 Meetings for field service

When and where the congregation meets for the ministry, and who conducts. Planners plan a month
(by hand, or from weekly presets with an automatic assignment) and **send** it: until then nobody
else sees it.

Status: first version, built on 2026-11-19. F1, F3 and F4 decided; one open question at the end.

## Records

### Preset (`service_preset`)

A meeting that happens every week.

| Field | Type | Rules |
|---|---|---|
| `weekday` | Sunday … Saturday | Required. |
| `time` | `HH:MM` | Required. |
| `place` | text | Required. Day, time and place together are unique. |
| `available` | set of persons | People with the privilege who **can** conduct this one. |
| `enabled` | bool | On by default. Off: *Fill month* skips it (for example for a few weeks); meetings already made stay. Switch it on again later. |

**Nobody is available by default** (decided 2026-11-19): a planner ticks, per preset, who can
conduct it. Someone who gets the privilege later is not assigned until they are ticked.

### Meeting (`service_meeting`)

| Field | Type | Rules |
|---|---|---|
| `date` | date | Required. |
| `time` | `HH:MM` | Required. |
| `place` | text | Required. |
| `conductor` | `Option<person>` | Empty is allowed (shown as "No conductor"). |
| `preset` | `Option<preset>` | The preset it was made from (for availability). Deleting the preset keeps the meeting. |
| `sent` | `Option<date, time, place, conductor>` | The version everyone sees. Empty: never sent. |
| `removed` | bool | Deleted after it was sent; disappears for everyone at the next send. |

## Who can conduct

The privilege **Meeting for field service** (M-01, "Privileges"; baptised publishers). For a
meeting on a day, each person is:

| State | When |
|---|---|
| Available | Has the privilege, no absence that day, available for the preset, no other meeting that day. |
| Not available at this time | Not ticked in the meeting's preset. |
| Already conducts that day | Conducts another meeting the same day. |
| Away | Has an absence that day (their own, or one of the whole congregation). |

In the meeting form the conductor list shows the people with the privilege first (available ones
on top, each with its state), then the other baptised publishers, for when nobody is available.
Choosing someone who is not available is allowed, with a note under the field.

## Filling a month

*Fill month* (planners):

| Choice | What it does |
|---|---|
| Fill | Adds a meeting for every preset on every matching day of the month, without a conductor. A day that already has a meeting at the same time and place is skipped. Days when the whole congregation is away (an assembly or a convention, M-12) are skipped. |
| Fill and assign | Fill, then *Assign the empty ones*. |
| Assign the empty ones | Gives a conductor to every meeting of the month that has none. Conductors already chosen stay. |
| Clear the month | Deletes every meeting of the month (asks first). |

**Automatic assignment**, meeting by meeting in date order: only people who are *Available*
(privilege, no absence, available for the preset, no other meeting that day). Among them, the one
with the fewest meetings that week, then the fewest that month, then the longest time since or
until their nearest other meeting; then by name, so the result is always the same. When nobody is
available the meeting stays without a conductor and a warning says how many.

## Sending

Planners see the meetings as they are now. Everyone else sees only the **sent** version.

| State (planners) | Meaning | Everyone else sees |
|---|---|---|
| (no tag) | Sent | The meeting |
| Not sent | New since the last send | Nothing |
| Changed · not sent | Changed after it was sent | The sent version |
| Removed · not sent | Deleted after it was sent | The sent version |

*Send* sends the shown month: every change of that month becomes visible, removed meetings
disappear. The button shows how many changes are waiting, and a confirmation lists them (new,
changed, removed). A meeting added or changed after a send is again "Not sent" until the next
send.

## Notices and "My assignments"

Sending notifies **only the conductors concerned** (decided 2026-11-19), never everyone:

| Change sent | Notice |
|---|---|
| New meeting with a conductor | *New* for the conductor |
| Changed (same conductor) | *Changed* for the conductor |
| Given to someone else | *Cancelled* for the old conductor, *New* for the new one |
| Removed | *Cancelled* for the conductor |

A newer notice about the same meeting replaces an unseen older one (new then changed stays
*New*; new then cancelled before it was seen disappears).

The home page widget **My assignments** (first on the home page by default) shows, for the signed-in person (their account linked to
their person, M-02): first their unseen notices with *Got it* to mark them seen, then their
upcoming assignments (only what was sent). Other assignments (meeting parts, duties) join this
widget when their modules exist. *All my assignments* opens a full page
(`/app/me/assignments`): a timeline grouped by month, with *Coming up* and *Past*, the next one
highlighted, new and changed ones marked, and cancelled ones listed with the notices. Notices are in the app; push notifications or e-mail come later
with notifications in general.

## Screens

*Ministry › Meetings for field service* (`/app/ministry/service-meetings`). The tabs and the month
stay at the top.

- **List:** one card per meeting: day and time, place, conductor (or "No conductor"), and for
  planners the send state. A short alert sums up what is not sent and which meetings have no
  conductor.
- **Calendar:** the shared month calendar (M-12), with the place under the time and conductor
  on large screens. For planners: blue sent, yellow not sent, red no conductor.
- **Presets** (planners only): one card per weekly meeting with how many can conduct it and
  "Off: not filled" when switched off; the dialog sets the day, time, place, on or off, and who
  can conduct it (ticked: available).
- Header actions (planners): *Fill month*, *Send (n)*, *Add meeting*; on the Presets tab *Add
  preset*.

## Permissions

The page *Ministry › Meetings for field service* (M-02): **write** plans (presets, meetings,
fill, send); **read** sees the sent meetings. Administrators always write.

## Decisions (2026-11-19)

| # | Question | Decision |
|---|---|---|
| F1 | Who is notified when a month is sent? | Only the conductors concerned, in *My assignments* on the home page (see "Notices"). |
| F3 | Sisters as conductors? | The privilege rule stays "baptised publishers": unusual, but it can happen. Planners give the privilege to who should conduct. |
| F4 | Availability by default? | Nobody is available until ticked in the preset. |

## Open questions

| # | Question | Proposal |
|---|---|---|
| F2 | More than one conductor per meeting (or an assistant)? | One for now. |
