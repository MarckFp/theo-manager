//! The vault, keyring, envelopes, accounts, grants and signatures, through the public API only.
//! All data is synthetic.

use theo_crypto::envelope::{self, RecordContext, SUITE_DOMAIN, SUITE_SEALED};
use theo_crypto::grant::{grant, open_grant};
use theo_crypto::{
    account_salt, AccountKeys, DomainId, Error, HybridVerifier, KdfParams, Keyring, PersonalKeys, PublicIdentity, RecoveryKey, Vault,
};

// The minimum allowed cost keeps the tests fast.
const FAST: KdfParams = KdfParams::MINIMUM;
const CONG: &str = "cong-riverside";

fn ctx(record_id: &str) -> RecordContext<'_> {
    RecordContext { congregation_id: CONG, collection: "person", scope_id: "", record_id, version: 1 }
}

fn keyring() -> Keyring {
    let mut k = Keyring::new();
    for d in [DomainId::SCHEDULE, DomainId::PEOPLE, DomainId::REPORTS, DomainId::TERRITORIES] {
        assert_eq!(k.rotate(d), Ok(1));
    }
    k
}

/// A tiny deterministic generator for the fuzz-like tests.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
}

// ----------------------------------------------------------------------------------------------
// Vault
// ----------------------------------------------------------------------------------------------

#[test]
fn vault_unlocks_with_the_password_only() -> Result<(), Error> {
    let (vault, root) = Vault::create(CONG, "Blue-Lamp-7", FAST)?;
    let stored = Vault::from_bytes(&vault.to_bytes())?;
    assert_eq!(stored, vault);
    // The same root key comes back: a keyring entry wrapped before opens after.
    let mut k = keyring();
    let wrapped = k.wrap_entry(&root, CONG, DomainId::PEOPLE, 1)?;
    let again = stored.unlock(CONG, "Blue-Lamp-7")?;
    k.unwrap_entry(&again, CONG, DomainId::PEOPLE, 1, &wrapped)?;
    assert_eq!(stored.unlock(CONG, "blue-lamp-7").err(), Some(Error::WrongPasswordOrDamaged));
    // A header copied to another congregation does not open.
    assert_eq!(stored.unlock("cong-other", "Blue-Lamp-7").err(), Some(Error::WrongPasswordOrDamaged));
    Ok(())
}

#[test]
fn vault_header_changes_and_weak_parameters_are_refused() -> Result<(), Error> {
    let (vault, _) = Vault::create(CONG, "pw", FAST)?;
    let bytes = vault.to_bytes();
    // Any changed byte after the parameters (salt, wrapped key) makes the unlock fail.
    for i in 17..bytes.len() - 1 {
        let mut bad = bytes.clone();
        bad[i] ^= 0x40;
        assert!(Vault::from_bytes(&bad).and_then(|v| v.unlock(CONG, "pw")).is_err(), "byte {i}");
    }
    // Raising t passes the minimum check but breaks the binding.
    let mut raised = bytes.clone();
    raised[9] += 1;
    assert_eq!(Vault::from_bytes(&raised)?.unlock(CONG, "pw").err(), Some(Error::WrongPasswordOrDamaged));
    // Lowering memory is refused before any work.
    let mut weak = bytes.clone();
    weak[5..9].copy_from_slice(&1024u32.to_le_bytes());
    assert_eq!(Vault::from_bytes(&weak).err(), Some(Error::WeakParameters));
    assert_eq!(Vault::create(CONG, "pw", KdfParams { m_kib: 1024, t: 1, p: 1 }).err(), Some(Error::WeakParameters));
    let mut newer = bytes.clone();
    newer[4] = 9;
    assert_eq!(Vault::from_bytes(&newer).err(), Some(Error::UnknownFormat));
    assert_eq!(Vault::from_bytes(&bytes[..bytes.len() - 1]).err(), Some(Error::NotAnEnvelope));
    Ok(())
}

#[test]
fn record_ids_are_stable_per_root_and_look_random() -> Result<(), Error> {
    let (vault, root) = Vault::create(CONG, "pw", FAST)?;
    let again = vault.unlock(CONG, "pw")?;
    assert_eq!(root.record_id("people", "1"), again.record_id("people", "1"));
    assert_ne!(root.record_id("people", "1"), root.record_id("people", "2"));
    assert_ne!(root.record_id("people", "1"), root.record_id("groups", "1"));
    // Fields cannot slide into each other.
    assert_ne!(root.record_id("ab", "c"), root.record_id("a", "bc"));
    let (_, other) = Vault::create(CONG, "pw", FAST)?;
    assert_ne!(root.record_id("people", "1"), other.record_id("people", "1"));
    Ok(())
}

#[test]
fn password_change_and_recovery_keep_the_same_root() -> Result<(), Error> {
    let (mut vault, root) = Vault::create(CONG, "old", FAST)?;
    let recovery = vault.add_recovery(CONG, &root)?;
    assert!(vault.has_recovery());
    let k = keyring();
    let wrapped = k.wrap_entry(&root, CONG, DomainId::REPORTS, 1)?;

    let changed = vault.change_password(CONG, &root, "new", FAST)?;
    assert_eq!(changed.unlock(CONG, "old").err(), Some(Error::WrongPasswordOrDamaged));
    for r in [changed.unlock(CONG, "new")?, changed.unlock_with_recovery(CONG, &recovery)?] {
        Keyring::new().unwrap_entry(&r, CONG, DomainId::REPORTS, 1, &wrapped)?;
    }
    // A wrong recovery key, or a removed kit, does not open.
    let wrong = RecoveryKey::from_bytes(&[7u8; 32])?;
    assert_eq!(changed.unlock_with_recovery(CONG, &wrong).err(), Some(Error::WrongPasswordOrDamaged));
    let mut off = changed.clone();
    off.remove_recovery();
    assert_eq!(off.unlock_with_recovery(CONG, &recovery).err(), Some(Error::NoKey));
    assert_eq!(Vault::from_bytes(&off.to_bytes())?, off);
    Ok(())
}

// ----------------------------------------------------------------------------------------------
// Keyring
// ----------------------------------------------------------------------------------------------

#[test]
fn keyring_rotation_keeps_old_versions() -> Result<(), Error> {
    let (_, root) = Vault::create(CONG, "pw", FAST)?;
    let mut k = keyring();
    assert_eq!(k.rotate(DomainId::PEOPLE)?, 2);
    assert_eq!(k.current(DomainId::PEOPLE).map(|c| c.0), Some(2));
    assert_eq!(k.current(DomainId::REPORTS).map(|c| c.0), Some(1));
    assert!(k.get(DomainId::PEOPLE, 1).is_some());
    assert_eq!(k.versions().count(), 5);
    // Each row is bound to congregation, domain and version.
    let row = k.wrap_entry(&root, CONG, DomainId::PEOPLE, 2)?;
    let mut other = Keyring::new();
    assert!(other.unwrap_entry(&root, "cong-other", DomainId::PEOPLE, 2, &row).is_err());
    assert!(other.unwrap_entry(&root, CONG, DomainId::REPORTS, 2, &row).is_err());
    assert!(other.unwrap_entry(&root, CONG, DomainId::PEOPLE, 1, &row).is_err());
    other.unwrap_entry(&root, CONG, DomainId::PEOPLE, 2, &row)?;
    // Same secret, so the same blind index.
    let a = k.get(DomainId::PEOPLE, 2).map(|s| s.keys().blind_index("email", b"ann@example.org"));
    let b = other.get(DomainId::PEOPLE, 2).map(|s| s.keys().blind_index("email", b"ann@example.org"));
    assert_eq!(a, b);
    let other_field = k.get(DomainId::PEOPLE, 2).map(|s| s.keys().blind_index("phone", b"ann@example.org"));
    assert_ne!(a, other_field);
    k.clear();
    assert_eq!(k.versions().count(), 0);
    Ok(())
}

// ----------------------------------------------------------------------------------------------
// Envelopes
// ----------------------------------------------------------------------------------------------

#[test]
fn domain_envelope_round_trip_and_aad_binding() -> Result<(), Error> {
    let k = keyring();
    let c = ctx("person:1");
    let sealed = envelope::seal(&k, DomainId::PEOPLE, &c, b"Ann Example, 12 Sample Street")?;
    let h = envelope::header(&sealed)?;
    assert_eq!((h.suite, h.domain, h.key_version), (SUITE_DOMAIN, DomainId::PEOPLE, 1));
    assert_eq!(envelope::open(&k, &c, &sealed)?.as_slice(), b"Ann Example, 12 Sample Street");
    // Fresh nonce every time (R-303).
    assert_ne!(envelope::seal(&k, DomainId::PEOPLE, &c, b"x")?, envelope::seal(&k, DomainId::PEOPLE, &c, b"x")?);
    // Moving the record anywhere else fails (R-304).
    let moved = [
        RecordContext { congregation_id: "cong-other", ..c },
        RecordContext { collection: "report", ..c },
        RecordContext { scope_id: "p2", ..c },
        RecordContext { record_id: "person:2", ..c },
        RecordContext { version: 2, ..c },
    ];
    for m in moved {
        assert_eq!(envelope::open(&k, &m, &sealed).err(), Some(Error::Damaged), "{m:?}");
    }
    // Every changed byte fails, never panics.
    for i in 0..sealed.len() {
        let mut bad = sealed.clone();
        bad[i] ^= 1;
        assert!(envelope::open(&k, &c, &bad).is_err(), "byte {i}");
    }
    Ok(())
}

#[test]
fn rotation_new_records_use_the_new_version_and_old_ones_still_open() -> Result<(), Error> {
    let mut k = keyring();
    let c = ctx("report:1");
    let old = envelope::seal(&k, DomainId::REPORTS, &c, b"old")?;
    k.rotate(DomainId::REPORTS)?;
    let new = envelope::seal(&k, DomainId::REPORTS, &c, b"new")?;
    assert_eq!(envelope::header(&new)?.key_version, 2);
    assert_eq!(envelope::open(&k, &c, &old)?.as_slice(), b"old");
    assert_eq!(envelope::open(&k, &c, &new)?.as_slice(), b"new");
    // A reader without the domain cannot open it.
    let mut people_only = Keyring::new();
    people_only.rotate(DomainId::PEOPLE)?;
    assert_eq!(envelope::open(&people_only, &c, &new).err(), Some(Error::NoKey));
    assert_eq!(envelope::seal(&people_only, DomainId::REPORTS, &c, b"x").err(), Some(Error::NoKey));
    Ok(())
}

#[test]
fn sealed_submission_opens_for_domain_readers_and_the_author_only() -> Result<(), Error> {
    let k = keyring();
    let (version, secret) = k.current(DomainId::REPORTS).ok_or(Error::NoKey)?;
    let domain_pk = secret.keys().public_key();
    let author = PersonalKeys::generate()?;
    let stranger = PersonalKeys::generate()?;
    let c = ctx("report:2026-11:p7");
    let sealed = envelope::seal_to(&[&domain_pk, &author.public().kem], DomainId::REPORTS, version, &c, b"hours: 12")?;
    assert_eq!(envelope::header(&sealed)?.suite, SUITE_SEALED);
    assert_eq!(envelope::open(&k, &c, &sealed)?.as_slice(), b"hours: 12");
    assert_eq!(envelope::open_sealed(&author.kem(), &c, &sealed)?.as_slice(), b"hours: 12");
    assert_eq!(envelope::open_sealed(&stranger.kem(), &c, &sealed).err(), Some(Error::NoKey));
    assert_eq!(envelope::open_sealed(&author.kem(), &ctx("report:other"), &sealed).err(), Some(Error::Damaged));
    // A domain-key envelope is not a sealed one.
    let plain = envelope::seal(&k, DomainId::REPORTS, &c, b"x")?;
    assert_eq!(envelope::open_sealed(&author.kem(), &c, &plain).err(), Some(Error::UnknownSuite));
    // No recipients is refused.
    assert_eq!(envelope::seal_to(&[], DomainId::REPORTS, version, &c, b"x").err(), Some(Error::NoKey));
    // Changes anywhere (header, recipients, body) fail; sample bytes to keep the test quick.
    for i in (0..sealed.len()).step_by(97).chain([sealed.len() - 1]) {
        let mut bad = sealed.clone();
        bad[i] ^= 0x10;
        assert!(envelope::open(&k, &c, &bad).is_err(), "byte {i}");
    }
    Ok(())
}

#[test]
fn unknown_formats_and_suites_fail_closed() -> Result<(), Error> {
    let k = keyring();
    let c = ctx("x");
    let sealed = envelope::seal(&k, DomainId::SCHEDULE, &c, b"meeting")?;
    let mut fmt = sealed.clone();
    fmt[2] = 0x02;
    assert_eq!(envelope::open(&k, &c, &fmt).err(), Some(Error::UnknownFormat));
    let mut suite = sealed.clone();
    suite[3] = 0x7f;
    assert_eq!(envelope::open(&k, &c, &suite).err(), Some(Error::UnknownSuite));
    assert_eq!(envelope::header(b"XX\x01\x01\x01\0\0\0\0").err(), Some(Error::NotAnEnvelope));
    assert_eq!(envelope::header(b"TH").err(), Some(Error::NotAnEnvelope));
    Ok(())
}

#[test]
fn padding_hides_exact_sizes() -> Result<(), Error> {
    assert_eq!(envelope::padme(0), 0);
    assert_eq!(envelope::padme(1), 1);
    assert_eq!(envelope::padme(9), 10);
    assert_eq!(envelope::padme(100), 104);
    assert_eq!(envelope::padme(1000), 1024);
    for n in 2..5000usize {
        let p = envelope::padme(n);
        assert!(p >= n && (p - n) * 100 <= n * 12 + 100, "{n} -> {p}");
    }
    let k = keyring();
    let c = ctx("x");
    let a = envelope::seal(&k, DomainId::PEOPLE, &c, &[1u8; 1000])?;
    let b = envelope::seal(&k, DomainId::PEOPLE, &c, &[1u8; 1010])?;
    assert_eq!(a.len(), b.len());
    assert_eq!(envelope::open(&k, &c, &envelope::seal(&k, DomainId::PEOPLE, &c, b"")?)?.len(), 0);
    Ok(())
}

#[test]
fn garbage_never_opens_and_never_panics() -> Result<(), Error> {
    let k = keyring();
    let c = ctx("x");
    let reader = PersonalKeys::generate()?;
    let (version, secret) = k.current(DomainId::REPORTS).ok_or(Error::NoKey)?;
    let samples = [
        envelope::seal(&k, DomainId::PEOPLE, &c, b"payload")?,
        envelope::seal_to(&[&secret.keys().public_key(), &reader.public().kem], DomainId::REPORTS, version, &c, b"payload")?,
    ];
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    for s in &samples {
        // Every truncation.
        for n in 0..s.len() {
            assert!(envelope::open(&k, &c, &s[..n]).is_err());
            assert!(envelope::open_sealed(&reader.kem(), &c, &s[..n]).is_err());
        }
        // Random mutations and extensions.
        for _ in 0..300 {
            let mut m = s.clone();
            for _ in 0..1 + rng.below(4) {
                let i = rng.below(m.len());
                m[i] = rng.next() as u8;
            }
            if rng.below(4) == 0 {
                m.extend((0..rng.below(64)).map(|_| rng.next() as u8));
            }
            if m != *s {
                assert!(envelope::open(&k, &c, &m).is_err());
            }
        }
    }
    // Random bytes with a valid-looking prefix.
    for _ in 0..500 {
        let mut m = b"TH\x01".to_vec();
        m.extend((0..rng.below(2600)).map(|_| rng.next() as u8));
        assert!(envelope::open(&k, &c, &m).is_err());
        assert!(envelope::open_sealed(&reader.kem(), &c, &m).is_err());
        assert!(theo_crypto::open_with_password("pw", &m).is_err());
        assert!(Vault::from_bytes(&m).is_err());
    }
    Ok(())
}

// ----------------------------------------------------------------------------------------------
// Accounts and grants
// ----------------------------------------------------------------------------------------------

#[test]
fn account_salt_is_deterministic_and_normalized() {
    let a = account_salt("https://theo.example", "theo", "riverside", "Ann.Example ");
    assert_eq!(a, account_salt("https://theo.example", "theo", "riverside", "ann.example"));
    assert_ne!(a, account_salt("https://theo.example", "theo", "lakeside", "ann.example"));
    // Fields cannot slide into each other.
    assert_ne!(account_salt("a", "bc", "d", "e"), account_salt("ab", "c", "d", "e"));
}

#[test]
fn personal_keys_open_only_with_the_account_password() -> Result<(), Error> {
    let salt = account_salt("https://theo.example", "theo", "riverside", "ann");
    let account = AccountKeys::derive("Correct-Horse-9", &salt, FAST)?;
    assert_ne!(account.auth_key(), &[0u8; 32]);
    let keys = PersonalKeys::generate()?;
    let wrapped = keys.wrap(&account, "user:ann")?;
    let again = AccountKeys::derive("Correct-Horse-9", &salt, FAST)?;
    assert_eq!(account.auth_key(), again.auth_key());
    let opened = PersonalKeys::unwrap(&again, "user:ann", &wrapped)?;
    assert_eq!(opened.public(), keys.public());
    let wrong = AccountKeys::derive("correct-horse-9", &salt, FAST)?;
    assert_ne!(wrong.auth_key(), account.auth_key());
    assert_eq!(PersonalKeys::unwrap(&wrong, "user:ann", &wrapped).err(), Some(Error::WrongPasswordOrDamaged));
    assert_eq!(PersonalKeys::unwrap(&again, "user:bob", &wrapped).err(), Some(Error::WrongPasswordOrDamaged));
    // Private labels open only with the same account keys.
    let label = account.seal_label("Riverside")?;
    assert_eq!(again.open_label(&label)?, "Riverside");
    assert!(wrong.open_label(&label).is_err());
    // The public identity survives storage.
    let public = keys.public();
    let bytes = public.to_bytes();
    assert_eq!(PublicIdentity::from_bytes(&bytes)?, public);
    assert_eq!(PublicIdentity::from_bytes(&bytes[1..]).err(), Some(Error::InvalidKey));
    Ok(())
}

#[test]
fn grants_give_a_domain_to_one_member() -> Result<(), Error> {
    let k = keyring();
    let ann = PersonalKeys::generate()?;
    let bob = PersonalKeys::generate()?;
    let secret = k.get(DomainId::TERRITORIES, 1).ok_or(Error::NoKey)?;
    let g = grant(&ann.public().kem, CONG, "user:ann", DomainId::TERRITORIES, 1, secret)?;
    let got = open_grant(&ann.kem(), CONG, "user:ann", DomainId::TERRITORIES, 1, &g)?;
    // Ann can now read territory records.
    let c = ctx("territory:12");
    let record = envelope::seal(&k, DomainId::TERRITORIES, &c, b"card 12")?;
    let mut hers = Keyring::new();
    hers.insert(DomainId::TERRITORIES, 1, got);
    assert_eq!(envelope::open(&hers, &c, &record)?.as_slice(), b"card 12");
    // Nobody else, and no relabelling.
    assert_eq!(open_grant(&bob.kem(), CONG, "user:ann", DomainId::TERRITORIES, 1, &g).err(), Some(Error::NoKey));
    assert_eq!(open_grant(&ann.kem(), CONG, "user:bob", DomainId::TERRITORIES, 1, &g).err(), Some(Error::NoKey));
    assert_eq!(open_grant(&ann.kem(), CONG, "user:ann", DomainId::PEOPLE, 1, &g).err(), Some(Error::NoKey));
    assert_eq!(open_grant(&ann.kem(), CONG, "user:ann", DomainId::TERRITORIES, 2, &g).err(), Some(Error::NoKey));
    assert_eq!(open_grant(&ann.kem(), "cong-other", "user:ann", DomainId::TERRITORIES, 1, &g).err(), Some(Error::NoKey));
    assert!(open_grant(&ann.kem(), CONG, "user:ann", DomainId::TERRITORIES, 1, &g[..g.len() - 1]).is_err());
    Ok(())
}

// ----------------------------------------------------------------------------------------------
// Signatures
// ----------------------------------------------------------------------------------------------

#[test]
fn hybrid_signature_needs_both_halves() -> Result<(), Error> {
    let keys = PersonalKeys::generate()?;
    let signer = keys.signer();
    let verifier = HybridVerifier::from_bytes(&signer.verifier().to_bytes())?;
    let sig = signer.sign("report", b"2026-11: 12 h")?;
    assert_eq!(sig.len(), theo_crypto::sign::SIGNATURE_LEN);
    verifier.verify("report", b"2026-11: 12 h", &sig)?;
    assert_eq!(verifier.verify("report", b"2026-11: 13 h", &sig).err(), Some(Error::BadSignature));
    assert_eq!(verifier.verify("manifest", b"2026-11: 12 h", &sig).err(), Some(Error::BadSignature));
    // Breaking only the ML-DSA half, or only the Ed25519 half, is enough to fail.
    for i in [1, 2000, sig.len() - 64, sig.len() - 1] {
        let mut bad = sig.clone();
        bad[i] ^= 1;
        assert_eq!(verifier.verify("report", b"2026-11: 12 h", &bad).err(), Some(Error::BadSignature), "byte {i}");
    }
    let other = PersonalKeys::generate()?.signer().verifier();
    assert_eq!(other.verify("report", b"2026-11: 12 h", &sig).err(), Some(Error::BadSignature));
    assert_eq!(verifier.verify("report", b"x", &sig[..10]).err(), Some(Error::BadSignature));
    assert_eq!(signer.sign(&"c".repeat(201), b"x").err(), Some(Error::InvalidKey));
    Ok(())
}
