# M-03 Midweek meeting

The plan of the *Our Christian Life and Ministry* meeting: each week's sections and parts (from
the official workbook), who has each part in the main hall and, when it is used, in the auxiliary
classroom, and sending the assignments to the people concerned.

Status: first version, built on 2026-11-24.

## Where the content comes from (REQUIREMENTS.md 1.6, D-15)

The app reads the official **Life and Ministry Meeting Workbook EPUB** on each device. It never
scrapes wol.jw.org (R-031), and it never uploads publication text to a server (R-032).

1. **The link:** jw.org's public download service answers with the EPUB address of an issue
   (`GETPUBMEDIALINKS`, `pub=mwb`, the issue `YYYYMM` of an odd month, the language). This
   request is allowed from browsers too.
2. **The file:**
   - Desktop and phone: downloaded **in the background**, in a background thread.
   - Web: the browser cannot read the EPUB from jw.org, because the server does not allow it
     (CORS, spike S8), and there is no relay server on purpose (D-44). The page shows the official
     link: the user downloads it and picks the file (R-035).
3. **Reading:** the reader (`src/workbook.rs`) does not depend on the language.
   - Sections are told apart by the colour class of their headings: teal is *Treasures from
     God's Word*, gold is *Apply yourself to the field ministry*, maroon is *Living as
     Christians*.
   - Parts come from their numbered headings, and minutes from the first number in parentheses.
   - Songs come from the song links.
   - Weeks follow each other from the first Monday of the issue's first month.
4. **When:** when a week is shown that has no plan and no workbook on the device, the app gets the
   workbook by itself (once per week and session). One button, *Get the plan* (or *Refresh plan*
   when the week has one), downloads, reads and plans every week of the issue; refreshing keeps
   the assignments and the planners' changes. The EPUB file is never kept: only what was read
   from it stays on the device. On the web, when the browser cannot download it, the two steps
   (download, choose the file) show only for the week that has no plan yet.

The workbook is kept **on the device** (in this browser on the web), keyed by issue and language.
The shared plan keeps only **references** (issue, language, document, part number), minutes,
kinds, songs, the reading and the assignments (R-033). Titles and descriptions are shown from
the device's workbook, unless a planner typed their own.

## Records

### Week (`midweek_week`)

| Field | Rules |
|---|---|
| `monday` | The week. The meeting date is Monday plus the congregation's midweek day (*Settings*). |
| `source` | `Option`: issue, language and document in the workbook. None: planned by hand. |
| `reading`, `songs` | The Bible reading and the opening, middle and closing song. Editable. |
| `sections` | In order. Each has a kind (Treasures, Ministry, Living, or Other for sections a planner adds, with a title) and parts. |
| `chairman`, `opening_prayer`, `closing_prayer`, `aux_counselor` | People. The counselor only when the auxiliary classroom is used. |
| `aux_on` | The auxiliary classroom is used this week. New weeks follow the setting; each week can switch it. |
| `no_meeting` | No meeting (for example a convention). Nothing is assigned or sent for that week. |
| `sent` | What everyone sees: the date, each place and its person. |

### Part

| Field | Rules |
|---|---|
| `kind` | Treasures talk, Spiritual gems, Bible reading, Starting a conversation, Following up, Making disciples, Explaining your beliefs, Student talk, Field ministry discussion, Living as Christians part, Congregation Bible Study, Other. Set from the workbook; planners can change it. It decides who can have the part. |
| `source` | Its number in the workbook (for the title and description). |
| `title`, `description` | `Option`: typed by a planner; otherwise the workbook's. |
| `minutes` | 1–120. |
| `edited` | Changed by a planner: refreshing the workbook leaves it as it is. Parts that are not edited follow the workbook (and disappear if the workbook no longer has them). |
| `main`, `assistant`, `aux`, `aux_assistant` | People. An assistant for the student demonstrations and a reader for the Congregation Bible Study. The auxiliary classroom for the Bible reading and the student parts. |

## Who can have a place

| Place | Privilege (M-01) |
|---|---|
| Chairman | Chairman (midweek) |
| Auxiliary classroom counselor | Auxiliary classroom chairman |
| Prayers | Prayer |
| A part | The privilege of its kind (for example *Bible reading*). *Explaining your beliefs* uses *Making disciples*. *Other*: any publisher. |
| Assistant | Assistant, and the **same gender as the student or a member of their family** |
| Reader | Congregation Bible Study reader |

**Choosing:** the list shows everyone who may take the place, in three groups:
1. *Available*.
2. *Already has a part this week*, with the other part named. Possible, but not ideal.
3. *Away that day* (absences).

Within each group, the person who has gone **longest without this kind of part** comes first
(never had it, first of all), then the one who has gone **longest without any part**, then by
name. Each row says both ("This part 6 weeks ago · any part 2 weeks ago").

**Autofill** (the shown week, or the month): every empty place gets the first *available*
candidate who has nothing else that week. Students are filled before their assistants. Places
nobody fits stay empty, and a message says how many.

## Times

The meeting starts at the congregation's midweek time (*Congregation settings*). Every moment
gets its start time, and the week and each section show when they start and end:

| Moment | Minutes |
|---|---|
| Opening song, then the opening prayer | 4 + 1 |
| Opening comments (chairman) | 1 |
| Each part | its minutes |
| Counsel after the Bible reading and each student part (chairman) | 1 |
| Middle song (opens *Living as Christians*) | 4 |
| Concluding comments | 3 |
| Closing song, then the closing prayer | 4 + 1 |

Moving, adding or deleting parts, or changing their minutes, moves the times after them.

## Export

*Export* (everyone with access): from a month, 1 to 12 whole months, as **PDF, Word or CSV**. One
line per moment: date, time, part (number, title, minutes), who has it, and the assistant or
reader; with the auxiliary classroom, two more columns. Readers export what was sent.

## Sending

As in M-15:
- *Send* (the shown week or the month) shows the plan to everyone with read access.
- Only the people whose assignments are new, changed (another part, or the date moved) or
  removed get a notice in *My assignments*, which lists their midweek parts.
- Readers see the last sent version only.

## Screens

*Meetings › Midweek meeting* (`/app/meetings/midweek`). Header (write): *Settings* (auxiliary
classroom), *Workbook*, *Autofill*, *Send (n)*.

- **Week** (`‹ Thursday 26-11-2026 ›`): moves from meeting day to meeting day. It opens on the next
  meeting (this week's until its day has passed); *Today* goes back to it.
  - Alerts for conventions, assemblies, the circuit overseer's visit and the Memorial in that week.
  - The reading and songs, the send state and how many places are filled.
  - *No meeting this week*, and *Auxiliary classroom this week* when it is on.
  - The meeting in its order:
    - *Opening*: chairman (opening comments, 1 min), the opening song, the opening prayer (and the
      counselor of the auxiliary classroom when it is used).
    - The sections, each with a header in the workbook's colour (teal for *Treasures from God's
      Word*, gold for *Apply yourself to the field ministry*, maroon for *Living as Christians*,
      as on wol.jw.org) and its minutes. *Living as Christians* starts with the middle song.
    - *Closing*: concluding comments (3 min), the closing song, the closing prayer.
  - Parts are numbered 1, 2, 3… in meeting order, also the parts a planner added or moved. Each
    part shows its title, minutes, description and its places as buttons, with "Away that day"
    when it applies.
  - The arrows next to the number move a part up or down; at the edge of a section it moves to
    the next section. The arrows in a section's header move the section.
  - The pencil edits a part (kind, title, description, minutes) or deletes it. *Add part*, *Delete
    section*, *Add section*.
- **Month:** one card per meeting with every part and who has it (and the assistant), the send
  state, and *Open the week*.
- **Statistics** (last 3, 6 or 12 months):
  - meetings, places filled, how many had a part, parts per person;
  - assignments per person (people with a privilege and none included);
  - longest without a part;
  - parts by kind.

## Home widget

*Midweek meeting* (for everyone, in *For you*): the next meeting as it was sent, with its day and
times, one line per moment (time, part, who). The user's own name is in colour and a badge says
what they have ("You: Chairman"). *Show all* opens the whole list, also for people who cannot open
the meeting page; with access, *Open the meeting* goes to it. Not sent yet: it says so.

## Permissions

The page *Meetings › Midweek meeting* (M-02): **write** plans and sends; **read** sees the sent
weeks.

## Open questions

1. The circuit overseer's visit: should the app change the week by itself (the meeting on Tuesday,
   his talk instead of the Congregation Bible Study), or is a reminder enough?
2. Printing the schedule (S-140) and the assignment slips for students (S-89): next round?
