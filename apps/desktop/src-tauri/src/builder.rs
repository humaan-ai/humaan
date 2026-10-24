use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Version {
    id: String,
    name: String,
    saved_at: u64,
    links: usize,
    #[serde(default)]
    robot_id: Option<String>,
}
#[derive(Serialize, Deserialize)]
struct Snapshot {
    version: Version,
    robot: Value,
}

fn store(root: &str, kind: &str) -> Result<PathBuf, String> {
    if !matches!(kind, "versions" | "modules") {
        return Err("Invalid Builder collection.".into());
    }
    let (root, _) = humaan_engine::project::discover(Path::new(root))
        .map_err(|e| e.to_string())?
        .ok_or("Open a Humaan project first.")?;
    Ok(root.join(".humaan/builder").join(kind))
}
pub(crate) fn snapshot_path(root: &str, kind: &str, id: &str) -> Result<PathBuf, String> {
    if id.is_empty() || id.len() > 80 || !id.bytes().all(|b| b.is_ascii_digit() || b == b'-') {
        return Err("Invalid version ID.".into());
    }
    Ok(store(root, kind)?.join(format!("{id}.json")))
}
fn save(root: &str, kind: &str, robot: Value) -> Result<Version, String> {
    let name = robot["name"]
        .as_str()
        .filter(|s| !s.trim().is_empty())
        .ok_or("Robot name is required.")?
        .to_string();
    if robot["schema"] != 1 || !robot["joints"].is_array() || !robot["assets"].is_object() {
        return Err("Unsupported robot definition.".into());
    }
    let links = robot["links"]
        .as_array()
        .filter(|l| !l.is_empty())
        .ok_or("Add at least one link.")?
        .len();
    let folder = store(root, kind)?;
    fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?;
    let id = format!("{}-{}", now.as_nanos(), std::process::id());
    let snapshot = Snapshot {
        version: Version {
            id: id.clone(),
            name,
            saved_at: now.as_millis() as u64,
            links,
            robot_id: robot["projectRobotId"].as_str().map(str::to_owned),
        },
        robot,
    };
    let bytes = serde_json::to_vec(&snapshot).map_err(|e| e.to_string())?;
    if bytes.len() > 100 * 1024 * 1024 {
        return Err("Assembly exceeds the 100 MB storage limit.".into());
    }
    let mut file = tempfile::NamedTempFile::new_in(&folder).map_err(|e| e.to_string())?;
    file.write_all(&bytes).map_err(|e| e.to_string())?;
    file.as_file().sync_all().map_err(|e| e.to_string())?;
    file.persist_noclobber(snapshot_path(root, kind, &id)?)
        .map_err(|e| e.to_string())?;
    Ok(snapshot.version)
}
fn list(root: &str, kind: &str) -> Result<Vec<Version>, String> {
    let folder = store(root, kind)?;
    if !folder.exists() {
        return Ok(vec![]);
    }
    let mut versions = vec![];
    for entry in fs::read_dir(folder).map_err(|e| e.to_string())? {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let data = fs::read(&path).map_err(|e| e.to_string())?;
        let snapshot: Snapshot = serde_json::from_slice(&data)
            .map_err(|e| format!("Cannot read {}: {e}", path.display()))?;
        versions.push(snapshot.version);
    }
    versions.sort_by(|a, b| b.saved_at.cmp(&a.saved_at).then(b.id.cmp(&a.id)));
    Ok(versions)
}
#[tauri::command]
pub async fn builder_list(root: String, kind: String) -> Result<Vec<Version>, String> {
    tauri::async_runtime::spawn_blocking(move || list(&root, &kind))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn builder_load(root: String, kind: String, id: String) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let data = fs::read(snapshot_path(&root, &kind, &id)?).map_err(|e| e.to_string())?;
        let snapshot: Snapshot = serde_json::from_slice(&data).map_err(|e| e.to_string())?;
        Ok(snapshot.robot)
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn builder_save(root: String, kind: String, robot: Value) -> Result<Version, String> {
    tauri::async_runtime::spawn_blocking(move || save(&root, &kind, robot))
        .await
        .map_err(|e| e.to_string())?
}

fn delete_version(root: &str, id: &str) -> Result<(), String> {
    let path = snapshot_path(root, "versions", id)?;
    let (project_root, _) = humaan_engine::project::discover(Path::new(root))
        .map_err(|e| e.to_string())?
        .ok_or("Open a Humaan project first.")?;
    let project_root = fs::canonicalize(project_root).map_err(|e| e.to_string())?;
    let canonical = fs::canonicalize(&path).map_err(|e| e.to_string())?;
    if !canonical.starts_with(project_root) || !fs::symlink_metadata(&path).map_err(|e| e.to_string())?.file_type().is_file() {
        return Err("Invalid saved version path.".into());
    }
    fs::remove_file(path).map_err(|e| e.to_string())
}
#[tauri::command]
pub async fn builder_delete_version(root: String, id: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || delete_version(&root, &id))
        .await
        .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn versions_are_immutable_and_modules_are_separate() {
        let folder = tempfile::tempdir().unwrap();
        humaan_engine::project::init(folder.path(), "Builder test").unwrap();
        let root = folder.path().to_str().unwrap();
        let robot = serde_json::json!({"schema":1,"name":"Arm","links":[{"id":"base"}],"joints":[],"assets":{"mesh":{"data":"test"}}});
        let first = save(root, "versions", robot.clone()).unwrap();
        let mut changed = robot.clone();
        changed["name"] = "Arm v2".into();
        save(root, "versions", changed).unwrap();
        save(root, "modules", robot.clone()).unwrap();
        assert_eq!(list(root, "versions").unwrap().len(), 2);
        assert_eq!(list(root, "modules").unwrap().len(), 1);
        let original: Snapshot = serde_json::from_slice(
            &fs::read(snapshot_path(root, "versions", &first.id).unwrap()).unwrap(),
        )
        .unwrap();
        assert_eq!(original.robot, robot);
        assert!(delete_version(root, "../../escape").is_err());
        delete_version(root, &first.id).unwrap();
        assert_eq!(list(root, "versions").unwrap().len(), 1);
        assert_eq!(list(root, "modules").unwrap().len(), 1);
        assert!(delete_version(root, &first.id).is_err());
        assert!(snapshot_path(root, "versions", "../../escape").is_err());
        assert!(store(root, "../../escape").is_err());
        assert!(save(root, "versions", serde_json::json!({})).is_err());
    }

    #[test]
    fn versions_keep_robot_identity_without_changing_snapshot_paths() {
        let folder = tempfile::tempdir().unwrap();
        humaan_engine::project::init(folder.path(), "Multiple robots").unwrap();
        let root = folder.path().to_str().unwrap();
        for id in ["arm-a", "arm-b"] {
            let robot = serde_json::json!({"schema":1,"projectRobotId":id,"name":"Same name","links":[{"id":"base"}],"joints":[],"assets":{}});
            let version = save(root, "versions", robot).unwrap();
            assert_eq!(version.robot_id.as_deref(), Some(id));
            assert!(snapshot_path(root, "versions", &version.id).unwrap().exists());
        }
        let versions = list(root, "versions").unwrap();
        assert_eq!(versions.len(), 2);
        assert_ne!(versions[0].robot_id, versions[1].robot_id);
        let legacy: Version = serde_json::from_value(serde_json::json!({"id":"123","name":"Legacy","savedAt":0,"links":1})).unwrap();
        assert!(legacy.robot_id.is_none());
    }
}
