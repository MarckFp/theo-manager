//! Record envelopes (6.4): how every stored record is encrypted.
//!
//! ```text
//! envelope = magic "TH" (2) | format (1) = 0x01 | suite (1) | domain_id (1) | key_version (4, LE)
//!            | [suite 0x02 only: recipients] | nonce (24) | ciphertext ‖ tag (16)
//! recipients = count (1, ≥ 1) | count × ( key_id (8) | X-Wing encapsulation (1120) | wrapped content key (48) )
//! ```
//!
//! - **Suite 0x01** (domain key): XChaCha20-Poly1305 with the domain's record key.
//! - **Suite 0x02** (sealed, R-327): a random content key, X-Wing-encapsulated to each recipient
//!   (the domain's public key and the author's own key). Someone who cannot read the domain can
//!   still write into it; afterwards only domain readers and the author can open it.
//!
//! The AAD (R-304) binds the ciphertext to where it is stored:
//! `"theo/v1/record" ‖ congregation_id ‖ collection ‖ scope_id ‖ record_id ‖ version ‖ header`.
//! Strings are length-prefixed and the header holds suite, domain and key version (and the
//! recipients), so the server cannot move a record between rows, collections, scopes, versions or
//! domains without the tag failing.
//!
//! The plaintext is the caller's bytes (CBOR, R-306) padded with Padmé (R-305):
//! `length (4, LE) | bytes | zeros`. Unknown formats and suites fail closed (R-307).

use zeroize::Zeroizing;

use crate::kem::{self, KemPublicKey, KemSecretKey, KEY_ID_LEN};
use crate::keyring::{DomainId, Keyring};
use crate::util::{self, Reader};
use crate::Error;

const MAGIC: &[u8; 2] = b"TH";
/// The only envelope format so far.
pub const FORMAT: u8 = 0x01;
/// Suite 0x01: encrypted with the domain's record key.
pub const SUITE_DOMAIN: u8 = 0x01;
/// Suite 0x02: sealed to recipients' X-Wing keys.
pub const SUITE_SEALED: u8 = 0x02;
const PREFIX_LEN: usize = 2 + 1 + 1 + 1 + 4;
const RECIPIENT_LEN: usize = KEY_ID_LEN + kem::WRAPPED_LEN;
const SEAL_LABEL: &str = "theo/v1/seal";

/// Where a record lives. Every field is bound into the AAD.
#[derive(Clone, Copy, Debug)]
pub struct RecordContext<'a> {
    /// The congregation.
    pub congregation_id: &'a str,
    /// The table or collection, for example `"midweek_week"`.
    pub collection: &'a str,
    /// The scope inside the collection (for example a person's id), or `""`.
    pub scope_id: &'a str,
    /// The record id.
    pub record_id: &'a str,
    /// The record version (it increases on every write).
    pub version: u64,
}

/// What an envelope says about itself, readable without any key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Header {
    /// [`SUITE_DOMAIN`] or [`SUITE_SEALED`].
    pub suite: u8,
    /// The key domain.
    pub domain: DomainId,
    /// The key version of the domain.
    pub key_version: u32,
}

fn prefix(suite: u8, domain: DomainId, key_version: u32) -> Vec<u8> {
    let mut h = MAGIC.to_vec();
    h.push(FORMAT);
    h.push(suite);
    h.push(domain.0);
    h.extend_from_slice(&key_version.to_le_bytes());
    h
}

fn aad(ctx: &RecordContext<'_>, header: &[u8]) -> Vec<u8> {
    let mut a = b"theo/v1/record".to_vec();
    util::put(&mut a, ctx.congregation_id.as_bytes());
    util::put(&mut a, ctx.collection.as_bytes());
    util::put(&mut a, ctx.scope_id.as_bytes());
    util::put(&mut a, ctx.record_id.as_bytes());
    a.extend_from_slice(&ctx.version.to_le_bytes());
    a.extend_from_slice(header);
    a
}

/// Padmé (Nikitin et al., 2019): rounds a length up so that at most about 12 % is added, and
/// leaks only O(log log n) bits about the real size.
pub fn padme(len: usize) -> usize {
    if len < 2 {
        return len;
    }
    let e = usize::BITS - 1 - len.leading_zeros(); // floor(log2 len)
    let s = u32::BITS - e.leading_zeros(); // floor(log2 e) + 1
    let last_bits = e - s;
    let mask = (1usize << last_bits) - 1;
    (len + mask) & !mask
}

fn pad(data: &[u8]) -> Result<Zeroizing<Vec<u8>>, Error> {
    let len = u32::try_from(data.len()).map_err(|_| Error::InvalidKey)?;
    let mut out = Zeroizing::new(Vec::with_capacity(padme(data.len() + 4)));
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(data);
    out.resize(padme(data.len() + 4), 0);
    Ok(out)
}

fn unpad(padded: &[u8]) -> Result<Zeroizing<Vec<u8>>, Error> {
    let mut r = Reader::new(padded);
    let len = r.u32().map_err(|_| Error::Damaged)? as usize;
    let rest = r.rest();
    if len > rest.len() || rest[len..].iter().any(|b| *b != 0) {
        return Err(Error::Damaged);
    }
    Ok(Zeroizing::new(rest[..len].to_vec()))
}

/// Reads the header of an envelope. Unknown formats and suites fail closed (R-307).
pub fn header(envelope: &[u8]) -> Result<Header, Error> {
    let mut r = Reader::new(envelope);
    if r.take(2)? != MAGIC {
        return Err(Error::NotAnEnvelope);
    }
    if r.u8()? != FORMAT {
        return Err(Error::UnknownFormat);
    }
    let suite = r.u8()?;
    if suite != SUITE_DOMAIN && suite != SUITE_SEALED {
        return Err(Error::UnknownSuite);
    }
    let domain = DomainId(r.u8()?);
    let key_version = r.u32()?;
    Ok(Header { suite, domain, key_version })
}

/// Encrypts a record with the newest version of its domain (suite 0x01).
pub fn seal(keyring: &Keyring, domain: DomainId, ctx: &RecordContext<'_>, plaintext: &[u8]) -> Result<Vec<u8>, Error> {
    let (key_version, secret) = keyring.current(domain).ok_or(Error::NoKey)?;
    let keys = secret.keys();
    let mut out = prefix(SUITE_DOMAIN, domain, key_version);
    let body = util::seal(&keys.record_key, &aad(ctx, &out), &pad(plaintext)?)?;
    out.extend_from_slice(&body);
    Ok(out)
}

/// Seals a record to recipients (suite 0x02): usually the domain's public key (from
/// [`crate::keyring::DomainKeys::public_key`]) and the author's own key. `key_version` is the
/// domain version whose public key is used.
pub fn seal_to(recipients: &[&KemPublicKey], domain: DomainId, key_version: u32, ctx: &RecordContext<'_>, plaintext: &[u8]) -> Result<Vec<u8>, Error> {
    let count = u8::try_from(recipients.len()).map_err(|_| Error::InvalidKey)?;
    if count == 0 {
        return Err(Error::NoKey);
    }
    let content_key = util::random_secret()?;
    let mut out = prefix(SUITE_SEALED, domain, key_version);
    let wrap_aad = out.clone();
    out.push(count);
    for to in recipients {
        let id = to.key_id();
        let mut entry_aad = wrap_aad.clone();
        entry_aad.extend_from_slice(&id);
        out.extend_from_slice(&id);
        out.extend_from_slice(&kem::wrap_to(to, SEAL_LABEL, &entry_aad, &content_key)?);
    }
    let body = util::seal(&content_key, &aad(ctx, &out), &pad(plaintext)?)?;
    out.extend_from_slice(&body);
    Ok(out)
}

/// Opens any envelope with the keyring: suite 0x01 with the record key, suite 0x02 with the
/// domain's X-Wing key.
pub fn open(keyring: &Keyring, ctx: &RecordContext<'_>, envelope: &[u8]) -> Result<Zeroizing<Vec<u8>>, Error> {
    let h = header(envelope)?;
    let keys = keyring.get(h.domain, h.key_version).ok_or(Error::NoKey)?.keys();
    match h.suite {
        SUITE_DOMAIN => {
            let (head, body) = envelope.split_at(PREFIX_LEN);
            unpad(&util::open(&keys.record_key, &aad(ctx, head), body)?)
        }
        _ => open_sealed(&keys.kem, ctx, envelope),
    }
}

/// Opens a sealed envelope (suite 0x02) as one of its recipients, for example the author with
/// their personal key.
pub fn open_sealed(key: &KemSecretKey, ctx: &RecordContext<'_>, envelope: &[u8]) -> Result<Zeroizing<Vec<u8>>, Error> {
    if header(envelope)?.suite != SUITE_SEALED {
        return Err(Error::UnknownSuite);
    }
    let my_id = key.public_key().key_id();
    let wrap_aad = &envelope[..PREFIX_LEN];
    let mut r = Reader::new(&envelope[PREFIX_LEN..]);
    let count = r.u8()?;
    if count == 0 {
        return Err(Error::NotAnEnvelope);
    }
    let mut content_key = None;
    for _ in 0..count {
        let entry = r.take(RECIPIENT_LEN)?;
        let (id, wrapped) = entry.split_at(KEY_ID_LEN);
        if content_key.is_none() && id == my_id {
            let mut entry_aad = wrap_aad.to_vec();
            entry_aad.extend_from_slice(id);
            content_key = Some(kem::unwrap_with(key, SEAL_LABEL, &entry_aad, wrapped)?);
        }
    }
    let body = r.rest();
    let head = &envelope[..envelope.len() - body.len()];
    let content_key = content_key.ok_or(Error::NoKey)?;
    unpad(&util::open(&content_key, &aad(ctx, head), body)?)
}
