# theo-crypto

The cryptography of Theo Manager (`REQUIREMENTS.md` section 6). It has no other `theo-*`
dependency and no UI.

## What is in it

| Function | Use |
|---|---|
| `seal_with_password(password, data, KdfParams::DEFAULT)` | Encrypts data with a key derived from a password: Argon2id (64 MiB, 3 passes, 4 lanes, R-301), then XChaCha20-Poly1305. Used for congregation backups. |
| `open_with_password(password, bytes)` | Opens it. Refuses a wrong password, a changed file, weak parameters and files from newer versions. |

The envelope starts with `THBK`, a version byte and the Argon2 parameters, salt and nonce. That
header is authenticated together with the data. The key is wiped from memory after use.

Planned (phase 1): the key hierarchy and key domains (6.2), the record envelope (6.4) and the
post-quantum parts (X-Wing, ML-DSA-65).

## Checks

```sh
cargo test -p theo-crypto
```

In debug builds Argon2 is compiled with optimisations (see the workspace `Cargo.toml`), or it
would take minutes.
