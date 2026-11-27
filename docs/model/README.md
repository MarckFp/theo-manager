# Data model

One file per module (`REQUIREMENTS.md` 1.3). Each file says which records the module has, which
fields they have, and the rules that `theo-core` enforces. The Rust types in `theo-core/src`
follow these files. `theo-core/src/collections.rs` gives each collection its key domain.

| Module | File | Status |
|---|---|---|
| M-01 People (persons and families) | [m01-people.md](m01-people.md) | Reviewed 2026-11-18, 1 open question |
| M-02 Users and access | [m02-users-and-access.md](m02-users-and-access.md) | Reviewed 2026-11-18 |
| M-09 Field service groups | [m09-field-service-groups.md](m09-field-service-groups.md) | Reviewed 2026-11-17, all decided |
| M-10 Field service reports | [m10-field-service-reports.md](m10-field-service-reports.md) | Reviewed 2026-11-17, all decided |
| M-03 Midweek meeting | [m03-midweek-meeting.md](m03-midweek-meeting.md) | First version 2026-11-24, 2 open questions |
| M-04 Weekend meeting | [m04-weekend-meeting.md](m04-weekend-meeting.md) | First version 2026-11-25 (model only), 2 open questions |
| M-05 Duties | [m05-duties.md](m05-duties.md) | First version 2026-11-25 (model only), 2 open questions |
| M-06 Public talks | [m06-public-talks.md](m06-public-talks.md) | First version 2026-11-25 (model only), 2 open questions |
| M-07 Territories | [m07-territories.md](m07-territories.md) | First version 2026-11-20, 2 open questions |
| M-08 Public witnessing | [m16-public-witnessing.md](m16-public-witnessing.md) | First version 2026-11-19 |
| M-11 Meeting attendance | [m11-meeting-attendance.md](m11-meeting-attendance.md) | First version 2026-11-19, 2 open questions |
| M-12 Events | [m12-events.md](m12-events.md) | First version 2026-11-19, 2 open questions |
| M-15 Meetings for field service | [m15-field-service-meetings.md](m15-field-service-meetings.md) | First version 2026-11-19, 1 open question |
| M-14 Elderly care | [m14-elderly-care.md](m14-elderly-care.md) | Proposal, 6 open questions |
| Other modules | Written in each module's modeling session | Not started |

## Conventions for every module

**One identifier per record:** `record_id`, a 128-bit random value (`REQUIREMENTS.md` 5.1).
Links between records use it, and the app uses it in its own URLs on the device. Users never see
it: screens and exports show names. It is random, not incremental, so the server cannot see how
many records a congregation has or in which order they were created. (Decided on 2026-11-17: no
visible number.)

**Optional fields** are written as `Option<…>` in the field tables. An empty text is stored as
"no value", never as `""`.

**Dates** are calendar dates without a time and a time zone (`YYYY-MM-DD` inside the record). The
UI shows them in the user's date format (R-062).

**Deletes** are soft (a tombstone, R-013) until a purge (R-021). The tables say what happens to the
links when a record is deleted.

**Key domains.** Each collection belongs to one key domain (6.2). The module file names it.

**Derived values** (for example the activity status of a publisher) are computed on the device
from other records. They are never stored, so they cannot get out of date.
