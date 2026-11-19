# M-11 Meeting attendance

How many people attend each meeting, recorded month by month like the paper record: five
columns (week 1 to week 5) and two rows (midweek meeting and weekend meeting), with the total and
the average of each row.

Status: first version, built on 2026-11-19. Open questions at the end.

## Record

Collection `meeting_attendance`, one record per month that has at least one number.

| Field | Type | Rules |
|---|---|---|
| `month` | year and month | Unique. Not after the current month. |
| `midweek` | 5 × `Option<number>` | Week 1 to week 5. Whole numbers from 0 to 9,999. |
| `weekend` | 5 × `Option<number>` | Same. |

- **Count everyone** at the meeting: in person and by video conference together.
- **Empty means no meeting** that week (for example during a circuit assembly or the
  convention), or not entered yet. Empty weeks do not count as meetings.
- A month whose numbers are all emptied is removed.
- Week *n* is the *n*-th meeting of that kind in the month. The page shows the date of each
  week's meeting in the empty input ("Thu 5"), from the meeting days in the congregation settings.
  A month has 4 or 5 of them; the fifth column stays available in every month.

## Totals and averages

For one meeting over a period (a month or a service year):

| Value | Rule |
|---|---|
| Meetings | Number of weeks with a number. |
| Total | Sum of the numbers. |
| Average | Total ÷ meetings, shown as a whole number. Weeks without a meeting do not lower it. |

## Screens

*Meetings › Meeting attendance* (`/app/meetings/attendance`). The tabs and the service year
(September to August) stay at the top.

**Months tab.** One card per month of the service year up to the current month, newest first:
the average of each meeting and the number of meetings. A past month with nothing entered is
marked "Not entered". Opening a card shows the month.

**Month page** (`/app/meetings/attendance/YYYY-MM`). The 5 × 2 grid (on phones one block per
meeting), with meetings, total and average per row. Changes save as you type. A month picker
moves to other months. Below: the average of each meeting compared with the month before and
the same month a year earlier.

**Statistics tab**, for the chosen service year:

| Part | Content |
|---|---|
| Key numbers | Average of each meeting, compared with the service year before; the highest attendance (which meeting, which date); the weekend average as a share of the current publishers. |
| Average attendance per month | Line chart, midweek and weekend, September to August. Legend, the last values at the end of the lines, a tooltip per month (hover or keyboard). |
| Months by average attendance | Every month with numbers, highest first (both meetings together). |
| Highlights | Best and lowest month, how many more people come to the weekend meeting than to the midweek meeting, meetings recorded and people in total. |
| Yearly record | The table behind the chart: per month, meetings, total and average of each meeting, and the service-year totals. |

**Export** (PDF, DOCX, CSV): the yearly record of the chosen service year.

The chart uses two series colours of the design system (`--chart-1`, `--chart-2`, one pair per
theme). They were checked for colour-blind separation and contrast against the page in light and
dark mode.

## Permissions

The page *Meetings › Meeting attendance* has the usual levels (M-02): read shows the months and
statistics, write also changes the numbers.

## Open questions

| # | Question | Proposal |
|---|---|---|
| A1 | Record in person and by video conference separately? | One number for now, as on the paper record. Two numbers per meeting can be added later without losing data. |
| A2 | Record special meetings (the Memorial, the circuit overseer's visit, assemblies)? | Not on this page; they belong to *Events*. |
