# M-12 Events

Days that change the normal life of the congregation: the regional convention, the circuit
assembly, the Memorial, the circuit overseer's visit, and any custom event. An event can be for
the whole congregation or only for chosen people, and it can make them away from the meetings
(an absence, M-01).

Status: first version, built on 2026-11-19. Presets and E1 decided; open questions at the end.

## Event

Collection `event`.

| Field | Type | Rules |
|---|---|---|
| `kind` | regional convention, circuit assembly, Memorial, circuit overseer's visit, custom | Required. |
| `title` | text | Required. |
| `description` | `Option<text>` | |
| `from` | date | Required. |
| `to` | date | Required, not before `from`. |
| `people` | `Option<list of persons>` | None: the whole congregation. A list: only these people (at least one). |
| `absence` | bool | The event creates absences on its days. |

### Presets

Choosing a type fills in the form. The user can change every value afterwards.

| Type | Title | Days | Absence |
|---|---|---|---|
| Regional convention | "Regional convention" | 3 | Yes |
| Circuit assembly | "Circuit assembly" | 1 | Yes |
| Memorial | "Memorial" | 1 | Yes |
| Circuit overseer's visit | "Circuit overseer's visit" | 6 | No |
| Custom event | (empty) | 1 | No |

- A title the user typed is kept when the type changes; a preset title is replaced.
- Changing the first day keeps the length of the event.

### Who sees an event

- **Whole congregation:** everyone who can open *Events*.
- **Chosen people:** only those people and the administrators. Others do not see it in the list
  or the calendar.

### Absences

When `absence` is on, the event creates absences (M-01, "Absences") for its days:

- Whole congregation: one absence of the whole congregation.
- Chosen people: one absence per person.

These absences belong to the event: saving the event replaces them, deleting the event deletes
them. On *Users › Absences* they show an "Event" tag and open read-only, with a link to *Events*.
The absences of a private event carry no notes, so the title stays private; people who cannot
see the event read "It comes from an event you cannot see".

## Screens

*Meetings › Events* (`/app/meetings/events`), with two views (the choice is kept while the app is
open):

**List.** Cards with the title, the dates, the type, "Now", the number of days, the people of a
private event and "Away from meetings". Search by title or description. Filters: when (now,
coming up, past; starts with now and coming up), type, and for whom.

**Calendar.** A month calendar (`theo-ui` `Calendar`, reusable for other modules). On larger
screens events that last several days are bars across the days. On phones the month is a compact
grid with a dot per event on each day, and the list of the month below it; tapping a day lists
that day's events. The week starts on the user's first day of the week (R-062).

**Add or change an event:** a dialog with the type, title, first and last day, description, the
absence switch, and for whom (whole congregation or chosen people, with a search; the switch
comes first so it does not move down when the list of people opens). Delete asks
first and says that the absences are deleted too.

## Decisions (2026-11-19)

| # | Question | Decision |
|---|---|---|
| E1 | Should events change the meeting schedules (no midweek meeting during the circuit assembly week, the circuit overseer's talk, the Memorial)? | **Yes.** Built with the meeting modules (M-03 Midweek, M-04 Weekend): their schedules read the events of each week. Until then events only create absences. |
| E4 | Presets | As in the table above, confirmed; the Memorial creates an absence by default. |

## Open questions

| # | Question | Proposal |
|---|---|---|
| E2 | Reminders before an event (notifications)? | Later, together with notifications in general. |
| E3 | Repeat events (every year)? | No: the dates change every year. |
