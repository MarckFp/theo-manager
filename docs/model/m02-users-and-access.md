# M-02 Users and access

A **person** (M-01) is anyone the congregation keeps data about. An **account** is a login. They
are different things: most persons never use the app. **Permissions** say what an account may see
and change in the app. They are not privileges: privileges (M-01) say what a person may take part
in at the meetings.

Status: draft for review (2026-11-18).

## Who can have permissions

| Case | Person | Account | Permissions |
|---|---|---|---|
| Managed by others: no e-mail, no app | ✅ | – | – |
| Contact details only: has an e-mail, did not join | ✅ (the e-mail is only contact data) | – | – |
| Invited: link sent, not used yet | ✅ | open invite | – (after they join) |
| Joined with the invite link | ✅ | ✅ linked to the person | ✅ |

**What decides it is the link, not a field on the person.** When someone accepts an invite (5.6),
the server creates a **membership** that links their account to their person (R-019). That link is
the proof that they joined, so the person record needs no extra field:

- The **e-mail** never decides anything: anyone can type an e-mail into a person.
- A **password** cannot be stored on the person: in an end-to-end encrypted app the account password
  never leaves the user's device; the server keeps only a hash of a key derived from it (R-222).

Permissions are stored **on the membership**. When the membership ends (the person moves away, is
deleted, or an administrator removes the account), the permissions end with it, and the app
rotates the keys that account held (R-246).

**Local congregations** have one user, on one device, with every permission (R-244). The
permissions screen explains this; permissions apply once the congregation moves to Cloud.

## You are a person too

The account that creates a congregation is also a **person** of it: the wizard asks for your
first name, last name and gender ("About you"), adds you to *All users* as a baptised publisher
(change it later), links your account to that person and makes you the owner (administrator). So
your account and your person are the same in every list, and *My assignments* knows who you are.

- In *All users* your card has the tag **You**; your person page says "This is you".
- **Deleting yourself** removes your access to the congregation: it disappears from your list of
  congregations, and an administrator can invite you again. It is **not possible** on a Local
  congregation (you are its only user) or when you are the only administrator (make someone else
  administrator first).

## Permissions

| Permission | What it allows |
|---|---|
| **Read** a page | See that sub-menu, for example *Audio, video and stage*, without changing anything. One per sub-menu. |
| **Write** a page | Read and change everything on that sub-menu, for example manage all audio schedules. Write includes read. |
| Write the reports of their own group | Enter and change the field service reports of the members of their own group. |
| Write all field service reports | = **Write** on *Ministry › Field service reports*. |
| Read their own group | Contact details, field service reports and family of the members of their own group. |
| Read all groups | The same information for the members of every group. |
| Write user privileges | = **Write** on *Users › Privileges*. |
| Write user data | = **Write** on *Users › All users*. |
| **Administrator** | Every permission, now and for pages added later. Only administrators change permissions (*Users › Permissions* has no read or write level of its own). |

The person who **created the congregation** is always an administrator; nobody can take it away.

Without any permission, an account still sees its own data: its assignments, its own reports and
absences (R-017), and the names of its own group (M-09, "What members see").

### Membership record

Table `membership` (Cloud, R-221), readable only by its member and by administrators.

| Field | Type | Rules |
|---|---|---|
| `account` | account id | Set by the server when the invite is accepted. |
| `person` | person `record_id` | One account per person, one person per account (R-019). |
| `admin` | bool | Always true for the creator. |
| `pages` | map of page → `read` or `write` | Pages are the sub-menus (`"congregation/audio-video"`). Missing = no access. |
| `own_group_reports`, `own_group_read`, `all_groups_read` | bool | The group scope uses the member's home group (M-09). |

**As built (schema v3, 2026-11-26).** The permissions themselves stay in the encrypted data
(`Directory.access`). The server's `membership` row holds only what it must enforce: `role`
(`owner`, `full` or `member`) and `writes`, the collections the member may write, computed from the
permissions (`theo_core::invites::writes_for`). Administrators are `owner` or `full` and write
everything.

**Enforcement.** The app hides what an account may not use (R-245). In Cloud mode the server
checks the same permissions on every read and write (R-241), and each permission maps to the key
domains it needs (6.2): an account gets only the keys of what it may read (R-247).

## The Permissions screen

*Users › Permissions* lists the accounts (people who joined), with a search. It also says how many
people are invited or have no account. A card opens the account's permissions: Administrator, the
three group permissions, then one **No access / Read / Write** choice per sub-menu, grouped like
the menu. The person page's *Account* card shows a summary and a link to it.

## Presets

A preset is a ready set of permissions for a common responsibility. On the account page, choosing
a preset fills in the permissions; the administrator reviews them and saves. The account keeps its
own copy: changing a preset later does not change accounts.

| Preset | Permissions |
|---|---|
| Secretary | Write: all users, families, privileges, moved away, field service reports and groups. Read: events, congregation settings. Read all groups. |
| Service overseer | Write: everything in *Ministry*. Read: all users. Read all groups. |
| Group overseer | Read their own group, write the reports of their own group. Read: meetings for field service, midweek and weekend meetings. |
| Life and Ministry overseer | Write: midweek meeting. Read: privileges, all users. |
| Public talk coordinator | Write: public talks, outgoing talks, weekend meeting. Read: privileges. |
| Duties coordinator | Write: audio, video and stage, attendants, microphones, cleaning, maintenance. Read: events, privileges. |
| Publisher | No extra permissions. |

## Activity log

Every change of permissions is recorded: when (UTC), who made it, and what changed, for example
*"Permissions of Paul Walker: Midweek meeting: read → write; read all groups: on."* The log is part
of the encrypted congregation data and is shown in the congregation *Settings* (cog in the congregation switcher). Saving without changes
records nothing. Other kinds of changes will be added to the same log.

## Decisions (2026-11-18)

| # | Question | Decision |
|---|---|---|
| A1 | Saved sets of permissions? | Yes: built-in presets (above). Custom presets can follow. |
| A2 | Record who changed permissions? | Yes, in the activity log. |
