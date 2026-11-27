# M-05 Duties

The *Departments* section (D-28):
- **Audio, video and stage**, **attendants** and **microphones** are scheduled per meeting (midweek
  and weekend).
- **Cleaning** goes round the field service groups.
- **Maintenance** is a list of tasks with due dates.

Status: first version, 2026-11-25 (model and rules in `theo-core/src/duties.rs`; no screens yet).
Key domain: **schedule**.

## Duty types are data (I-10)

A congregation starts with the default types below. It can rename them, change how many people
they need and at which meetings, reorder them, delete them, and add its own (for example
sign-language interpreting).

| Department | Duty type | People | Who | Midweek | Weekend |
|---|---|---|---|---|---|
| Audio, video and stage | Audio | 1 | Privilege *Audio* | ✓ | ✓ |
| | Video | 1 | *Video* | ✓ | ✓ |
| | Stage | 1 | *Stage* | ✓ | ✓ |
| | Zoom host | 1 | *Zoom attendant* | ✓ | ✓ |
| | Zoom attendant | 1 | *Zoom attendant* | ✓ | ✓ |
| Attendants | Entrance | 1 | *Entrance attendant* | ✓ | ✓ |
| | Hall | 2 | *Attendant* | ✓ | ✓ |
| Microphones | Roving microphones | 2 | *Microphones* | ✓ | ✓ |

## Records

### Duty type (`duty_types`)

| Field | Rules |
|---|---|
| `department` | Audio, video and stage; Attendants; or Microphones. Cleaning and maintenance are not scheduled per meeting. |
| `name` | Required, unique in its department. |
| `slots` | 1 to 10 people per meeting. Lowering it removes the extra people from meetings that are not sent. |
| `eligible` | A **privilege** (M-01), or a **list of people** for custom duties. People who moved away are left out. |
| `midweek`, `weekend` | At which meetings. At least one. |
| `position` | Order in the department. |

### Duty meeting (`duty_meetings`)

| Field | Rules |
|---|---|
| `monday`, `meeting` | The week and the meeting (midweek or weekend). The date comes from the congregation's meeting days. |
| `slots` | The filled places: duty type, place number, person. |
| `sent` | What everyone sees. |

A meeting has **no duties** when its week says *no meeting* (M-03 or M-04).

### Cleaning (`cleaning`)

| Field | Rules |
|---|---|
| `groups` | Field service groups (M-09) in turn order. Unknown groups are left out. |
| `start` | The first group's first week (saved as its Monday). |
| `weeks_each` | Weeks per group, at least 1. |
| `overrides` | (week, group) exceptions. |

The group of a week is the exception for that week, or the rota from `start`. There is none
before `start` or without groups.

### Maintenance task (`maintenance_tasks`)

| Field | Rules |
|---|---|
| `title` | Required. |
| `every_days` | `Option`: the task repeats this many days after it is done. None: it is done once. |
| `due`, `last_done` | Marking a task done sets `last_done`. A repeating task is then due again `every_days` later; a one-off task has no due date any more. |
| `people` | Who looks after it. |

## Rules

- **Who:** only people who may do the duty can be assigned (`may_do_duty`).
- **Candidates and autofill:** sorted like every schedule (`scheduling.rs`).
  - "Busy" counts other duties at that meeting and the person's parts in that meeting (M-03 or
    M-04).
  - Autofill fills each empty place with the first free candidate, so nobody gets two places at
    one meeting.
- **Send:** per meeting. The people whose duties are new, changed or removed get a notice. The
  duty shows in *My assignments* (for example "Midweek meeting · Hall 2").

## Open questions

1. Should cleaning also be assignable to people (not only groups) for small congregations?
2. Should maintenance tasks send reminders to their people when due?
