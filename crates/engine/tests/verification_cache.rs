use humaan_engine::{self as engine, verification_cache};
use std::fs;

fn recording(store: &std::path::Path) {
    let root = store.join("record-1");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("data.bin"), b"abc").unwrap();
    fs::write(root.join("manifest.json"), r#"{"version":1,"id":"record-1","robot":"test","task":"pick","artifacts":[{"path":"data.bin","format":"application/octet-stream","bytes":3,"sha256":"ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"}]}"#).unwrap();
}

#[test]
fn persists_pass_failure_and_detects_changes_without_reverification() {
    let temp = tempfile::tempdir().unwrap();
    let store = temp.path().join("recordings");
    recording(&store);
    assert!(verification_cache::read(&store).unwrap().is_empty());
    assert!(engine::verify(&store).unwrap()[0].valid);
    assert!(!store.join(".verification.json").exists()); // CLI stays read-only.
    let passed = verification_cache::verify_and_save(&store).unwrap();
    assert_eq!(passed[0].status, "verified");
    assert!(passed[0].checked_at_unix_ms > 0);
    let reopened = verification_cache::read(&store).unwrap();
    assert_eq!(
        reopened["record-1"].checked_at_unix_ms,
        passed[0].checked_at_unix_ms
    );
    assert_eq!(reopened["record-1"].status, "verified");
    fs::write(store.join("record-1/data.bin"), "changed data").unwrap();
    assert_eq!(
        verification_cache::read(&store).unwrap()["record-1"].status,
        "needs_recheck"
    );
    let failed = verification_cache::verify_and_save(&store).unwrap();
    assert_eq!(failed.len(), 1); // Cache and temporary files are not recordings.
    assert_eq!(failed[0].status, "failed");
    assert!(failed[0].error.is_some());
    assert_eq!(
        verification_cache::read(&store).unwrap()["record-1"].status,
        "failed"
    );
    fs::write(store.join("record-1/data.bin"), "abc").unwrap();
    assert_eq!(
        verification_cache::read(&store).unwrap()["record-1"].status,
        "needs_recheck"
    );
    assert_eq!(
        verification_cache::verify_and_save(&store).unwrap()[0].status,
        "verified"
    );
    fs::write(store.join("record-1/manifest.json"), "broken manifest").unwrap();
    assert_eq!(
        verification_cache::read(&store).unwrap()["record-1"].status,
        "needs_recheck"
    );
    assert_eq!(
        verification_cache::verify_and_save(&store).unwrap()[0].status,
        "failed"
    );
    assert!(
        verification_cache::read(&temp.path().join("other-project"))
            .unwrap()
            .is_empty()
    );
}

#[cfg(unix)]
#[test]
fn refuses_symlinked_history() {
    let temp = tempfile::tempdir().unwrap();
    let store = temp.path().join("recordings");
    recording(&store);
    let external = temp.path().join("external.json");
    fs::write(&external, "untouched").unwrap();
    std::os::unix::fs::symlink(&external, store.join(".verification.json")).unwrap();
    assert!(verification_cache::read(&store).is_err());
    assert!(verification_cache::verify_and_save(&store).is_err());
    assert_eq!(fs::read_to_string(external).unwrap(), "untouched");
}
