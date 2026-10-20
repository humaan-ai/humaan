#[path = "../../../tests/support/recording.rs"]
mod support;

use humaan_engine::{
    self as engine,
    annotations::{self, Assistance, Outcome, ReviewInput, ReviewState},
    learning_sets::{self, CreateSet, Selection},
};
use std::fs;
fn review() -> ReviewInput {
    ReviewInput {
        outcome: Outcome::Failure,
        assistance: Assistance::Yes,
        notes: "Slip with human recovery".into(),
        reviewer: "Operator".into(),
        review_state: ReviewState::Reviewed,
    }
}
fn input(id: &str, revision: u64) -> CreateSet {
    CreateSet {
        name: "Grasp recovery".into(),
        learning_goal: "Reduce repeated slips".into(),
        curator: "Curator".into(),
        selections: vec![Selection {
            experience_id: id.into(),
            review_revision: revision,
            reason: "Investigate recovery after slipping".into(),
        }],
    }
}
fn setup() -> (tempfile::TempDir, String) {
    let temp = tempfile::tempdir().unwrap();
    engine::project::init(temp.path(), "Robot task").unwrap();
    let source = support::recording(temp.path());
    let manifest = engine::import(&temp.path().join(".humaan/recordings"), &source).unwrap();
    (temp, manifest.id)
}
#[test]
fn selection_preserves_review_and_recording_provenance_after_later_edits() {
    let (temp, id) = setup();
    let store = temp.path().join(".humaan/recordings");
    annotations::save(&store, &id, 0, review()).unwrap();
    let set = learning_sets::create(temp.path(), input(&id, 1)).unwrap();
    assert_eq!(set.members[0].review.review.outcome, Outcome::Failure);
    assert_eq!(set.members[0].manifest.id, id);
    assert_eq!(set.members[0].manifest_sha256.len(), 64);
    let mut changed = review();
    changed.outcome = Outcome::Success;
    annotations::save(&store, &id, 1, changed).unwrap();
    let saved = learning_sets::list(temp.path()).unwrap();
    assert_eq!(saved.len(), 1);
    assert_eq!(saved[0].members[0].review.revision, 1);
    assert_eq!(saved[0].members[0].review.review.outcome, Outcome::Failure);
    let exported = learning_sets::export(temp.path(), &set.id, temp.path()).unwrap();
    let portable: learning_sets::LearningSet =
        serde_json::from_slice(&fs::read(exported).unwrap()).unwrap();
    assert_eq!(portable.project_id, set.project_id);
    assert_eq!(
        portable.members[0].manifest.artifacts[0].sha256,
        set.members[0].manifest.artifacts[0].sha256
    );
    assert!(learning_sets::export(temp.path(), &set.id, temp.path()).is_err());
}
#[test]
fn invalid_selections_publish_nothing() {
    let (temp, id) = setup();
    let store = temp.path().join(".humaan/recordings");
    assert!(learning_sets::create(temp.path(), input(&id, 0)).is_err());
    let mut unreviewed = review();
    unreviewed.review_state = ReviewState::Unreviewed;
    annotations::save(&store, &id, 0, unreviewed).unwrap();
    assert!(learning_sets::create(temp.path(), input(&id, 1)).is_err());
    annotations::save(&store, &id, 1, review()).unwrap();
    assert!(learning_sets::create(temp.path(), input(&id, 1)).is_err());
    let mut duplicate = input(&id, 2);
    duplicate
        .selections
        .push(input(&id, 2).selections.remove(0));
    assert!(learning_sets::create(temp.path(), duplicate).is_err());
    let mut no_reason = input(&id, 2);
    no_reason.selections[0].reason.clear();
    assert!(learning_sets::create(temp.path(), no_reason).is_err());
    let mut empty = input(&id, 2);
    empty.selections.clear();
    assert!(learning_sets::create(temp.path(), empty).is_err());
    assert!(learning_sets::create(temp.path(), input("../escape", 2)).is_err());
    fs::write(store.join(&id).join("state.jsonl"), "corrupted").unwrap();
    assert!(learning_sets::create(temp.path(), input(&id, 2)).is_err());
    assert!(learning_sets::list(temp.path()).unwrap().is_empty());
}

#[test]
fn bundle_contains_selected_originals_and_preserves_pinned_review() {
    let (temp, id) = setup();
    let store = temp.path().join(".humaan/recordings");
    annotations::save(&store, &id, 0, review()).unwrap();
    let set = learning_sets::create(temp.path(), input(&id, 1)).unwrap();
    let mut updated = review();
    updated.outcome = Outcome::Success;
    annotations::save(&store, &id, 1, updated).unwrap();
    let bundle = learning_sets::export_files(temp.path(), &set.id, temp.path()).unwrap();
    let copied = bundle.join("recordings").join(&id);
    engine::inspect(&copied).unwrap();
    for file in ["manifest.json", "state.jsonl", "annotations.json"] {
        assert_eq!(
            fs::read(copied.join(file)).unwrap(),
            fs::read(store.join(&id).join(file)).unwrap()
        );
    }
    let snapshot: learning_sets::LearningSet =
        serde_json::from_slice(&fs::read(bundle.join("learning-set.json")).unwrap()).unwrap();
    assert_eq!(snapshot.members[0].review.review.outcome, Outcome::Failure);
    assert_eq!(annotations::read(&copied, &id).unwrap().revisions.len(), 2);
    assert_eq!(fs::read_dir(bundle.join("recordings")).unwrap().count(), 1);
    assert!(learning_sets::export_files(temp.path(), &set.id, temp.path()).is_err());
    assert!(copied.join("state.jsonl").is_file());
}

#[test]
fn failed_bundle_cleans_up_earlier_copies_and_rejects_changed_manifests() {
    let (temp, id) = setup();
    let store = temp.path().join(".humaan/recordings");
    annotations::save(&store, &id, 0, review()).unwrap();
    let second_source = temp.path().join("second-source");
    fs::create_dir(&second_source).unwrap();
    let mut manifest = engine::inspect_experience(&store, &id).unwrap();
    manifest.id = "second".into();
    fs::write(
        second_source.join("manifest.json"),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
    fs::copy(
        store.join(&id).join("state.jsonl"),
        second_source.join("state.jsonl"),
    )
    .unwrap();
    engine::import(&store, &second_source).unwrap();
    annotations::save(&store, "second", 0, review()).unwrap();
    let mut selection = input(&id, 1);
    selection
        .selections
        .push(input("second", 1).selections.remove(0));
    let set = learning_sets::create(temp.path(), selection).unwrap();
    let output = temp.path().join("output");
    fs::create_dir(&output).unwrap();
    fs::write(store.join("second/state.jsonl"), "broken").unwrap();
    assert!(learning_sets::export_files(temp.path(), &set.id, &output).is_err());
    assert_eq!(fs::read_dir(&output).unwrap().count(), 0);
    fs::copy(
        second_source.join("state.jsonl"),
        store.join("second/state.jsonl"),
    )
    .unwrap();
    manifest.task = "Changed task".into();
    fs::write(
        store.join("second/manifest.json"),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
    assert!(learning_sets::export_files(temp.path(), &set.id, &output).is_err());
    assert_eq!(fs::read_dir(&output).unwrap().count(), 0);
    fs::copy(
        second_source.join("manifest.json"),
        store.join("second/manifest.json"),
    )
    .unwrap();
    let bundle = learning_sets::export_files(temp.path(), &set.id, &output).unwrap();
    assert_eq!(fs::read_dir(bundle.join("recordings")).unwrap().count(), 2);
}
