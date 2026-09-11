use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

fn cli(store: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_humaan"))
        .arg("--store")
        .arg(store)
        .args(args)
        .output()
        .unwrap()
}
fn fixture() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/recording")
}

#[test]
fn offline_round_trip_and_duplicate_protection() {
    let temp = tempfile::tempdir().unwrap();
    let store = temp.path().join("store");
    assert!(cli(&store, &["list"]).status.success());
    assert!(!store.exists());
    let imported = cli(&store, &["import", fixture().to_str().unwrap(), "--json"]);
    assert!(
        imported.status.success(),
        "{}",
        String::from_utf8_lossy(&imported.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&imported.stdout).unwrap();
    assert!(
        imported.stderr.is_empty(),
        "JSON import must not emit progress output"
    );
    assert_eq!(json[0]["id"], "example-001");
    assert!(
        !cli(&store, &["import", fixture().to_str().unwrap()])
            .status
            .success()
    );
    assert!(cli(&store, &["inspect", "example-001"]).status.success());
    let listed = cli(&store, &["list", "--json"]);
    assert!(listed.status.success());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&listed.stdout).unwrap(),
        json
    );
    let exported = temp.path().join("export");
    assert!(
        cli(
            &store,
            &["export", "example-001", exported.to_str().unwrap()]
        )
        .status
        .success()
    );
    for file in ["manifest.json", "state.jsonl"] {
        assert_eq!(
            fs::read(fixture().join(file)).unwrap(),
            fs::read(exported.join(file)).unwrap()
        );
    }
    assert!(
        !cli(
            &store,
            &["export", "example-001", exported.to_str().unwrap()]
        )
        .status
        .success()
    );
    fs::write(store.join("example-001/state.jsonl"), "corrupt").unwrap();
    assert!(!cli(&store, &["inspect", "example-001"]).status.success());
    let rejected = temp.path().join("rejected");
    assert!(
        !cli(
            &store,
            &["export", "example-001", rejected.to_str().unwrap()]
        )
        .status
        .success()
    );
    assert!(!rejected.exists());
}

#[test]
fn invalid_recordings_never_enter_store() {
    let temp = tempfile::tempdir().unwrap();
    let store = temp.path().join("store");
    let source = temp.path().join("source");
    fs::create_dir(&source).unwrap();
    let original: serde_json::Value =
        serde_json::from_slice(&fs::read(fixture().join("manifest.json")).unwrap()).unwrap();
    for case in 0..5 {
        let mut manifest = original.clone();
        match case {
            0 => manifest["version"] = 99.into(),
            1 => manifest["id"] = "../escape".into(),
            2 => manifest["artifacts"][0]["path"] = "../outside".into(),
            3 => manifest["artifacts"][0]["sha256"] = "bad".into(),
            _ => manifest["artifacts"][0]["path"] = "missing".into(),
        }
        fs::copy(fixture().join("state.jsonl"), source.join("state.jsonl")).unwrap();
        fs::write(
            source.join("manifest.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        assert!(
            !cli(&store, &["import", source.to_str().unwrap()])
                .status
                .success()
        );
        assert!(!store.exists());
    }
    assert!(!cli(&store, &["inspect", "../escape"]).status.success());
}

#[cfg(unix)]
#[test]
fn rejects_symlink_artifacts() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    fs::create_dir(&source).unwrap();
    fs::copy(
        fixture().join("manifest.json"),
        source.join("manifest.json"),
    )
    .unwrap();
    std::os::unix::fs::symlink(fixture().join("state.jsonl"), source.join("state.jsonl")).unwrap();
    assert!(
        !cli(
            &temp.path().join("store"),
            &["import", source.to_str().unwrap()]
        )
        .status
        .success()
    );
}

fn in_project(directory: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_humaan"))
        .current_dir(directory)
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn project_identity_discovery_and_isolation() {
    let temp = tempfile::tempdir().unwrap();
    let first = temp.path().join("first");
    let second = temp.path().join("second");
    fs::create_dir(&first).unwrap();
    fs::create_dir(&second).unwrap();
    let initialized = in_project(&first, &["project", "init", "Robot Lab", "--json"]);
    assert!(
        initialized.status.success(),
        "{}",
        String::from_utf8_lossy(&initialized.stderr)
    );
    let metadata: serde_json::Value = serde_json::from_slice(&initialized.stdout).unwrap();
    assert_eq!(metadata["project"]["name"], "Robot Lab");
    assert!(!metadata["project"]["id"].as_str().unwrap().is_empty());
    assert!(metadata["project"].get("workspace").is_none());
    let saved = fs::read(first.join(".humaan/project.json")).unwrap();
    assert!(
        !in_project(&first, &["project", "init", "Replacement"])
            .status
            .success()
    );
    assert_eq!(saved, fs::read(first.join(".humaan/project.json")).unwrap());
    let nested = first.join("nested");
    fs::create_dir(&nested).unwrap();
    let shown = in_project(&nested, &["project", "show", "--json"]);
    assert!(shown.status.success());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&shown.stdout).unwrap(),
        metadata
    );
    assert!(
        in_project(&nested, &["import", fixture().to_str().unwrap()])
            .status
            .success()
    );
    assert!(
        first
            .join(".humaan/recordings/example-001/manifest.json")
            .is_file()
    );
    assert!(!nested.join(".humaan").exists());
    assert!(
        in_project(&first, &["inspect", "example-001"])
            .status
            .success()
    );
    let other = in_project(&second, &["project", "init", "Robot Lab", "--json"]);
    assert!(other.status.success());
    let other: serde_json::Value = serde_json::from_slice(&other.stdout).unwrap();
    assert_ne!(metadata["project"]["id"], other["project"]["id"]);
    let listed = in_project(&second, &["list", "--json"]);
    assert!(listed.status.success());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&listed.stdout).unwrap(),
        serde_json::json!([])
    );
    fs::write(first.join(".humaan/project.json"), "broken").unwrap();
    assert!(!in_project(&nested, &["list"]).status.success());
    assert!(!in_project(&nested, &["project", "show"]).status.success());
}

#[test]
fn initializing_preserves_existing_recordings_and_rejects_empty_names() {
    let temp = tempfile::tempdir().unwrap();
    assert!(
        !in_project(temp.path(), &["project", "init", " "])
            .status
            .success()
    );
    assert!(!temp.path().join(".humaan").exists());
    assert!(
        !in_project(temp.path(), &["project", "show"])
            .status
            .success()
    );
    assert!(
        in_project(temp.path(), &["import", fixture().to_str().unwrap()])
            .status
            .success()
    );
    assert!(
        in_project(temp.path(), &["project", "init", "Existing recordings"])
            .status
            .success()
    );
    assert!(
        in_project(temp.path(), &["inspect", "example-001"])
            .status
            .success()
    );
}

#[test]
fn verification_reports_all_entries_without_modifying_recordings() {
    let temp = tempfile::tempdir().unwrap();
    let store = temp.path().join("store");
    let empty = cli(&store, &["verify", "--json"]);
    assert!(empty.status.success());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&empty.stdout).unwrap(),
        serde_json::json!([])
    );
    assert!(!store.exists());
    assert!(
        cli(&store, &["import", fixture().to_str().unwrap()])
            .status
            .success()
    );
    let healthy = cli(&store, &["verify"]);
    assert!(healthy.status.success());
    assert!(String::from_utf8_lossy(&healthy.stdout).contains("OK   example-001"));
    let details = cli(&store, &["inspect", "example-001"]);
    assert!(details.status.success());
    assert!(
        String::from_utf8_lossy(&details.stdout)
            .contains("state.jsonl  format=application/x-ndjson")
    );
    assert!(String::from_utf8_lossy(&details.stdout).contains("sha256="));
    let malformed = store.join("aaa-broken");
    fs::create_dir(&malformed).unwrap();
    fs::write(malformed.join("manifest.json"), "broken").unwrap();
    fs::write(store.join("zzz-unexpected"), "keep").unwrap();
    fs::create_dir(store.join(".humaan-staging")).unwrap();
    let report = cli(&store, &["verify", "--json"]);
    assert_eq!(report.status.code(), Some(1));
    let report: serde_json::Value = serde_json::from_slice(&report.stdout).unwrap();
    assert_eq!(report.as_array().unwrap().len(), 3);
    assert_eq!(report[0]["id"], "aaa-broken");
    assert_eq!(report[0]["valid"], false);
    assert!(report[0]["error"].is_string());
    assert_eq!(report[1]["id"], "example-001");
    assert_eq!(report[1]["valid"], true);
    assert_eq!(report[2]["valid"], false);
    assert_eq!(
        fs::read(malformed.join("manifest.json")).unwrap(),
        b"broken"
    );
    assert_eq!(fs::read(store.join("zzz-unexpected")).unwrap(), b"keep");
    fs::write(store.join("example-001/state.jsonl"), "damaged").unwrap();
    let report = cli(&store, &["verify", "--json"]);
    assert_eq!(report.status.code(), Some(1));
    let report: serde_json::Value = serde_json::from_slice(&report.stdout).unwrap();
    assert_eq!(report[1]["valid"], false);
    assert!(
        report[1]["error"]
            .as_str()
            .unwrap()
            .contains("integrity check failed")
    );
}
