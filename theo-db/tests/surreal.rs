//! The SurrealDB engine against a live server. Skipped unless `THEO_SURREAL_URL` is set to a
//! server that has `surreal/schema.surql` imported (in namespace `theo`, database `theo`), e.g.
//!
//!   surreal start --user root --pass root memory          (a throwaway local server)
//!   curl -X POST http://127.0.0.1:8000/sql -u root:root -H 'Accept: application/json' \
//!       --data-binary 'DEFINE NAMESPACE IF NOT EXISTS theo; USE NS theo; DEFINE DATABASE IF NOT EXISTS theo;'
//!   curl -X POST http://127.0.0.1:8000/sql -u root:root -H 'Accept: application/json' \
//!       -H 'Surreal-NS: theo' -H 'Surreal-DB: theo' --data-binary @theo-db/surreal/schema.surql
//!   THEO_SURREAL_URL=http://127.0.0.1:8000 cargo test -p theo-db --test surreal
//!
//! The test itself uses no owner credentials: it signs up throwaway accounts, as the app does.

use theo_crypto::invite::InviteToken;
use theo_db::surreal::{join, sign_in, sign_up, Server};
use theo_db::{conformance, Db, DbConfig, DbError, RecordId};

fn url() -> Option<String> {
    std::env::var("THEO_SURREAL_URL").ok().filter(|u| !u.is_empty())
}

fn err(e: DbError) -> String {
    e.to_string()
}

#[test]
fn surreal_engine_conforms_and_keeps_congregations_apart() -> Result<(), String> {
    let Some(url) = url() else {
        eprintln!("THEO_SURREAL_URL is not set: skipped");
        return Ok(());
    };
    pollster::block_on(async {
        let server = Server::new(&url);
        let tag = RecordId::random().map_err(err)?.to_hex();
        let (ann, bob) = (format!("ann-{}", &tag[..8]), format!("bob-{}", &tag[..8]));
        let a = sign_up(&server, &ann, "auth-key-of-ann").await.map_err(err)?;
        let b = sign_up(&server, &bob, "auth-key-of-bob").await.map_err(err)?;
        if sign_up(&server, &ann, "again").await.is_ok() {
            return Err("a username was taken twice".into());
        }
        if sign_in(&server, &ann, "wrong").await.is_ok() {
            return Err("a wrong auth key signed in".into());
        }
        let a = sign_in(&server, &ann, "auth-key-of-ann").await.map_err(err).map(|_| a)?;

        let cong = RecordId::random().map_err(err)?;
        a.create_congregation(cong, "encrypted-label").await.map_err(err)?;
        let ms = a.memberships().await.map_err(err)?;
        if !ms.iter().any(|m| m.congregation == cong.to_hex() && m.role == "owner" && m.label == "encrypted-label") {
            return Err(format!("the owner membership is missing: {ms:?}"));
        }
        let db = Db::open(DbConfig::Surreal { session: a.clone(), congregation: cong.to_hex() }).await.map_err(err)?;
        conformance::run(&db).await?;

        // Bob is not a member: he sees nothing and cannot write.
        if !b.memberships().await.map_err(err)?.is_empty() {
            return Err("bob sees a membership".into());
        }
        let bobs = Db::open(DbConfig::Surreal { session: b.clone(), congregation: cong.to_hex() }).await.map_err(err)?;
        if bobs.read_header().await.map_err(err)?.is_some() || !bobs.changes(0).await.map_err(err)?.is_empty() {
            return Err("bob can read ann's congregation".into());
        }
        let _ = bobs.write_header(b"bob", Some(2)).await;
        if db.read_header().await.map_err(err)?.map(|h| h.bytes) != Some(b"h2".to_vec()) {
            return Err("bob changed ann's header".into());
        }

        // Leaving and deleting.
        Db::destroy(DbConfig::Surreal { session: a.clone(), congregation: cong.to_hex() }).await.map_err(err)?;
        if !a.memberships().await.map_err(err)?.is_empty() {
            return Err("the congregation is still there".into());
        }
        Ok(())
    })
}

#[test]
fn invites_roles_grants_and_the_audit_log() -> Result<(), String> {
    let Some(url) = url() else {
        eprintln!("THEO_SURREAL_URL is not set: skipped");
        return Ok(());
    };
    pollster::block_on(async {
        let server = Server::new(&url);
        let tag = RecordId::random().map_err(err)?.to_hex();
        let (ann, bob, eve) = (format!("ann-{}", &tag[..8]), format!("bob-{}", &tag[..8]), format!("eve-{}", &tag[..8]));
        let a = sign_up(&server, &ann, "key-ann").await.map_err(err)?;
        let b = sign_up(&server, &bob, "key-bob").await.map_err(err)?;
        let e = sign_up(&server, &eve, "key-eve").await.map_err(err)?;

        // The account's own data; nobody else reads the wrapped keys.
        a.set_personal_keys("wrapped-ann", "public-ann").await.map_err(err)?;
        a.set_profile("sealed-name").await.map_err(err)?;
        let data = a.account_data().await.map_err(err)?;
        if (data.keys.as_deref(), data.public.as_deref(), data.profile.as_deref()) != (Some("wrapped-ann"), Some("public-ann"), Some("sealed-name")) {
            return Err(format!("the account data did not round-trip: {data:?}"));
        }

        let cong = RecordId::random().map_err(err)?;
        a.create_congregation(cong, "label-ann").await.map_err(err)?;
        let db = Db::open(DbConfig::Surreal { session: a.clone(), congregation: cong.to_hex() }).await.map_err(err)?;
        db.write_header(b"vault", None).await.map_err(err)?;
        let rid = RecordId::random().map_err(err)?;
        db.put(&theo_db::NewRecord { collection: "people", record_id: rid, scope_id: None, envelope: b"ct" }, None).await.map_err(err)?;

        // Eve is no member: she cannot make an invite.
        let t = InviteToken::generate().map_err(|e| e.to_string())?;
        let events = vec!["events".to_string()];
        if e.create_invite(cong, &t.id(), "member", "bundle", &events, 7).await.is_ok() {
            return Err("a stranger made an invite".into());
        }
        a.create_invite(cong, &t.id(), "member", "bundle", &events, 7).await.map_err(err)?;
        if join(&server, &bob, "key-bob", "not-the-proof").await.is_ok() || join(&server, &bob, "wrong-key", &t.proof()).await.is_ok() {
            return Err("an invite was used without its proof or with a wrong password".into());
        }
        join(&server, &bob, "key-bob", &t.proof()).await.map_err(err)?;
        if join(&server, &eve, "key-eve", &t.proof()).await.is_ok() {
            return Err("an invite was used twice".into());
        }
        let ms = b.memberships().await.map_err(err)?;
        if !ms.iter().any(|m| m.congregation == cong.to_hex() && m.role == "member" && m.writes == events && !m.root_grant) {
            return Err(format!("bob is not a member who writes events: {ms:?}"));
        }
        let used = b.used_invite(&t.id()).await.map_err(err)?.ok_or("bob cannot read his invite")?;
        if used.bundle.as_deref() != Some("bundle") || used.kind != "member" {
            return Err(format!("the invite reads wrong: {used:?}"));
        }
        b.set_label(cong, "label-bob").await.map_err(err)?;
        b.put_grant(cong, 1, 1, "grant-bob").await.map_err(err)?;
        b.clear_bundle(&t.id()).await.map_err(err)?;
        if b.grants(cong).await.map_err(err)?.iter().map(|g| (g.domain, g.key_version, g.sealed.as_str())).collect::<Vec<_>>() != vec![(1, 1, "grant-bob")] {
            return Err("bob's grant did not round-trip".into());
        }

        // The member reads and writes records, but not the vault header (R-256), keys or the log.
        let bobs = Db::open(DbConfig::Surreal { session: b.clone(), congregation: cong.to_hex() }).await.map_err(err)?;
        if bobs.changes(0).await.map_err(err)?.len() != 1 {
            return Err("bob does not see the records".into());
        }
        bobs.put(&theo_db::NewRecord { collection: "events", record_id: RecordId::random().map_err(err)?, scope_id: None, envelope: b"ct2" }, None).await.map_err(err)?;
        if bobs.read_header().await.map_err(err)?.is_some() {
            return Err("a member reads the vault header".into());
        }
        let _ = bobs.write_header(b"x", Some(1)).await;
        if db.read_header().await.map_err(err)?.map(|h| h.bytes) != Some(b"vault".to_vec()) {
            return Err("a member wrote the vault header".into());
        }

        // Overwriting (R-241): bob writes events only, and cannot give himself more.
        if bobs.put(&theo_db::NewRecord { collection: "people", record_id: rid, scope_id: None, envelope: b"evil" }, Some(1)).await != Err(DbError::Denied) {
            return Err("a member overwrote a people record".into());
        }
        if db.get("people", rid).await.map_err(err)?.map(|r| r.envelope) != Some(b"ct".to_vec()) {
            return Err("the people record changed".into());
        }
        let _ = b.query_raw("UPDATE membership SET writes = ['people'], role = 'full' WHERE account = $auth.id;").await;
        let mine = b.memberships().await.map_err(err)?;
        if !mine.iter().any(|m| m.writes == events && m.role == "member") {
            return Err(format!("bob changed his own permissions: {mine:?}"));
        }
        if b.set_member_writes(cong, &bob, &["people".to_string()]).await.is_ok() {
            return Err("a member set write permissions".into());
        }
        a.set_member_writes(cong, &bob, &["events".to_string(), "people".to_string()]).await.map_err(err)?;
        bobs.put(&theo_db::NewRecord { collection: "people", record_id: rid, scope_id: None, envelope: b"ok" }, Some(1)).await.map_err(err)?;

        // Members and grants, as the owner sees them; a grant to bob, and ann's root grant.
        let members = a.members(cong).await.map_err(err)?;
        if members.len() != 2 || !members.iter().any(|m| m.username == bob && m.role == "member") {
            return Err(format!("the owner does not see the members: {members:?}"));
        }
        a.put_grant_for(cong, &bob, 2, 1, "grant-for-bob").await.map_err(err)?;
        a.put_grant(cong, 0, 0, "root-of-ann").await.map_err(err)?;
        if !a.memberships().await.map_err(err)?.iter().any(|m| m.root_grant) {
            return Err("ann's root grant is not seen".into());
        }
        let grants = a.member_grants(cong).await.map_err(err)?;
        if !grants.contains(&(bob.clone(), 2, 1)) || !grants.contains(&(ann.clone(), 0, 0)) {
            return Err(format!("the owner does not see the grants: {grants:?}"));
        }
        if b.remove_member(cong, &ann).await.is_ok() {
            return Err("a member removed the owner".into());
        }
        if !b.audit(cong, 0, 10).await.map_err(err)?.is_empty() {
            return Err("a member reads the audit log".into());
        }

        // The owner sees who joined, and the audit log.
        let invites = a.invites(cong).await.map_err(err)?;
        if !invites.iter().any(|i| i.id == t.id() && i.used_by.as_deref() == Some(bob.as_str()) && !i.expired && i.expires.starts_with("20")) {
            return Err(format!("the owner does not see the used invite with its expiry: {invites:?}"));
        }
        // An invite that expires at once: listed as expired, and it cannot be used.
        let gone = InviteToken::generate().map_err(|e| e.to_string())?;
        a.create_invite(cong, &gone.id(), "member", "bundle", &events, 0).await.map_err(err)?;
        if !a.invites(cong).await.map_err(err)?.iter().any(|i| i.id == gone.id() && i.expired) {
            return Err("an expired invite is not listed as expired".into());
        }
        if join(&server, &eve, "key-eve", &gone.proof()).await.is_ok() {
            return Err("an expired invite was used".into());
        }
        let log = a.audit(cong, 0, 50).await.map_err(err)?;
        if a.audit(cong, 1, 1).await.map_err(err)?.first() != log.get(1) {
            return Err("the audit log does not page".into());
        }
        for (what, action) in [("member", "create"), ("invite", "use"), ("record", "create"), ("grant", "create")] {
            if !log.iter().any(|l| l.what == what && l.action == action) {
                return Err(format!("the audit log has no {what} {action}: {log:?}"));
            }
        }
        if !log.iter().any(|l| l.what == "record" && l.actor.as_deref() == Some(bob.as_str())) {
            return Err("the audit log does not name bob as an author".into());
        }

        // The owner removes bob: he loses the membership, his grants and every read at once.
        a.remove_member(cong, &bob).await.map_err(err)?;
        if !b.memberships().await.map_err(err)?.is_empty() || !b.grants(cong).await.map_err(err)?.is_empty() {
            return Err("bob is still a member, or still has grants".into());
        }
        if !bobs.changes(0).await.map_err(err)?.is_empty() {
            return Err("a removed member still reads records".into());
        }
        if a.members(cong).await.map_err(err)?.len() != 1 {
            return Err("bob is still listed".into());
        }
        Db::destroy(DbConfig::Surreal { session: a.clone(), congregation: cong.to_hex() }).await.map_err(err)?;
        Ok(())
    })
}

#[test]
fn only_the_owner_deletes_and_deleting_ends_everyones_access() -> Result<(), String> {
    let Some(url) = url() else {
        eprintln!("THEO_SURREAL_URL is not set: skipped");
        return Ok(());
    };
    pollster::block_on(async {
        let server = Server::new(&url);
        let tag = RecordId::random().map_err(err)?.to_hex();
        let names = ["own", "adm", "mem", "out"].map(|n| format!("{n}-{}", &tag[..8]));
        let owner = sign_up(&server, &names[0], "k0").await.map_err(err)?;
        let admin = sign_up(&server, &names[1], "k1").await.map_err(err)?;
        let member = sign_up(&server, &names[2], "k2").await.map_err(err)?;
        sign_up(&server, &names[3], "k3").await.map_err(err)?;

        let cong = RecordId::random().map_err(err)?;
        owner.create_congregation(cong, "label").await.map_err(err)?;
        let db = Db::open(DbConfig::Surreal { session: owner.clone(), congregation: cong.to_hex() }).await.map_err(err)?;
        db.write_header(b"vault", None).await.map_err(err)?;
        db.put(&theo_db::NewRecord { collection: "people", record_id: RecordId::random().map_err(err)?, scope_id: None, envelope: b"ct" }, None).await.map_err(err)?;

        // A full-access admin and a member join.
        let (full, plain, later) = (InviteToken::generate().map_err(|e| e.to_string())?, InviteToken::generate().map_err(|e| e.to_string())?, InviteToken::generate().map_err(|e| e.to_string())?);
        owner.create_invite(cong, &full.id(), "full", "b", &[], 7).await.map_err(err)?;
        owner.create_invite(cong, &plain.id(), "member", "b", &["events".to_string()], 7).await.map_err(err)?;
        owner.create_invite(cong, &later.id(), "member", "b", &[], 7).await.map_err(err)?;
        join(&server, &names[1], "k1", &full.proof()).await.map_err(err)?;
        join(&server, &names[2], "k2", &plain.proof()).await.map_err(err)?;
        member.put_grant(cong, 1, 1, "g").await.map_err(err)?;

        // Only the owner deletes the congregation.
        if admin.delete_congregation(cong).await.is_ok() || member.delete_congregation(cong).await.is_ok() {
            return Err("someone other than the owner deleted the congregation".into());
        }
        if owner.memberships().await.map_err(err)?.len() != 1 || db.changes(0).await.map_err(err)?.is_empty() {
            return Err("a refused delete still removed something".into());
        }
        // A member cannot remove anyone; nobody removes the owner.
        if member.remove_member(cong, &names[1]).await.is_ok() || admin.remove_member(cong, &names[0]).await.is_ok() {
            return Err("a member removed someone, or the owner was removed".into());
        }
        // An administrator can stop one member's access.
        admin.remove_member(cong, &names[2]).await.map_err(err)?;
        let members_db = Db::open(DbConfig::Surreal { session: member.clone(), congregation: cong.to_hex() }).await.map_err(err)?;
        if !member.memberships().await.map_err(err)?.is_empty() || !members_db.changes(0).await.map_err(err)?.is_empty() || !member.grants(cong).await.map_err(err)?.is_empty() {
            return Err("a removed member still has access".into());
        }

        // The owner deletes it (moving it to a device): nobody else keeps anything.
        Db::destroy(DbConfig::Surreal { session: owner.clone(), congregation: cong.to_hex() }).await.map_err(err)?;
        let admins_db = Db::open(DbConfig::Surreal { session: admin.clone(), congregation: cong.to_hex() }).await.map_err(err)?;
        if !admin.memberships().await.map_err(err)?.is_empty() || !admins_db.changes(0).await.map_err(err)?.is_empty() || admins_db.read_header().await.map_err(err)?.is_some() || !admins_db.keyring().await.map_err(err)?.is_empty() {
            return Err("an administrator still has access after the delete".into());
        }
        if join(&server, &names[3], "k3", &later.proof()).await.is_ok() {
            return Err("an open invite still worked after the delete".into());
        }
        Ok(())
    })
}
