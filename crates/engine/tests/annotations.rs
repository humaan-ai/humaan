use humaan_engine::{
    self as engine,
    annotations::{self, Assistance, Outcome, ReviewInput, ReviewState},
};
use std::{fs, path::Path};
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
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/recording");
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
