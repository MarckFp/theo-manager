# Hosting a Theo Manager server

Cloud congregations live on a stock **SurrealDB** server (REQUIREMENTS.md 5.3). There is no
Theo Manager backend: the server holds only ciphertext, and its rules are in one schema file. This
page is for the person who runs that server (the *owner*).

The app never has the owner's credentials (D-46). The owner sets the server up once; after that,
people only sign up and sign in with ordinary accounts.

## 1. Start SurrealDB

Use SurrealDB 4.x. Keep the root user and its password to yourself.

```sh
surreal start --user root --pass '<a long secret>' rocksdb:/var/lib/theo/data
```

For a throwaway test server, `memory` instead of `rocksdb:...` keeps nothing on disk.

## 2. Apply the schema

SurrealDB 4 does not create the namespace and the database on first use. Create them first: in
the Surreal Cloud console, or as the server's root user:

```sh
curl -X POST https://db.example.org/sql -u root -H 'Accept: application/json' \
    --data-binary 'DEFINE NAMESPACE IF NOT EXISTS theo; USE NS theo; DEFINE DATABASE IF NOT EXISTS theo;'
```

Then apply the schema as a user with the **Owner** role on that database (or the root user). Sign
in for a token first: basic auth (`-u`) works only for root users, not for database users.

```sh
H=https://db.example.org
read -s -p "Password: " PASS; echo
TOKEN=$(curl -sS -X POST "$H/signin" -H 'Accept: application/json' -H 'Content-Type: application/json' \
    -d "{\"ns\":\"theo\",\"db\":\"theo\",\"user\":\"<database user>\",\"pass\":\"$PASS\"}" | jq -r .token)
unset PASS
curl -sS -X POST "$H/sql" -H 'Accept: application/json' -H "Authorization: Bearer $TOKEN" \
    -H 'Surreal-NS: theo' -H 'Surreal-DB: theo' \
    --data-binary @theo-db/surreal/schema.surql | jq '[.[] | select(.status != "OK")]'
```

- It must print `[]`: every statement succeeded. Check the version with
  `--data-binary 'SELECT VALUE version FROM meta:schema;'` (the schema's current version).
- `read -s` keeps the password out of the shell history. For the root user, drop `ns` and `db`
  from the sign-in.
- Do not use `surreal import`: since SurrealDB 4 it is for data dumps and switches events off,
  which the audit log needs. Do not pipe the file into `surreal sql` either: it splits the file at
  line ends and breaks the multi-line blocks.
- The app uses namespace `theo` and database `theo` unless an invite link names others.
- Apply the file again after you update Theo Manager. Every statement uses `OVERWRITE`, so it
  upgrades in place, and existing rows get values for new fields. The app refuses a server whose
  schema version it does not know.

## 3. Let browsers reach it (CORS)

The web app calls the server from its own address (for example `https://app.example.org`).
Browsers allow that only when the server answers with CORS headers for that address.

- `surreal start` allows every origin by default.
- To allow only your app's address, use `--allow-origin https://app.example.org`. Check the exact
  flag with `surreal start --help`, because it may change between versions.
- A reverse proxy in front of SurrealDB must pass the CORS headers through, and must answer
  `OPTIONS` preflight requests (or pass them on). The app sends these request headers:
  `Accept`, `Content-Type`, `Authorization`, `Surreal-NS` and `Surreal-DB`.

When a browser blocks a request, the app says that the server could not be reached and names the
address that must be allowed. The installed apps (desktop and phone) do not need CORS.

## 4. Put a proxy in front (R-225)

A reverse proxy gives TLS and limits. A minimal Caddy example:

```caddy
db.example.org {
    # Sign-in and sign-up are the endpoints to protect against password guessing.
    # Add a rate-limit plugin (for example caddy-ratelimit) for /signin and /signup.
    request_body {
        max_size 16MB
    }
    reverse_proxy 127.0.0.1:8000
}
```

Recent Caddy versions offer the hybrid post-quantum key exchange `X25519MLKEM768` by default.

## 5. Build the apps for it

The apps talk to one Cloud server, chosen when they are built; users never type an address.
Set the build variables (the `wss://…/rpc` form SurrealDB shows works too):

```sh
THEO_CLOUD_URL=https://db.example.org dx build --release --platform web
```

`THEO_CLOUD_NS` and `THEO_CLOUD_DB` change the namespace and database (default `theo`). Release
builds read `THEO_CLOUD_URL` from the repository secret of the same name. A build without it
offers Local congregations only.

## 6. Check it

The live test signs up throwaway accounts, as the app does, and checks the permissions, invites
and audit log. It needs no owner credentials:

```sh
THEO_SURREAL_URL=https://db.example.org cargo test -p theo-db --test surreal
```

The test deletes the congregations it creates. Its accounts stay; they are named `ann-…`, `bob-…`
and `eve-…`.

## What the server stores

| Table | Holds | Who reads it |
|---|---|---|
| `account` | Username, `argon2(auth key)`, wrapped personal keys, encrypted display name | The account. Co-members see the username and public keys. |
| `congregation`, `membership` | Ids, roles, each member's encrypted name of the congregation | Members. Managers see all memberships. |
| `vault_header`, `keyring` | The root key under the congregation password, the wrapped domain keys | Full-access members (owner and `full`) |
| `item` | Encrypted records, collection names, versions | Members |
| `invite`, `grant` | Invite hashes and sealed bundles, sealed domain keys per member | Managers, and the member concerned |
| `audit` | Who changed what and when, with no content (R-227) | Managers |

*Managers* are the owner and the full-access members.
