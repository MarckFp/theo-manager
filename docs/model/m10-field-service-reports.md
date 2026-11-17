# M-10 Field service reports

Every publisher reports their ministry once a month. At the start of a month they report the
month before: in November 2026 the **reporting month** is October 2026. The reports drive the
activity status of each publisher (M-01), the goals of pioneers and the statistics of the
congregation.

Status: reviewed on 2026-11-17. All questions are decided (see the end).

## Who reports what

Only current publishers report: every publisher type except Bible student and visitor (M-01).
People who moved away do not report.

| Kind of publisher | Took part (yes or no) | Hours | Credit hours | Bible studies | Notes | Goal |
|---|---|---|---|---|---|---|
| Publisher (baptised or not) | ✅ required | | | ✅ required | optional | none |
| Auxiliary pioneer **for that month** (baptised publishers) | | ✅ required | | ✅ required | optional | 30 h or 15 h for that month |
| Continuous auxiliary pioneer | | ✅ required | ✅ optional | ✅ required | optional | 30 h or 15 h every month |
| Regular pioneer, special pioneer, missionary | | ✅ required | ✅ optional | ✅ required | optional | 600 h per service year |

- **Total hours** = hours + credit hours. Goals use the total.
- **Credit hours** are at most **50 per month**.
- Hours, credit hours and Bible studies are whole numbers from 0.
- For people who report hours, "took part" is true when the total is more than 0.
- **Monthly auxiliary pioneering** (baptised publishers) is planned per person and month: ahead of
  time, or once the month has started, on the person page (*Auxiliary pioneering*), or with the
  switch in that month's report. It changes only that month. Planned months are stored as
  `aux_plan` records (person, month).

## Report

Collection `fs_report`, key domain `reports`, `scope_id` = the person's home group (M-09).

| Field | Type | Rules |
|---|---|---|
| `person` | person `record_id` | A current publisher. |
| `month` | year and month | Not after the reporting month. One report per person and month. |
| `took_part` | bool | Required for publishers; derived from the hours for the others. |
| `hours` | `Option<u32>` | Required for pioneers and monthly auxiliary pioneers; none for others. |
| `credit_hours` | `Option<u32>` | Continuous auxiliary, regular and special pioneers and missionaries only. At most 50. |
| `bible_studies` | `u32` | Required. |
| `auxiliary_pioneer` | bool | Only for baptised publishers; makes `hours` required. |
| `notes` | `Option<text>` | At most 1,000 characters. |

The publisher type of the report is the person's type **when the report is saved**. A later
change of type does not change old reports, so the statistics of past years stay correct. (The
scaffold uses the person's current type.)

## Goals

| Goal | Value | Counted over |
|---|---|---|
| Auxiliary pioneer (monthly and continuous) | **30 h**, or **15 h** in months the congregation marks | One month |
| Regular pioneer, special pioneer, missionary | **600 h** | The service year: September to August |

**15-hour months.** By default every month has a 30-hour goal. The congregation marks the months
with a 15-hour goal, usually the Memorial campaign, the regional convention campaign and the
month of the circuit overseer's visit. The setting is per service year, in the **Goals** tab.
Later, the calendar (M-12) can propose these months from its events.

**Pace.** For a yearly goal the app shows how far the pioneer should be by now: 600 h × months
reported so far ÷ 12. A pioneer at or above that line is "On track", below it "Behind", with the
number of hours.

## When a report is missing

Reports of the reporting month are due at the start of the month.

| Day of the current month | A missing report of the reporting month shows | Tone |
|---|---|---|
| 1 to 4 | Not in yet | neutral |
| 5 to 14 | Missing | warning |
| 15 and later | Late | error |

A missing report of any earlier month is always **Late**.

## Activity status

The activity status of M-01 is computed from these reports, over the last 6 complete months:
a month counts as **active** when the person reported that they took part (for pioneers: more
than 0 hours). Months before the person's first report do not count.

## The Field service reports screen

*Users › Field service reports* has three tabs.

| Tab | What it shows |
|---|---|
| **Month** | A month picker across the full width (previous, next, and a popover to jump to any month and year; starts at the reporting month). Totals: reports received, publishers who took part, hours, Bible studies. Search and filters: **Report status** (received, not in yet, missing, late) and the *All users* filters except status and consent (type, appointment, group, activity, gender, anointed). One card per publisher with their report, or its status. A card opens the report form. Export of the month as PDF, DOCX or CSV. |
| **Statistics** | A service year. Averages and totals for the congregation and for each field service group, and publishers who took part per month. |
| **Goals** | A service year and the goal progress of everyone with a goal, grouped by kind: regular pioneers, special pioneers, missionaries, continuous auxiliary pioneers, auxiliary pioneers for single months. Empty groups are not shown. **Goal settings** (next to the service year) opens a dialog with the 15-hour months of that year and the goal of each kind. |

The person page shows the person's last 6 months, their recent reports, their goal progress, and
a button to enter the report of the reporting month. The dashboard widget shows how many reports
of the reporting month are in.

## Decisions (2026-11-17)

| # | Question | Decision |
|---|---|---|
| R1 | Goal of special pioneers and missionaries? | 600 h per service year, like regular pioneers. |
| R2 | When is a missing report late? | A warning from the 5th, late from the 15th (see [When a report is missing](#when-a-report-is-missing)). |
| R3 | Who may enter reports? | The publisher themselves (with an account) and users whose role has the permission (M-02). |
| R4 | When is monthly auxiliary pioneering set? | Ahead of time, or once the month has started. Both are possible. |
| R5 | Does a report that says "did not take part" count? | It counts as received, not as active. |
