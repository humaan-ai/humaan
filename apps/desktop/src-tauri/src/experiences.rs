use humaan_engine as engine;
use serde::Serialize;
use std::path::{Path, PathBuf};
use tauri::Manager;

#[derive(Serialize)]
pub struct LocalProject {
    root: PathBuf,
    project: engine::project::Project,
}

async fn blocking<T: Send + 'static>(
    operation: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(operation)
        .await
        .map_err(|error| error.to_string())?
}

fn project_store(root: &str) -> Result<PathBuf, String> {
    let (root, _) = engine::project::discover(Path::new(root))
        .map_err(|e| e.to_string())?
        .ok_or("No Humaan project in this folder. Open or create a project first.")?;
    Ok(root.join(".humaan/recordings"))
}

#[tauri::command]
pub async fn experience_project(
    path: String,
    name: Option<String>,
) -> Result<LocalProject, String> {
    blocking(move || {
        let path = path.trim();
        if path.is_empty() { return Err("Enter a project location.".into()); }
        let mut location = if path == "~" || path.starts_with("~/") || path.starts_with("~\\") {
            let home = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
                .ok_or("Could not find your home folder. Enter an absolute project location.")?;
            PathBuf::from(home).join(if path == "~" { "" } else { &path[2..] })
        } else {
            PathBuf::from(path)
        };
        if !location.is_absolute() { return Err("Enter an absolute project location or a path starting with ~/ .".into()); }
        if let Some(ref name) = name {
            let folder_name = name.trim();
            let stem = folder_name.split('.').next().unwrap_or("").to_ascii_uppercase();
            let reserved = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
                || (stem.len() == 4 && (stem.starts_with("COM") || stem.starts_with("LPT"))
                    && matches!(stem.as_bytes()[3], b'1'..=b'9'));
            if folder_name.is_empty() || folder_name.starts_with('.') || folder_name.ends_with('.')
                || folder_name.chars().any(|c| c.is_control() || "<>:\"/\\|?*".contains(c)) || reserved {
                return Err("Enter a valid project folder name without path separators, reserved characters, or a leading or trailing dot.".into());
            }
            std::fs::create_dir_all(&location).map_err(|e| format!("Could not create project location: {e}"))?;
            location = location.join(folder_name);
            std::fs::create_dir(&location).map_err(|e| {
                if e.kind() == std::io::ErrorKind::AlreadyExists {
                    "A folder with this project name already exists. Choose another name or open the existing project.".to_string()
                } else { format!("Could not create project folder: {e}") }
            })?;
        }
        let root = location.canonicalize().map_err(|e| e.to_string())?;
        if !root.is_dir() { return Err("Select a project directory.".into()); }
        if let Some(name) = name {
            let project = engine::project::init(&root, &name).map_err(|e| e.to_string())?;
            Ok(LocalProject { root, project })
        } else {
            let (root, project) = engine::project::discover(&root).map_err(|e| e.to_string())?
                .ok_or("No Humaan project found. Choose the folder used by `humaan project init`, or create a project.")?;
            Ok(LocalProject { root, project })
        }
    }).await
}

#[derive(Serialize)]
pub struct RecordSummary {
    #[serde(flatten)]
    manifest: engine::Manifest,
    duration_s: Option<f64>,
    verification: Option<engine::verification_cache::Report>,
    source_error: Option<String>,
}

fn record_summary(store: &Path, manifest: engine::Manifest) -> RecordSummary {
    // Read only the small source metadata file, never media, hashes, or a replay.
    let (duration_s, source_error) = match engine::lerobot_details(store, &manifest) {
        Ok(source) => (
            source
                .and_then(|source| source["duration_s"].as_f64())
                .filter(|seconds| seconds.is_finite() && *seconds >= 0.0),
            None,
        ),
        Err(error) => (None, Some(error.to_string())),
    };
    RecordSummary {
        manifest,
        duration_s,
        verification: None,
        source_error,
    }
}

#[derive(Serialize)]
pub struct Catalog {
    records: Vec<RecordSummary>,
    warning: Option<String>,
}

#[tauri::command]
pub async fn experience_list(root: String) -> Result<Catalog, String> {
    blocking(move || {
        let store = project_store(&root)?;
        let (history, warning) = match engine::verification_cache::read(&store) {
            Ok(history) => (history, None),
            Err(error) => (Default::default(), Some(format!(
                "Saved verification history could not be read: {error}. Records are shown as not checked. Run Verify all to rebuild the history."
            ))),
        };
        let mut records = Vec::new();
        if !store.exists() {
            return Ok(Catalog { records, warning });
        }
        for entry in std::fs::read_dir(&store).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let id = entry.file_name().to_string_lossy().into_owned();
            if id.starts_with('.') {
                continue;
            }
            let mut summary = match engine::experience_metadata(&store, &id) {
                Ok(manifest) => record_summary(&store, manifest),
                Err(error) => RecordSummary {
                    manifest: engine::Manifest {
                        version: 1,
                        id: id.clone(),
                        task: "Unreadable recording".into(),
                        robot: "Unknown".into(),
                        artifacts: vec![],
                    },
                    duration_s: None,
                    verification: None,
                    source_error: Some(error.to_string()),
                },
            };
            summary.verification = history.get(&id).cloned();
            records.push(summary);
        }
        records.sort_by(|a, b| a.manifest.id.cmp(&b.manifest.id));
        Ok(Catalog { records, warning })
    })
    .await
}

#[tauri::command]
pub async fn experience_import(
    root: String,
    source: String,
) -> Result<Vec<engine::Manifest>, String> {
    blocking(move || {
        engine::import_recordings(&project_store(&root)?, Path::new(&source))
            .map_err(|e| e.to_string())
    })
    .await
}

#[derive(Serialize)]
pub struct Inspection {
    manifest: engine::Manifest,
    source: Option<serde_json::Value>,
    source_error: Option<String>,
    annotations: engine::annotations::Annotations,
}

#[tauri::command]
pub async fn experience_inspect(root: String, id: String) -> Result<Inspection, String> {
    blocking(move || {
        let store = project_store(&root)?;
        let manifest = engine::experience_metadata(&store, &id).map_err(|e| e.to_string())?;
        let (source, source_error) = match engine::lerobot_details(&store, &manifest) {
            Ok(source) => (source, None),
            Err(error) => (None, Some(error.to_string())),
        };
        let annotations =
            engine::annotations::read(&store.join(&id), &id).map_err(|e| e.to_string())?;
        Ok(Inspection {
            manifest,
            source,
            source_error,
            annotations,
        })
    })
    .await
}

#[tauri::command]
pub async fn experience_verify(
    root: String,
) -> Result<Vec<engine::verification_cache::Report>, String> {
    blocking(move || {
        engine::verification_cache::verify_and_save(&project_store(&root)?)
            .map_err(|e| e.to_string())
    })
    .await
}

#[tauri::command]
pub async fn experience_export(root: String, id: String, parent: String) -> Result<String, String> {
    blocking(move || {
        // Engine validates the ID before any writes. The selected parent contains
        // a new directory named after the experience; existing exports are preserved.
        let destination = Path::new(&parent).join(&id);
        engine::export(&project_store(&root)?, &id, &destination).map_err(|e| e.to_string())?;
        Ok(destination.display().to_string())
    })
    .await
}

#[cfg(test)]
#[path = "../../../../tests/support/recording.rs"]
mod test_support;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn record_summary_reads_duration_without_verifying_artifacts() {
        let temp = tempfile::tempdir().unwrap();
        let recording = temp.path().join("record-1");
        std::fs::create_dir(&recording).unwrap();
        let manifest = || engine::Manifest {
            version: 1,
            id: "record-1".into(),
            robot: "test".into(),
            task: "pick".into(),
            artifacts: vec![engine::Artifact {
                path: "source.json".into(),
                format: "application/vnd.humaan.lerobot-experience+json".into(),
                bytes: 0,
                sha256: "deliberately-not-verified".into(),
            }],
        };
        std::fs::write(recording.join("source.json"), r#"{"duration_s":12.5}"#).unwrap();
        let summary = record_summary(temp.path(), manifest());
        assert_eq!(summary.duration_s, Some(12.5));
        assert!(summary.source_error.is_none());
        let json = serde_json::to_value(summary).unwrap();
        assert_eq!(json["id"], "record-1");
        assert_eq!(json["duration_s"], 12.5);
        std::fs::write(recording.join("source.json"), "invalid json").unwrap();
        let summary = record_summary(temp.path(), manifest());
        assert!(summary.duration_s.is_none());
        assert!(summary.source_error.is_some());
        std::fs::write(recording.join("source.json"), r#"{"duration_s":-1}"#).unwrap();
        assert!(record_summary(temp.path(), manifest()).duration_s.is_none());
        let mut native = manifest();
        native.artifacts.clear();
        let summary = record_summary(temp.path(), native);
        assert!(summary.duration_s.is_none());
        assert!(summary.source_error.is_none());
    }

    #[test]
    fn corrupt_verification_history_keeps_catalog_browsable_and_can_be_rebuilt() {
        tauri::async_runtime::block_on(async {
            let temp = tempfile::tempdir().unwrap();
            engine::project::init(temp.path(), "Cache recovery").unwrap();
            let source = test_support::recording(temp.path());
            let root = temp.path().to_str().unwrap().to_string();
            experience_import(root.clone(), source.display().to_string())
                .await
                .unwrap();
            let cache = project_store(&root).unwrap().join(".verification.json");
            for damaged in ["broken", r#"{"version":99,"reports":{}}"#] {
                std::fs::write(&cache, damaged).unwrap();
                let catalog = experience_list(root.clone()).await.unwrap();
                assert_eq!(catalog.records.len(), 1);
                assert!(catalog.records[0].verification.is_none());
                assert!(catalog.warning.unwrap().contains("Verify all"));
                assert_eq!(std::fs::read_to_string(&cache).unwrap(), damaged);
                assert!(experience_inspect(root.clone(), "example-001".into())
                    .await
                    .is_ok());
                assert!(experience_verify(root.clone()).await.unwrap()[0].valid);
                let restored = experience_list(root.clone()).await.unwrap();
                assert!(restored.warning.is_none());
                assert_eq!(
                    restored.records[0].verification.as_ref().unwrap().status,
                    "verified"
                );
            }
        });
    }

    #[test]
    fn desktop_creation_uses_named_folder_in_selected_location() {
        tauri::async_runtime::block_on(async {
            let temp = tempfile::tempdir().unwrap();
            let parent = temp.path().join("chosen location");
            let location = parent.to_string_lossy().into_owned();
            let project = experience_project(location.clone(), Some(" Pick and place ".into())).await.unwrap();
            assert_eq!(project.root, parent.join("Pick and place").canonicalize().unwrap());
            assert_eq!(project.project.name, "Pick and place");
            assert!(project.root.join(".humaan/project.json").is_file());
            assert!(!parent.join(".humaan").exists());
            let reopened = experience_project(project.root.to_string_lossy().into_owned(), None).await.unwrap();
            assert_eq!(reopened.project.id, project.project.id);
            assert!(experience_project(location.clone(), Some("Pick and place".into())).await.is_err());
            std::fs::create_dir(parent.join("Existing folder")).unwrap();
            assert!(experience_project(location.clone(), Some("Existing folder".into())).await.is_err());
            assert!(!parent.join("Existing folder/.humaan").exists());
            for name in ["", "..", "../escape", "a/b", "a\\b", ".hidden", "CON", "COM1", "name."] {
                assert!(experience_project(location.clone(), Some(name.into())).await.is_err(), "accepted {name}");
            }
        });
    }

    #[test]
    fn requires_existing_project_and_uses_cli_store() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        assert!(project_store(root.to_str().unwrap()).is_err());
        engine::project::init(&root, "Desktop test").unwrap();
        assert_eq!(
            project_store(root.to_str().unwrap()).unwrap(),
            root.canonicalize().unwrap().join(".humaan/recordings")
        );
    }

    #[test]
    fn desktop_commands_round_trip_cli_recordings_and_report_corruption() {
        tauri::async_runtime::block_on(async {
            let temp = tempfile::tempdir().unwrap();
            let root = temp.path().to_str().unwrap().to_string();
            engine::project::init(temp.path(), "CLI project").unwrap();
            let project = experience_project(root.clone(), None).await.unwrap();
            assert_eq!(project.project.name, "CLI project");
            let source = test_support::recording(temp.path());
            let imported = experience_import(root.clone(), source.display().to_string())
                .await
                .unwrap();
            let id = imported[0].id.clone();
            assert_eq!(
                experience_list(root.clone()).await.unwrap().records.len(),
                1
            );
            assert_eq!(
                experience_inspect(root.clone(), id.clone())
                    .await
                    .unwrap()
                    .manifest
                    .id,
                id
            );
            assert!(
                experience_import(root.clone(), source.display().to_string())
                    .await
                    .is_err()
            );
            let review = engine::annotations::ReviewInput {
                outcome: engine::annotations::Outcome::Success,
                assistance: engine::annotations::Assistance::Yes,
                notes: "Human recovery".into(),
                reviewer: "Desktop reviewer".into(),
                review_state: engine::annotations::ReviewState::Reviewed,
            };
            experience_annotate(root.clone(), id.clone(), 0, review)
                .await
                .unwrap();
            let reviews = experience_reviews(root.clone()).await.unwrap();
            assert_eq!(reviews[&id].revisions.len(), 1);
            assert_eq!(
                reviews[&id].revisions[0].review.assistance,
                engine::annotations::Assistance::Yes
            );
            // A reopened project reads the saved review from disk, then permits
            // a new revision without erasing the original judgment.
            let reopened = experience_project(root.clone(), None).await.unwrap();
            let reopened_root = reopened.root.display().to_string();
            let persisted = experience_reviews(reopened_root.clone()).await.unwrap();
            let mut updated = persisted[&id].revisions[0].review.clone();
            updated.outcome = engine::annotations::Outcome::Unknown;
            updated.notes = "Insufficient evidence after a second look".into();
            let saved = experience_annotate(reopened_root.clone(), id.clone(), 1, updated.clone())
                .await
                .unwrap();
            assert_eq!(saved.revisions.len(), 2);
            assert!(
                experience_annotate(reopened_root.clone(), id.clone(), 1, updated)
                    .await
                    .is_err()
            );
            let persisted = experience_reviews(reopened_root).await.unwrap();
            assert_eq!(persisted[&id].revisions.len(), 2);
            assert_eq!(
                persisted[&id].revisions[0].review.outcome,
                engine::annotations::Outcome::Success
            );
            assert_eq!(
                persisted[&id].revisions[1].review.outcome,
                engine::annotations::Outcome::Unknown
            );
            let set = learning_set_create(
                root.clone(),
                engine::learning_sets::CreateSet {
                    name: "Recovery evidence".into(),
                    learning_goal: "Understand grasp failures".into(),
                    curator: "Operator".into(),
                    selections: vec![engine::learning_sets::Selection {
                        experience_id: id.clone(),
                        review_revision: 2,
                        reason: "Human recovery worth inspecting".into(),
                    }],
                },
            )
            .await
            .unwrap();
            assert_eq!(learning_set_list(root.clone()).await.unwrap().len(), 1);
            let selection_export = learning_set_export(root.clone(), set.id.clone(), root.clone())
                .await
                .unwrap();
            assert!(Path::new(&selection_export).is_file());
            let bundle = learning_set_export_files(root.clone(), set.id, root.clone())
                .await
                .unwrap();
            assert!(Path::new(&bundle).join("learning-set.json").is_file());
            assert!(Path::new(&bundle)
                .join("recordings")
                .join(&id)
                .join("state.jsonl")
                .is_file());
            let destination = temp.path().join("exports");
            std::fs::create_dir(&destination).unwrap();
            let exported =
                experience_export(root.clone(), id.clone(), destination.display().to_string())
                    .await
                    .unwrap();
            assert_eq!(
                std::fs::read(Path::new(&exported).join("state.jsonl")).unwrap(),
                std::fs::read(source.join("state.jsonl")).unwrap()
            );
            assert!(
                experience_export(root.clone(), id.clone(), destination.display().to_string())
                    .await
                    .is_err()
            );
            assert!(experience_inspect(root.clone(), "../escape".into())
                .await
                .is_err());
            std::fs::write(
                project_store(&root).unwrap().join(&id).join("state.jsonl"),
                "broken",
            )
            .unwrap();
            assert_eq!(
                experience_list(root.clone()).await.unwrap().records.len(),
                1
            );
            assert!(!experience_verify(root.clone()).await.unwrap()[0].valid);
            // Browsing still works with damaged or absent payloads, while
            // operations that consume them continue to enforce integrity.
            assert_eq!(
                experience_inspect(root.clone(), id.clone())
                    .await
                    .unwrap()
                    .annotations
                    .revisions
                    .len(),
                2
            );
            std::fs::remove_file(project_store(&root).unwrap().join(&id).join("state.jsonl"))
                .unwrap();
            assert!(experience_inspect(root.clone(), id.clone()).await.is_ok());
            assert!(engine::inspect_experience(&project_store(&root).unwrap(), &id).is_err());
            assert!(experience_export(
                root,
                id,
                temp.path().join("missing-export").display().to_string()
            )
            .await
            .is_err());
        });
    }
}

#[tauri::command]
pub async fn experience_replay(
    root: String,
    id: String,
    viewer_path: Option<String>,
) -> Result<(), String> {
    blocking(move || {
        let viewer = viewer_path.filter(|path| !path.trim().is_empty()).map(PathBuf::from)
            .or_else(|| std::env::var_os("HUMAAN_RERUN_PATH").map(PathBuf::from))
            .unwrap_or_else(|| {
                let cargo_binary = std::env::var_os("HOME").map(PathBuf::from)
                    .map(|home| home.join(".cargo/bin").join(if cfg!(windows) { "rerun.exe" } else { "rerun" }));
                cargo_binary.filter(|path| path.is_file()).unwrap_or_else(|| PathBuf::from("rerun"))
            });
        let version = std::process::Command::new(&viewer).arg("--version").output()
            .map_err(|error| format!("Rerun viewer not available: {error}. Install standalone Rerun 0.37.2 and add it to PATH, or set HUMAAN_RERUN_PATH to its executable before starting Humaan."))?;
        if !version.status.success() || !String::from_utf8_lossy(&version.stdout).split_whitespace().any(|part| part == "0.37.2") {
            return Err("This integration requires the standalone Rerun 0.37.2 viewer. Set HUMAAN_RERUN_PATH to the matching executable.".into());
        }
        let temporary = tempfile::Builder::new().prefix("humaan-rerun-").tempdir().map_err(|e| e.to_string())?;
        let recording = temporary.path().join("experience.rrd");
        engine::replay::save(&project_store(&root)?, &id, &recording).map_err(|e| e.to_string())?;
        let mut child = std::process::Command::new(viewer).arg(&recording)
            .stdin(std::process::Stdio::null()).spawn().map_err(|e| format!("Could not start Rerun: {e}"))?;
        // Retain the temporary recording until the separate viewer has exited.
        std::thread::spawn(move || { let _ = child.wait(); drop(temporary); });
        Ok(())
    }).await
}

#[tauri::command]
pub async fn experience_reviews(
    root: String,
) -> Result<std::collections::BTreeMap<String, engine::annotations::Annotations>, String> {
    blocking(move || {
        let store = project_store(&root)?;
        let mut reviews = std::collections::BTreeMap::new();
        if !store.exists() {
            return Ok(reviews);
        }
        for entry in std::fs::read_dir(&store).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let id = entry.file_name().to_string_lossy().into_owned();
            if id.starts_with('.') || engine::experience_metadata(&store, &id).is_err() {
                continue;
            }
            let annotations =
                engine::annotations::read(&store.join(&id), &id).map_err(|e| e.to_string())?;
            reviews.insert(id, annotations);
        }
        Ok(reviews)
    })
    .await
}

#[tauri::command]
pub async fn experience_annotate(
    root: String,
    id: String,
    expected_revision: u64,
    review: engine::annotations::ReviewInput,
) -> Result<engine::annotations::Annotations, String> {
    blocking(move || {
        engine::annotations::save(&project_store(&root)?, &id, expected_revision, review)
            .map_err(|e| e.to_string())
    })
    .await
}

#[tauri::command]
pub async fn learning_set_list(
    root: String,
) -> Result<Vec<engine::learning_sets::LearningSet>, String> {
    blocking(move || engine::learning_sets::list(Path::new(&root)).map_err(|e| e.to_string())).await
}
#[tauri::command]
pub async fn learning_set_create(
    root: String,
    input: engine::learning_sets::CreateSet,
) -> Result<engine::learning_sets::LearningSet, String> {
    blocking(move || {
        engine::learning_sets::create(Path::new(&root), input).map_err(|e| e.to_string())
    })
    .await
}
#[tauri::command]
pub async fn learning_set_export(
    root: String,
    id: String,
    parent: String,
) -> Result<String, String> {
    blocking(move || {
        engine::learning_sets::export(Path::new(&root), &id, Path::new(&parent))
            .map(|path| path.display().to_string())
            .map_err(|e| e.to_string())
    })
    .await
}

#[tauri::command]
pub async fn learning_set_export_files(
    root: String,
    id: String,
    parent: String,
) -> Result<String, String> {
    blocking(move || {
        engine::learning_sets::export_files(Path::new(&root), &id, Path::new(&parent))
            .map(|path| path.display().to_string())
            .map_err(|e| e.to_string())
    })
    .await
}

#[tauri::command]
pub async fn learning_set_prepare(
    app: tauri::AppHandle,
    root: String,
    id: String,
    parent: String,
    config: engine::dataset_preparation::PreparationConfig,
) -> Result<String, String> {
    let environment = app
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?
        .join("dataset-tools");
    blocking(move || {
        engine::dataset_preparation::prepare(
            Path::new(&root),
            &id,
            Path::new(&parent),
            config,
            &environment,
        )
        .map(|path| path.display().to_string())
        .map_err(|e| e.to_string())
    })
    .await
}

#[tauri::command]
pub async fn dataset_tools_status(
    app: tauri::AppHandle,
) -> Result<engine::dataset_environment::Status, String> {
    let directory = app
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?
        .join("dataset-tools");
    blocking(move || {
        if let Some(python) = std::env::var_os("HUMAAN_DATASET_PYTHON") {
            let exists = Path::new(&python).is_file();
            return Ok(engine::dataset_environment::Status { ready: exists,
                custom: true, installing: false, phase: "Using custom dataset tools".into(), error: (!exists).then(|| "The custom Python executable is missing. Update HUMAAN_DATASET_PYTHON or unset it to use managed tools.".into()) });
        }
        engine::dataset_environment::status(&directory).map_err(|e| e.to_string())
    }).await
}
#[tauri::command]
pub async fn dataset_tools_install(
    app: tauri::AppHandle,
) -> Result<engine::dataset_environment::Status, String> {
    let directory = app
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?
        .join("dataset-tools");
    blocking(move || engine::dataset_environment::install(&directory).map_err(|e| e.to_string()))
        .await
}
