# M-09 Field service groups

A field service group is a set of publishers with a group overseer and a group assistant. It
organises the ministry. Leading a group gives **no extra access** in the app: a user sees or
changes group data only through a permission of their role (`REQUIREMENTS.md` 5.5).

Status: reviewed on 2026-11-17. All questions are decided (see the end).

## Group

Collection `fs_group`, key domain `people`.

| Field | Type | Rules |
|---|---|---|
| `name` | text | Required. Unique in the congregation. |
| `overseer` | `Option<person record_id>` | A current baptised brother (M-01). |
| `assistant` | `Option<person record_id>` | A current baptised brother. Not the same person as `overseer`. |
| `members` | list of person `record_id` | Current publishers only. |

A group has no meeting place: the congregation does not need one (decided 2026-11-17).

The overseer and the assistant are **always members** of their group. The app does not store them
in `members` twice: the member list of a group is `members` plus `overseer` plus `assistant`.

## Rules

- Only publishers can be in a group (every type except Bible student and visitor, M-01). Bible
  students are never in a group.
- The overseer and the assistant MUST be **baptised brothers**. The app recommends them in this
  order: elders, ministerial servants, then other baptised brothers. It shows a **warning**, not
  an error, when the chosen brother is not an elder or a ministerial servant.
- A person is a plain member of **at most one** group. This group is their **home group**.
- To add someone to a group, they must have no group. To move someone, remove them from their
  group first (or change the group on their person page).
- An overseer or assistant MAY lead a second group as well. This is rare, and the group page warns
  when it happens.
- A group without an overseer or assistant is allowed, so admins can set up groups step by step.
  The groups page marks it.

## What members see

A member with an account sees the **names** of the people in their own group, and nothing else
about them. Being the overseer or assistant does not change this. Users whose role has the right
permission (for example the secretary or the service overseer) see all data
(`REQUIREMENTS.md` 5.5).

| Who | Sees |
|---|---|
| A member of the group, also its overseer or assistant | The group name, the overseer and assistant, and the display names of the members. No contact data, no type, no reports. |
| A user whose role has `people.read` | Everything in M-01 and M-09, for the scope of the permission (`all` or `own_group`). |

To make this hold also against the server (key domains, R-247), each group has a small **roster**
record with only these names. It is encrypted with a key of its own for each group, and only the
members of that group get a grant for it. The full person records stay in the `people` domain,
which members do not hold. When someone leaves the group (moves to another group, moves away or is
deleted), the app writes the roster again under a new group key, so the person who left cannot
read new versions.

The roster is derived: the app rewrites it when the group or a member's name changes. It is never
edited by hand.

## Home group and permissions

Records that belong to one person, like field service reports (M-10), carry the home group as
their `scope_id` (R-242). The server uses it for permissions with the scope `own_group`, which an
admin MAY give to a role (for example a role for some group overseers). For a person who leads one
group and is a plain member of another, the home group is the group where they are a plain member.
For a person who only leads groups, it is the first group they lead.

When a person moves to another group, the app updates the `scope_id` of their records.

## What happens when something changes

| Change | Result |
|---|---|
| A person becomes a Bible student or a visitor | They leave their group, and stop being its overseer or assistant. |
| An overseer or assistant stops being a baptised brother (type or gender changes) | They stop leading the group and stay in it as a plain member. The person page says so before saving. |
| A person moves away or is deleted | They leave their group and stop leading it. The roster gets a new key. |
| The overseer or assistant is changed | The old one stays in the group as a plain member, unless they already have another home group. |
| A group is deleted | Its members have no group. The app asks to confirm and shows how many persons this affects. |

## The Field service groups screen

*Users › Field service groups* uses the same patterns as *All users* (`REQUIREMENTS.md` R-436).

| Part | What it shows |
|---|---|
| Alert | How many current publishers are in no group, with a button that opens *All users* filtered on "No group". |
| Search | Group name or the name of any member. |
| Cards | Group name, overseer and assistant, the number of members, and warnings: no overseer, no assistant, how many members are irregular or inactive. Sorted by name. |
| Export | **All groups only**, never a single group and never only the search result: PDF, DOCX or CSV with one row per person (group, name, role, mobile phone, e-mail). Same menu and warning as *All users* (R-066). |
| Group page | Form: name, overseer, assistant, saved with **Save**. The overseer and assistant lists have sections: *Elders*, *Ministerial servants*, *Other baptised brothers*. Warnings from [Rules](#rules) under the form. Side cards: members (open a person, remove a member, add members) and the activity of the group. Member changes apply at once, like family changes on the person page. |
| Add members | A dialog with search and checkboxes. It lists **only current publishers without a group**. |
| Delete | Asks first. Members keep their data and have no group afterwards, unless they lead another group. |

## Decisions (2026-11-17)

| # | Question | Decision |
|---|---|---|
| G1 | What can members see of their own group? | The names only (see [What members see](#what-members-see)). Everything else needs a permission. |
| G2 | Can Bible students be in a group? | No. Publishers only. |
| G3 | Do groups need a meeting place? | No. |
| G4 | Who can be overseer or assistant? | Baptised brothers only. Recommended order: elders, ministerial servants, other baptised brothers. |
| G5 | What can be exported? | All groups together only. No export of a single group. |
| G6 | Do group overseers get access to their group's data? | Not by being overseer. Only users whose role has the permission. |
| G7 | Can the member picker add people from other groups? | No. It lists only publishers without a group; to move someone, remove them first. |
| G8 | Can the overseer or assistant lists offer brothers who are in another group? | Yes, with a warning on the group page, so the rare case of leading two groups stays possible. |
