# M-01 People: persons and families

A **person** is anyone the congregation keeps data about: publishers, Bible students and
visitors. The UI calls them "users" (*Users › All users*). A person is not an account: an account
is a login, and a membership links at most one account to a person (R-019). Persons can be
grouped in **families**, with one family head who is the main contact.

Status: draft, reviewed on 2026-11-17 (decisions and open questions at the end).

```mermaid
erDiagram
    PERSON ||--o| MEMBERSHIP : "has an account through"
    PERSON }o--o| FAMILY : "belongs to"
    FAMILY ||--|| PERSON : "has as head"
    FAMILY ||--o{ RELATIONSHIP : contains
    RELATIONSHIP }o--|| PERSON : from
    RELATIONSHIP }o--|| PERSON : to
    PERSON }o--o{ FS_GROUP : "member of (M-09)"
```

## Person

Collection `person`, key domain `people`.

| Field | Type | Default | Rules |
|---|---|---|---|
| `first_name` | text | | Required. |
| `last_name` | text | | Required. |
| `birthday` | `Option<date>` | | Not in the future. |
| `gender` | `Male` or `Female` | | Required. Drives eligibility (R-010). |
| `email` | `Option<text>` | | Must look like an e-mail address. |
| `telephone` | `Option<text>` | | Free text, so any national format works. |
| `mobile_phone` | `Option<text>` | | Free text. |
| `baptism_date` | `Option<date>` | | Only for baptised types. After `birthday`. |
| `publisher_type` | see below | `Student` | Required. |
| `infirm` | bool | `false` | |
| `pioneer_number` | `Option<text>` | | Only for regular pioneers, special pioneers and missionaries. |
| `appointment` | `Option<Elder or MinisterialServant>` | none | Only for baptised brothers. |
| `anointed` | bool | `false` | Only for baptised persons. |
| `address_line_1` | `Option<text>` | | |
| `address_line_2` | `Option<text>` | | |
| `postal_code` | `Option<text>` | | |
| `city` | `Option<text>` | | |
| `consent_date` | `Option<date>` | | The date the person agreed that the congregation keeps their data. Not in the future. |
| `notes` | `Option<text>` | | Free text, at most 2,000 characters. |
| `moved_away_date` | `Option<date>` | none | Set when the person moves away (see [Moving away](#moving-away)). Not in the future. |

There is **no password field**. The request listed a nullable `password`, but a password in the
person record would break the zero-knowledge model: anyone who can read people could read it, and
a person would need it to sign in before they can decrypt anything. A person "has a password" when
they have an account. The account is created by an invite (5.6), and the membership links it to
the person. The person screen shows the account state:

| Account state | Meaning | Comes from |
|---|---|---|
| No account | Normal for people without a device | No membership for this person |
| Invited | An invite for this person is open | Open `invite` record (M-02) |
| Linked | The person signs in with their own account | `membership.person` points to this person |

### Publisher types

| Value | Label | Baptised | Publisher | Pioneer number |
|---|---|---|---|---|
| `Student` | Bible student | no | no | no |
| `Visitor` | Visitor | no | no | no |
| `UnbaptisedPublisher` | Unbaptised publisher | no | yes | no |
| `BaptisedPublisher` | Baptised publisher | yes | yes | no |
| `ContinuousAuxiliaryPioneer` | Continuous auxiliary pioneer | yes | yes | no |
| `RegularPioneer` | Regular pioneer | yes | yes | yes |
| `SpecialPioneer` | Special pioneer | yes | yes | yes |
| `Missionary` | Missionary | yes | yes | yes |

"Publisher" means every type except `Student` and `Visitor`. Only publishers can be in a field
service group (M-09) and have an activity status.

### What happens when a value changes

`theo-core` checks the rules on every save. A failed rule is shown on the field and nothing is
saved. When a change makes other fields invalid, the app clears those fields and shows what it
cleared before the user saves:

| Change | Effect |
|---|---|
| Type changes to a type without a pioneer number | `pioneer_number` is cleared (data minimisation, R-020). |
| Type changes to a not-baptised type | `baptism_date`, `appointment` and `anointed` are cleared. |
| Gender changes to `Female` | `appointment` is cleared. Relationships whose type does not fit the new gender are shown as errors. |
| Type changes to `Student` or `Visitor` | The person leaves their field service group, and stops being its overseer or assistant (M-09). |

### Activity status (derived)

The activity status is computed from field service reports (M-10). It is never stored. It looks
at the **last 6 complete months**: in November 2026, that is May to October 2026. Months before
the person's first report do not count, so a new publisher is not irregular.

| Status | Rule |
|---|---|
| Active | A report for every counted month. |
| Irregular | At least one counted month without a report, and at least one with a report. |
| Inactive | No report in any of the 6 months. |
| (none) | Students and visitors, publishers with no report yet, and people who moved away. |

Reports are in the `reports` key domain. A user who does not hold that key cannot compute the
status, so the app hides the status and its filter for them.

## Family

A family is two or more persons with relationships between them, and one head. The head is the
main contact of the family.

Collection `family`, key domain `people`.

| Field | Type | Rules |
|---|---|---|
| `head` | person `record_id` | Required. Must be a member of the family. |
| `members` | list of person `record_id` | At least 2. A person is in **exactly one family or none**, never two. |
| `relationships` | list of relationships, see below | Every member except the head has at least one relationship. |

The family has no name field. The UI shows it as "Family of `<head name>`", so the name follows
the head when the head changes.

### Relationships

A relationship is stored once, in one direction: "`from` is the `kind` of `to`". For example
"John is the Father of Tom".

| Field | Type | Rules |
|---|---|---|
| `from` | person `record_id` | A member of the same family. |
| `to` | person `record_id` | A member of the same family. Not the same as `from`. |
| `kind` | `Father`, `Mother`, `Spouse`, `Son`, `Daughter`, `Brother`, `Sister`, `Other` | The kind fits the gender of `from`: `Father`, `Son` and `Brother` need a brother, `Mother`, `Daughter` and `Sister` need a sister. |

Rules:

- One relationship per pair of persons, in either direction.
- A person has at most one `Spouse`.
- The app shows each relationship from both sides. It derives the other side from the kind and
  the gender of `to`:

| Stored kind (`from` is … of `to`) | Shown on the page of `from` | Shown on the page of `to` |
|---|---|---|
| `Father` or `Mother` | `to` is the Son or Daughter | `from` is the Father or Mother |
| `Son` or `Daughter` | `to` is the Father or Mother | `from` is the Son or Daughter |
| `Brother` or `Sister` | `to` is the Brother or Sister | `from` is the Brother or Sister |
| `Spouse` or `Other` | the same kind | the same kind |

### Joining, leaving and deleting

| Action | Result |
|---|---|
| Add a relationship between two persons who are not in a family | A new family with both. The head is the parent (`Father`, `Mother`), or for `Spouse` the brother. In other cases, it is the person whose page you are on. The user can change it. |
| Add a relationship between a family member and a person without a family | The person joins that family. |
| Add a relationship between members of two different families | Not allowed. Remove one person from their family first. |
| Remove a person from a family | Their relationships in that family are deleted. If only one member is left, the family is deleted. |
| Remove or delete the head | The app asks for a new head first. It proposes the spouse, else the oldest member. |
| Delete a person | As "remove from family", then the person record is deleted (soft delete). |

## Moving away

When a person moves to another congregation, a user with the right permission marks them as
**moved away** with a date. The person then leaves the current users, but the congregation keeps
their data for **1 year**, for example to answer a request from the new congregation. After that,
the app removes them for good.

| Step | What happens |
|---|---|
| Mark as moved away | `moved_away_date` is set. The person leaves their field service group, and stops being its overseer or assistant (M-09). In Cloud mode their membership ends and open invites are cancelled (R-246). |
| Family | If the rest of the family moves too, the user says so in the same step: everyone gets the date and the family stays a family. A person who moves alone leaves the family, as in "Remove a person from a family". |
| While moved away | The person is on *Users › Moved away*, and in *All users* only when the Status filter includes "Moved away". The page shows the date of removal. They can still be edited, moved back or deleted. They have no activity status, and they cannot be assigned. |
| Move back | `moved_away_date` is cleared. The person is current again, without a field service group. |
| After 1 year | The app **purges** the person (R-021): the record, their relationships and their history. Past assignments then show "Former member". The purge runs on the first device with access to the `people` key that opens the congregation after the date, and the activity log records it (without the name). |

The retention period is 1 year for now. It becomes a congregation setting with the other
retention rules of R-021.

## Deleting a person

Deleting is a soft delete (R-013). The person disappears from lists, filters and exports. Past
assignments and reports keep pointing at the record, so history still shows the name. A purge
(R-021) removes the record and its history for real. Before deleting, the app shows what changes:
family head moves, and field service groups that lose their overseer or assistant (M-09).

## The All users screen

The list, the filters and the export are described here because they depend on this model.

**Filters.** All filters work together. Inside one filter, a person matches if they have **any**
of the chosen values. Across filters, a person must match **all** of them. Example: gender Male
and Female, group North, consent "not recorded" shows all persons of North without a consent
date.

| Filter | Values | Based on |
|---|---|---|
| Search | Free text | First and last name, in both orders, ignoring case and accents |
| Status | Current, Moved away | `moved_away_date`. **The list starts with "Current"**, and "Clear filters" goes back to it. |
| Type | The 8 publisher types | `publisher_type` |
| Appointment | Elder, Ministerial servant, None | `appointment` |
| Anointed | Yes, No | `anointed` |
| Field service group | Each group, No group | M-09 membership, also as overseer or assistant |
| Gender | Male, Female | `gender` |
| Personal consent | Recorded, Not recorded | `consent_date` |
| Activity | Active, Irregular, Inactive | Activity status (derived) |

Each filter value shows how many persons it would add with the other filters applied, so users
do not pick combinations with no result.

**Cards, not a table.** Each person is a card with the name in the effective name format (R-062),
the field service group, and labels for the type, the appointment, anointed, infirm and an activity
status that needs attention (irregular or inactive). The list is sorted by the name as shown. The
app shows no number or other identifier: the name is enough.

**Export.** The export uses the filtered list, in the same order. It runs on the device. The file
is **not encrypted**, so the app warns before it saves it.

| Format | Content |
|---|---|
| CSV | All person fields, one row per person, UTF-8. Cells that start with `=`, `+`, `-` or `@` get a leading `'` so spreadsheets do not run them as formulas. |
| DOCX, PDF | A printable list: name, type, appointment, group, mobile phone and e-mail, with the active filters in the header. |

## Decisions (2026-11-17)

| # | Question | Decision |
|---|---|---|
| P1 | Does the user need a visible number? | No. Records have only the internal random `record_id`; the UI shows names. |
| P2 | Can a person be in more than one family? | No. One family or none. |
| P3 | Delete a person who moves away, or keep them? | Keep them for 1 year with a "moved away" state, then purge ([Moving away](#moving-away)). |

## Open questions

| # | Question | Proposal |
|---|---|---|
| P4 | Is `consent_date` enough, or do you also need what the consent covers (for example "may appear on printed schedules")? | The date only, for now. |
| P5 | Should emergency contacts (listed in M-01 in `REQUIREMENTS.md` 1.3) be persons in the family, or separate fields? | A relationship of kind `Other` to a person, so they can be outside the congregation later. |
