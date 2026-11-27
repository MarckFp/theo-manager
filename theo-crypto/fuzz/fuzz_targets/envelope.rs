//! R-308: no input may make envelope, vault, grant or backup parsing panic, and no input made up
//! by the fuzzer may open.
#![no_main]

use std::sync::OnceLock;

use libfuzzer_sys::fuzz_target;
use theo_crypto::envelope::{self, RecordContext};
use theo_crypto::{DomainId, Keyring, PersonalKeys, Vault};

struct Keys {
    keyring: Keyring,
    person: PersonalKeys,
}

fn keys() -> Option<&'static Keys> {
    static KEYS: OnceLock<Option<Keys>> = OnceLock::new();
    KEYS.get_or_init(|| {
        let mut keyring = Keyring::new();
        keyring.rotate(DomainId::PEOPLE).ok()?;
        keyring.rotate(DomainId::REPORTS).ok()?;
        Some(Keys { keyring, person: PersonalKeys::generate().ok()? })
    })
    .as_ref()
}

fuzz_target!(|data: &[u8]| {
    let Some(k) = keys() else { return };
    let ctx = RecordContext { congregation_id: "c", collection: "x", scope_id: "", record_id: "r", version: 1 };
    let _ = envelope::header(data);
    assert!(envelope::open(&k.keyring, &ctx, data).is_err());
    assert!(envelope::open_sealed(&k.person.kem(), &ctx, data).is_err());
    let _ = Vault::from_bytes(data);
    assert!(theo_crypto::grant::open_grant(&k.person.kem(), "c", "m", DomainId::PEOPLE, 1, data).is_err());
});
