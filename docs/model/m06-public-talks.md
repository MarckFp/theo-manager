# M-06 Public talks

Public talks by **number only**:
- other congregations and their public talk coordinators;
- visiting speakers;
- the talks each local speaker gives;
- **outgoing** talks (local speakers in other congregations).

The **incoming** schedule is the public talk of each weekend meeting (M-04).

Status: first version, 2026-11-25 (model and rules in `theo-core/src/talks.rs`; no screens yet).
Key domain: **schedule**.

## Talks are numbers (decided 2026-11-25)

The app stores talk **numbers** only (1 to 999). Titles are never stored or committed, because
the outlines are not ours to publish. Lists can be typed as `1, 5, 12-14`, and are shown with
runs joined (`1-3, 7`).

## Records

### Settings (`talk_settings`)

| Field | Default | Rules |
|---|---|---|
| `repeat_months` | 6 | A talk given here less than this many months before or after gets a warning. |
| `max_outgoing_per_month` | 1 | More outgoing talks for one speaker in one month get a warning. |
| `not_in_use` | none | Talk numbers not to schedule (warning). |

### Other congregation (`other_congregations`)

| Field | Rules |
|---|---|
| `name` | Required, unique. |
| `number`, `address`, `meeting_time` | `Option`. `meeting_time` is text ("Sunday 10:00"). |
| `coordinator_name`, `coordinator_phone`, `coordinator_email` | `Option`. The e-mail must contain `@`. |
| `notes` | `Option`. |

It cannot be deleted while visiting speakers or outgoing talks use it.

### Visiting speaker (`visiting_speakers`)

| Field | Rules |
|---|---|
| `first_name`, `last_name` | Required. |
| `congregation` | Required: one of the other congregations. |
| `appointment` | `Option`: elder or ministerial servant. |
| `phone`, `email`, `notes` | `Option`. |
| `talks` | The talk numbers they give. |

Deleting a speaker removes them from weekends that are **not sent**. Sent weekends keep what was
sent.

### Local speaker talks (`speaker_talks`)

Person → talk numbers. Only brothers with the *Public talk* privilege who did not move away.

### Outgoing talk (`outgoing_talks`)

| Field | Rules |
|---|---|
| `date` | Required. |
| `time` | `Option`, `HH:MM`. |
| `speaker` | Required: a person with the *Public talk in other congregations* privilege. |
| `congregation` | Required: one of the other congregations. |
| `talk` | `Option`: the number. |
| `notes` | `Option`. |
| `sent` | What the speaker sees. |

**Warnings (not refused):**
- the speaker has an absence that day;
- the speaker has a weekend place here that week;
- the speaker has more outgoing talks that month than allowed;
- the talk is not in the speaker's list.

**Send:** the speaker gets a notice and sees the talk in *My assignments*. Deleting a sent talk
sends them a *cancelled* notice.

## Open questions

1. Where does *Other congregations* live in the menu (D-26)? Proposal: a tab of *Public talks*.
2. Should the coordinator's details be shared between congregations that both use Theo Manager?
