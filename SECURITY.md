# Security policy

Theo Manager handles sensitive personal data, so we take security reports seriously.

## Report a problem

Please **do not** open a public issue. Use GitHub's private vulnerability reporting instead:
**Security → Report a vulnerability** on this repository. Include the steps to reproduce, what an
attacker could do, and the version or commit.

We answer within 7 days, keep you informed while we fix it, and credit you in the release notes
if you want.

## Scope

| In scope | Out of scope |
|---|---|
| The app (all platforms), the encryption and key handling, the SurrealDB schema and permissions, the release workflow | Problems in a self-hosted server that is not configured as documented; social engineering; denial of service against someone's own server |

The threat model is in [`REQUIREMENTS.md`](REQUIREMENTS.md), section 7.

## Supported versions

The project is in early development. Only the latest release gets fixes. Until encryption and
storage are released, do not store real personal data in the app.
