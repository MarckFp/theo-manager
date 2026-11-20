# M-08 Public witnessing (shifts)

Shifts at the carts and stands: a date, a start and end time, a place and the people who take
part. The flow is the same as meetings for field service (M-15, file `m15-field-service-meetings.md`):
weekly presets with availability, filling and assigning a month, sending it, and notices for the
people concerned. In addition a shift needs **several people of different kinds**, and the
congregation can use **shift supervisors**.

Status: first version, built on 2026-11-19.

## Settings

| Setting | Rules |
|---|---|
| Shift supervisors on | Off by default. Switching it off removes the supervisor places from the presets. |
| Who can be a shift supervisor | People with the privilege *Public witnessing* (brothers or sisters). Shown only when supervisors are on. |

## Preset (`shift_preset`)

| Field | Rules |
|---|---|
| `weekday`, `start`, `end`, `place` | Required; the end after the start. |
| `needs` | How many **shift supervisors** (only when they are on), **brothers**, **sisters** and **anyone** (any gender). At least one person. Supervisors are places of their own: "1 supervisor + 1 brother + 1 sister" is 3 people. |
| `available` | People with the privilege who can take part at this time. **Nobody until ticked** (as in M-15). |
| `enabled` | Off: *Fill month* skips it; shifts already made stay. |

## Shift (`shift`)

| Field | Rules |
|---|---|
| `date`, `start`, `end`, `place` | Required; the end after the start. |
| `preset` | Optional link (for availability). |
| `slots` | The places, each with a kind (supervisor, brother, sister, anyone) and a person or empty. Nobody twice in a shift. |
| `sent`, `removed` | As in M-15: everyone else sees only the sent version. |

**How many people of each kind is decided only in the presets.** A shift made from a preset starts
with the preset's places. When a shift is changed or added by hand, the planner only decides the
**members**: the places show as a list ("Michael Brown · Shift supervisor"), tapping one opens the
person picker for that kind, × takes the person out (an empty place goes away), *Add person* adds
anyone (more than the preset is fine) and *Add supervisor* adds a supervisor place.

**No supervisor:** when shift supervisors are on and a shift has none, the planner is warned (in the
form, with a "No supervisor" tag on the card and in the month summary). It can still be saved.

## Who fits a place

| State | When |
|---|---|
| Available | Privilege, right gender (brother or sister places), a supervisor (supervisor places), no absence, ticked in the preset, no other shift that day. |
| Not available at this time | Not ticked in the preset. |
| Already has a shift that day | In another shift the same day. |
| Away | An absence that day (their own or the whole congregation's). |

The picker lists who fits the kind of place (available first), then the other baptised
publishers of the right gender for when nobody fits (not for supervisor places).

## Filling and assigning

As in M-15: *Fill*, *Fill and assign*, *Assign the empty places*, *Clear the month*. The
assignment fills the empty places of each shift (supervisors first, then brothers, sisters,
anyone) with people who are *Available*; among them the fewest shifts that week, then that month,
then the longest since or until their nearest other shift, then by name. Places nobody fits stay
empty, and a warning says how many.

## Sending and notices

As in M-15. On send, only the people concerned get a notice in *My assignments*: added to a shift
(*New*), taken out of it (*Cancelled*), or still in it when its date, time or place changed
(*Changed*). Their shifts appear in *My assignments* (with "Shift supervisor" when they are one).

## Screens

*Ministry › Public witnessing* (`/app/ministry/witnessing`). Tabs: *List*, *Calendar*, and for
planners *Statistics*. Header: *Presets* (planners), *Export* (the shown month as PDF, DOCX or CSV:
date, time, place, supervisor, people), *Fill month*, *Send (n)*, *Add shift*.

- **List:** one card per shift: day and time, place and the people, a tag with how many places
  are filled ("3 people", "2 of 3 people", "Nobody yet"), the supervisor, and the send state.
- **Calendar:** the shared month calendar; on large screens the people under each shift.
- **Presets** (a page of its own, `/app/ministry/witnessing/presets`): the *Shift supervisors* card (switch, how many can supervise, *Choose who*), then one
  card per weekly shift with what it needs and how many are available.
- **Statistics** (last 6 or 12 months, sent shifts): shifts, places filled, how many took part, shifts
  per person on average; shifts per person (with how many as supervisor; people without a shift
  marked); availability per preset, fewest available first (red when not enough for one shift);
  highlights (fewest and most available preset, most shifts, who had none).

## Permissions

The page *Ministry › Public witnessing* (M-02): **write** plans and sends; **read** sees the sent
shifts.
