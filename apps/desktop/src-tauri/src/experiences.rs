use humaan_engine as engine;
use serde::Serialize;
use std::path::{Path, PathBuf};

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
        let root = Path::new(&path).canonicalize().map_err(|e| e.to_string())?;
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

#[tauri::command]
pub async fn experience_list(root: String) -> Result<Vec<engine::Manifest>, String> {
    blocking(move || engine::list_metadata(&project_store(&root)?).map_err(|e| e.to_string())).await
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
}

#[tauri::command]
pub async fn experience_inspect(root: String, id: String) -> Result<Inspection, String> {
    blocking(move || {
        let store = project_store(&root)?;
        let manifest = engine::inspect_experience(&store, &id).map_err(|e| e.to_string())?;
        let source = engine::lerobot_details(&store, &manifest).map_err(|e| e.to_string())?;
        Ok(Inspection { manifest, source })
    })
    .await
}

#[tauri::command]
pub async fn experience_verify(root: String) -> Result<Vec<engine::Verification>, String> {
    blocking(move || engine::verify(&project_store(&root)?).map_err(|e| e.to_string())).await
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
mod tests {
    use super::*;
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
            let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../examples/recording");
            let imported = experience_import(root.clone(), source.display().to_string())
                .await
                .unwrap();
            let id = imported[0].id.clone();
            assert_eq!(experience_list(root.clone()).await.unwrap().len(), 1);
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
            assert_eq!(experience_list(root.clone()).await.unwrap().len(), 1);
            assert!(!experience_verify(root.clone()).await.unwrap()[0].valid);
            assert!(experience_inspect(root, id).await.is_err());
        });
    }
}

#[tauri::command]
pub async fn experience_replay(root: String, id: String) -> Result<(), String> {
    blocking(move || {
        let viewer = std::env::var_os("HUMAAN_RERUN_PATH").map(PathBuf::from)
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
        engine::list_metadata(&store)
            .map_err(|e| e.to_string())?
            .into_iter()
            .map(|m| {
                engine::annotations::read(&store.join(&m.id), &m.id)
                    .map(|a| (m.id, a))
                    .map_err(|e| e.to_string())
            })
            .collect()
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
