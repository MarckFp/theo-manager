//! Grants (R-321): a domain secret given to one member, X-Wing-encapsulated to their personal
//! public key. Only that member can open it.
//!
//! ```text
//! grant = magic "THGR" (4) | format (1) = 1 | X-Wing encapsulation (1120) | wrapped secret (48)
//! ```
//!
//! The wrapped secret is bound to the congregation, the member, the domain and the key version,
//! so the server cannot hand a grant to someone else or relabel it as another domain.

use crate::kem::{self, KemPublicKey, KemSecretKey};
use crate::keyring::{DomainId, DomainSecret};
use crate::vault::RootKey;
use crate::util::{self, Reader};
use crate::Error;

const MAGIC: &[u8; 4] = b"THGR";
const FORMAT: u8 = 1;
const LABEL: &str = "theo/v1/grant";

fn aad(congregation_id: &str, member_id: &str, domain: DomainId, version: u32) -> Vec<u8> {
    let mut a = LABEL.as_bytes().to_vec();
    util::put(&mut a, congregation_id.as_bytes());
    util::put(&mut a, member_id.as_bytes());
    a.push(domain.0);
    a.extend_from_slice(&version.to_le_bytes());
    a
}

/// Gives `secret` (domain `domain`, version `version`) to the member whose public key is `to`.
pub fn grant(to: &KemPublicKey, congregation_id: &str, member_id: &str, domain: DomainId, version: u32, secret: &DomainSecret) -> Result<Vec<u8>, Error> {
    let mut out = MAGIC.to_vec();
    out.push(FORMAT);
    out.extend_from_slice(&kem::wrap_to(to, LABEL, &aad(congregation_id, member_id, domain, version), &secret.0)?);
    Ok(out)
}

/// Opens a grant with the member's personal X-Wing key.
pub fn open_grant(key: &KemSecretKey, congregation_id: &str, member_id: &str, domain: DomainId, version: u32, grant: &[u8]) -> Result<DomainSecret, Error> {
    let mut r = Reader::new(grant);
    if r.take(4)? != MAGIC {
        return Err(Error::NotAnEnvelope);
    }
    if r.u8()? != FORMAT {
        return Err(Error::UnknownFormat);
    }
    kem::unwrap_with(key, LABEL, &aad(congregation_id, member_id, domain, version), r.rest()).map(DomainSecret)
}

/// The domain number of a root-key grant on the server (real domains start at 1).
pub const ROOT_DOMAIN: DomainId = DomainId(0);
const ROOT_LABEL: &str = "theo/v1/root-grant";

/// Gives the congregation's **root key** to a full-access member (the owner, or a member invited
/// with full access): with it they open the congregation with their account password only, and
/// can manage keys (6.2). Stored as domain [`ROOT_DOMAIN`], version 0. A different label from
/// domain grants, so neither can be passed off as the other.
pub fn grant_root(to: &KemPublicKey, congregation_id: &str, member_id: &str, root: &RootKey) -> Result<Vec<u8>, Error> {
    let mut out = MAGIC.to_vec();
    out.push(FORMAT);
    out.extend_from_slice(&kem::wrap_to(to, ROOT_LABEL, &aad(congregation_id, member_id, ROOT_DOMAIN, 0), &root.0)?);
    Ok(out)
}

/// Opens a root-key grant with the member's personal X-Wing key.
pub fn open_root_grant(key: &KemSecretKey, congregation_id: &str, member_id: &str, grant: &[u8]) -> Result<RootKey, Error> {
    let mut r = Reader::new(grant);
    if r.take(4)? != MAGIC {
        return Err(Error::NotAnEnvelope);
    }
    if r.u8()? != FORMAT {
        return Err(Error::UnknownFormat);
    }
    kem::unwrap_with(key, ROOT_LABEL, &aad(congregation_id, member_id, ROOT_DOMAIN, 0), r.rest()).map(RootKey)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{KdfParams, PersonalKeys, Vault};

    #[test]
    fn a_root_grant_opens_only_for_its_member_and_congregation() -> Result<(), Error> {
        let (_, root) = Vault::create("cong-1", "pw", KdfParams::MINIMUM)?;
        let ann = PersonalKeys::generate()?;
        let g = grant_root(&ann.public().kem, "cong-1", "ann", &root)?;
        let back = open_root_grant(&ann.kem(), "cong-1", "ann", &g)?;
        assert_eq!(back.record_id("people", "1"), root.record_id("people", "1"));
        assert!(open_root_grant(&ann.kem(), "cong-1", "bob", &g).is_err());
        assert!(open_root_grant(&ann.kem(), "cong-2", "ann", &g).is_err());
        assert!(open_root_grant(&PersonalKeys::generate()?.kem(), "cong-1", "ann", &g).is_err());
        // A root grant is not a domain grant, and the other way round.
        assert!(open_grant(&ann.kem(), "cong-1", "ann", ROOT_DOMAIN, 0, &g).is_err());
        let d = grant(&ann.public().kem, "cong-1", "ann", ROOT_DOMAIN, 0, &DomainSecret::generate()?)?;
        assert!(open_root_grant(&ann.kem(), "cong-1", "ann", &d).is_err());
        Ok(())
    }
}
