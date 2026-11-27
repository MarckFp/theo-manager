# M-04 Weekend meeting

The plan of the weekend meeting: the chairman, the public talk (speaker and talk number, with
hospitality for a visiting speaker), the Watchtower Study conductor and reader, and the prayers.
Sending works like the midweek meeting: only the people concerned get a notice.

Status: first version, 2026-11-25 (model and rules in `theo-core/src/weekend.rs`; no screen yet).
Key domain: **schedule**.

## Records

### Week (`weekend_weeks`)

| Field | Rules |
|---|---|
| `monday` | The week. The meeting date is Monday plus the congregation's weekend day (*Settings*). |
| `songs` | Opening, middle and closing song numbers. |
| `chairman`, `opening_prayer`, `closing_prayer` | People. |
| `speaker` | `Option`: a **local** speaker (a person with the *Public talk* privilege) or a **visiting** speaker (M-06). |
| `talk` | `Option`: the talk **number** only. Titles are not stored (they are not ours to store). |
| `host` | `Option`: hospitality for a visiting speaker (privilege *Hospitality*). Cleared for a local speaker. |
| `watchtower` | `Option`: a reference to the study article (issue `YYYYMM`, article number). The text stays in the device's EPUB (R-033). |
| `wt_conductor`, `wt_reader` | People. |
| `co_visit` | The circuit overseer's visit: he gives the public talk (no speaker, no talk, no host). |
| `no_meeting` | No meeting (convention or assembly). Nothing is assigned or sent. |
| `note` | `Option`: free text for the planners. |
| `sent` | What everyone sees: the date, each place and its person. |

### Settings (`weekend_settings`)

| Field | Default | Rules |
|---|---|---|
| `chairman_prays` | off | The chairman says the opening prayer: the opening-prayer place is left out. |
| `speaker_prays` | on | A **local** speaker says the closing prayer: the closing-prayer place is left out. A visiting speaker or the circuit overseer always leaves the place in. |

## Places and who can take them

| Place | Privilege | When |
|---|---|---|
| Chairman | Weekend chairman | Always |
| Opening prayer | Prayer | Unless the chairman prays |
| Public talk | Public talk | A local speaker or no speaker yet; not during the circuit overseer's visit |
| Hospitality | Hospitality | A visiting speaker |
| Watchtower Study conductor | Watchtower conductor | Always |
| Watchtower reader | Watchtower reader | Always |
| Closing prayer | Prayer | Unless a local speaker prays |

## Rules

- **Events (M-12, decision E1).** A new week reads the events for the whole congregation. A
  regional convention or circuit assembly on the meeting day gives *no meeting*. A circuit
  overseer's visit that week gives `co_visit`. Planners can change both.
- **Candidates** are people with the place's privilege who did not move away. They are sorted
  like every schedule (`scheduling.rs`):
  1. Available first (no absence that day).
  2. Then people with nothing else that day. "Busy" counts other weekend places, duties at the
     weekend meeting (M-05) and an outgoing talk that day (M-06).
  3. Then the longest without this place, then the longest without any weekend place.
  4. Then by name.
- **Autofill** fills empty places with the first free candidate. It never chooses the speaker:
  talks are arranged with the speakers.
- **The talk** gets warnings, but is not refused, when:
  - it is not in the speaker's list,
  - it is marked *not in use*,
  - or it is also given here within `repeat_months` (M-06 settings, default 6).
- **Send:** like the midweek meeting. Everyone sees the sent copy. People whose places are new,
  changed (also a changed date) or removed get a notice, and the week shows in *My assignments*.
- **Statistics:** assignments per person in a period (`weekend_per_person`).

## Open questions

1. Should the reader be chosen from a separate pool for the circuit overseer's visit week?
2. Do we need an interpreter place (privilege *Interpreter*) for bilingual congregations?
