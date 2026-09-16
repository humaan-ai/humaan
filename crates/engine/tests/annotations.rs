#[path = "../../../tests/support/recording.rs"]
mod support;

use humaan_engine::{
    self as engine,
    annotations::{self, Assistance, Outcome, ReviewInput, ReviewState},
};
use std::fs;
fn review() -> ReviewInput {
    ReviewInput {
        outcome: Outcome::Success,
        assistance: Assistance::Yes,
        notes: "Recovered after a slip".into(),
        reviewer: "Operator".into(),
        review_state: ReviewState::Reviewed,
    }
}
#[test]
fn reviews_preserve_recordings_history_and_export_round_trip() {
    let temp = tempfile::tempdir().unwrap();
    let source = support::recording(temp.path());
    let store = temp.path().join("store");
    let manifest = engine::import(&store, &source).unwrap();
    let dir = store.join(&manifest.id);
    let original = fs::read(dir.join("manifest.json")).unwrap();
    assert!(
        annotations::read(&dir, &manifest.id)
            .unwrap()
            .revisions
            .is_empty()
    );
    let first = annotations::save(&store, &manifest.id, 0, review()).unwrap();
    assert_eq!(first.revisions[0].review.assistance, Assistance::Yes);
    assert!(annotations::save(&store, &manifest.id, 0, review()).is_err());
    let mut correction = review();
    correction.outcome = Outcome::Unknown;
    annotations::save(&store, &manifest.id, 1, correction).unwrap();
    assert_eq!(fs::read(dir.join("manifest.json")).unwrap(), original);
    let destination = temp.path().join("export");
    engine::export(&store, &manifest.id, &destination).unwrap();
    let imported = temp.path().join("reimport");
    engine::import(&imported, &destination).unwrap();
    let history = annotations::read(&imported.join(&manifest.id), &manifest.id).unwrap();
    assert_eq!(history.revisions.len(), 2);
    assert_eq!(history.revisions[0].review.outcome, Outcome::Success);
    assert_eq!(history.revisions[1].review.outcome, Outcome::Unknown);
    assert_eq!(
        history.revisions[1].review.review_state,
        ReviewState::Reviewed
    );
    assert!(annotations::save(&store, "../escape", 0, review()).is_err());
    let mut invalid = review();
    invalid.reviewer = " ".into();
    assert!(annotations::save(&store, &manifest.id, 2, invalid).is_err());
    fs::write(dir.join("annotations.json"), "broken").unwrap();
    assert!(annotations::save(&store, &manifest.id, 2, review()).is_err());
    assert!(engine::export(&store, &manifest.id, &temp.path().join("bad-export")).is_err());
}
#[cfg(unix)]
#[test]
fn rejects_symlinked_reviews() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path().join("target");
    fs::write(&target, "{}").unwrap();
    std::os::unix::fs::symlink(&target, temp.path().join("annotations.json")).unwrap();
    assert!(annotations::read(temp.path(), "test").is_err());
}

#[test]
fn stale_lock_file_does_not_block_saves_but_active_writer_does() {
    let temp = tempfile::tempdir().unwrap();
    let source = support::recording(temp.path());
    let store = temp.path().join("store");
    let manifest = engine::import(&store, &source).unwrap();
    let directory = store.join(&manifest.id);
    let lock_path = directory.join(".annotation-lock");
    fs::write(&lock_path, "").unwrap();
    annotations::save(&store, &manifest.id, 0, review()).unwrap();
    let guard = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&lock_path)
        .unwrap();
    guard.try_lock().unwrap();
    assert!(annotations::save(&store, &manifest.id, 1, review()).is_err());
    assert_eq!(
        annotations::read(&directory, &manifest.id)
            .unwrap()
            .revisions
            .len(),
        1
    );
    drop(guard);
    annotations::save(&store, &manifest.id, 1, review()).unwrap();
    assert!(annotations::save(&store, &manifest.id, 1, review()).is_err());
    assert_eq!(
        annotations::read(&directory, &manifest.id)
            .unwrap()
            .revisions
            .len(),
        2
    );
}

#[test]
fn lock_holder_process() {
    let Some(path) = std::env::var_os("HUMAAN_TEST_ANNOTATION_LOCK") else {
        return;
    };
    let guard = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .unwrap();
    guard.try_lock().unwrap();
    // Exit without running Rust destructors, as with an abruptly terminated app.
    std::process::exit(0);
}

#[test]
fn process_exit_releases_review_lock_without_cleanup() {
    let temp = tempfile::tempdir().unwrap();
    let source = support::recording(temp.path());
    let store = temp.path().join("store");
    let manifest = engine::import(&store, &source).unwrap();
    let lock_path = store.join(&manifest.id).join(".annotation-lock");
    fs::write(&lock_path, "").unwrap();
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "lock_holder_process"])
        .env("HUMAAN_TEST_ANNOTATION_LOCK", &lock_path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(lock_path.is_file());
    annotations::save(&store, &manifest.id, 0, review()).unwrap();
}
