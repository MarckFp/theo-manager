//! Invites (REQUIREMENTS.md 5.6): a random 256-bit token `T` in the invite link's fragment.
//!
//! ```text
//! T (32 B random, in the link only)
//!  ├──HKDF "theo/v1/invite-proof"──► proof   → sent once, when joining
//!  │                                  id = SHA-256(hex(proof))  → all the server stores (R-253)
//!  └──HKDF "theo/v1/invite-bundle"─► bundle key → seals the congregation's name and, for a member
//!                                                 invite, the domain secrets of its roles
//! bundle = magic "THIB" (4) | format (1) = 1 | nonce (24) | ciphertext ‖ tag
//!   plaintext = u16 name length | name | u8 n | n × (domain (1) | version (4) | secret (32))
//! ```
//!
//! The server learns `proof` only when the invite is used, and then the invite is spent. `proof`
//! does not give the bundle key (HKDF is one-way), so the server never opens the bundle. The
//! bundle is bound to the congregation, so it cannot be replayed into another one.

use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

use crate::keyring::{DomainId, DomainSecret};
use crate::util::{self, Reader, Secret32};
use crate::Error;

const MAGIC: &[u8; 4] = b"THIB";
const FORMAT: u8 = 1;

/// The secret of an invite link. Wiped on drop.
pub struct InviteToken(Secret32);

impl std::fmt::Debug for InviteToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("InviteToken(..)")
    }
}

/// What an invite bundle holds: the congregation's name, and the domain secrets it gives.
#[derive(Debug)]
pub struct Bundle {
    /// The congregation's name (the new member stores it encrypted for their own account).
    pub name: String,
    /// (domain, key version, secret). Empty for a full-access invite: those members unlock the
    /// root key with the congregation password instead.
    pub secrets: Vec<(DomainId, u32, DomainSecret)>,
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn aad(congregation_id: &str) -> Vec<u8> {
    let mut a = MAGIC.to_vec();
    a.push(FORMAT);
    a.extend_from_slice(b"theo/v1/invite-bundle");
    util::put(&mut a, congregation_id.as_bytes());
    a
}

impl InviteToken {
    /// A fresh random token.
    pub fn generate() -> Result<Self, Error> {
        util::random_secret().map(InviteToken)
    }

    /// The token as text for the link (64 hex digits).
    pub fn to_text(&self) -> String {
        hex(self.0.as_ref())
    }

    /// Reads [`InviteToken::to_text`].
    pub fn from_text(text: &str) -> Result<Self, Error> {
        let text = text.trim();
        if text.len() != 64 || !text.is_ascii() {
            return Err(Error::InvalidKey);
        }
        let mut out = Zeroizing::new([0u8; 32]);
        for (i, b) in out.iter_mut().enumerate() {
            *b = u8::from_str_radix(&text[2 * i..2 * i + 2], 16).map_err(|_| Error::InvalidKey)?;
        }
        Ok(InviteToken(out))
    }

    /// What the joining app sends to the server to use the invite (64 hex digits).
    pub fn proof(&self) -> String {
        hex(util::hkdf32(self.0.as_ref(), "theo/v1/invite-proof").as_ref())
    }

    /// The invite's id on the server: SHA-256 of the proof's text, in hex. The server computes the
    /// same with `crypto::sha256($proof)`, so it can find the invite without ever storing `T`.
    pub fn id(&self) -> String {
        hex(&Sha256::digest(self.proof().as_bytes()))
    }

    /// Seals the bundle for `congregation_id`.
    pub fn seal_bundle(&self, congregation_id: &str, bundle: &Bundle) -> Result<Vec<u8>, Error> {
        let name = bundle.name.as_bytes();
        if name.len() > u16::MAX as usize || bundle.secrets.len() > u8::MAX as usize {
            return Err(Error::Damaged);
        }
        let mut plain = Zeroizing::new(Vec::with_capacity(3 + name.len() + 37 * bundle.secrets.len()));
        plain.extend_from_slice(&(name.len() as u16).to_le_bytes());
        plain.extend_from_slice(name);
        plain.push(bundle.secrets.len() as u8);
        for (domain, version, secret) in &bundle.secrets {
            plain.push(domain.0);
            plain.extend_from_slice(&version.to_le_bytes());
            plain.extend_from_slice(secret.0.as_ref());
        }
        let key = util::hkdf32(self.0.as_ref(), "theo/v1/invite-bundle");
        let mut out = MAGIC.to_vec();
        out.push(FORMAT);
        out.extend_from_slice(&util::seal(&key, &aad(congregation_id), &plain)?);
        Ok(out)
    }

    /// Opens what [`InviteToken::seal_bundle`] made. A wrong token or congregation fails.
    pub fn open_bundle(&self, congregation_id: &str, sealed: &[u8]) -> Result<Bundle, Error> {
        let mut r = Reader::new(sealed);
        if r.take(4)? != MAGIC {
            return Err(Error::NotAnEnvelope);
        }
        if r.u8()? != FORMAT {
            return Err(Error::UnknownFormat);
        }
        let key = util::hkdf32(self.0.as_ref(), "theo/v1/invite-bundle");
        let plain = util::open(&key, &aad(congregation_id), r.rest()).map_err(|_| Error::WrongPasswordOrDamaged)?;
        let mut r = Reader::new(&plain);
        let len = r.take(2)?;
        let len = u16::from_le_bytes([len[0], len[1]]) as usize;
        let name = String::from_utf8(r.take(len)?.to_vec()).map_err(|_| Error::Damaged)?;
        let n = r.u8()?;
        let mut secrets = Vec::with_capacity(n as usize);
        for _ in 0..n {
            let domain = DomainId(r.u8()?);
            let version = r.u32()?;
            secrets.push((domain, version, DomainSecret(util::secret_from(r.take(32)?)?)));
        }
        if !r.rest().is_empty() {
            return Err(Error::Damaged);
        }
        Ok(Bundle { name, secrets })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bundle_opens_only_with_its_token_and_congregation() -> Result<(), Error> {
        let t = InviteToken::generate()?;
        let secret = DomainSecret::generate()?;
        let public = secret.keys().public_key();
        let bundle = Bundle { name: "Riverside".into(), secrets: vec![(DomainId::SCHEDULE, 2, secret)] };
        let sealed = t.seal_bundle("cong-1", &bundle)?;

        let back = InviteToken::from_text(&t.to_text())?.open_bundle("cong-1", &sealed)?;
        assert_eq!(back.name, "Riverside");
        assert_eq!(back.secrets.len(), 1);
        assert_eq!((back.secrets[0].0, back.secrets[0].1), (DomainId::SCHEDULE, 2));
        assert_eq!(back.secrets[0].2.keys().public_key(), public);

        assert!(InviteToken::generate()?.open_bundle("cong-1", &sealed).is_err());
        assert!(t.open_bundle("cong-2", &sealed).is_err());
        // The id is SHA-256 of the proof's text; neither shows the token.
        assert_eq!(t.id(), hex(&Sha256::digest(t.proof().as_bytes())));
        assert_eq!((t.id().len(), t.proof().len()), (64, 64));
        assert!(t.id() != t.to_text() && t.proof() != t.to_text());
        assert!(InviteToken::from_text("zz").is_err());
        Ok(())
    }
}
