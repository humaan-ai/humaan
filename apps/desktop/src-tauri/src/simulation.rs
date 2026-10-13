use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tauri::Manager;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    id: String,
    version_id: String,
    name: String,
    created_at: u64,
    status: String,
}
#[derive(Serialize, Deserialize)]
pub struct Run {
    #[serde(flatten)]
    summary: Summary,
    robot: Value,
    config: Value,
    result: Value,
}
fn folder(root: &str) -> Result<PathBuf, String> {
    let (root, _) = humaan_engine::project::discover(Path::new(root))
        .map_err(|e| e.to_string())?
        .ok_or("Open a Humaan project first.")?;
    let root = fs::canonicalize(root).map_err(|e| e.to_string())?;
    let mut path = root.clone();
    for segment in [".humaan", "simulation", "runs"] {
        path.push(segment);
        if path.exists() {
            if !fs::canonicalize(&path)
                .map_err(|e| e.to_string())?
                .starts_with(&root)
            {
                return Err("Simulation storage must stay inside the project.".into());
            }
        } else {
            fs::create_dir(&path).map_err(|e| e.to_string())?;
        }
    }
    Ok(path)
}
fn run_path(root: &str, id: &str) -> Result<PathBuf, String> {
    if id.is_empty() || id.len() > 80 || !id.bytes().all(|b| b.is_ascii_digit() || b == b'-') {
        return Err("Invalid run ID.".into());
    }
    let folder = folder(root)?;
    let path = folder.join(format!("{id}.json"));
    if path.exists()
        && !fs::canonicalize(&path)
            .map_err(|e| e.to_string())?
            .starts_with(fs::canonicalize(folder).map_err(|e| e.to_string())?)
    {
        return Err("Invalid run path.".into());
    }
    Ok(path)
}
fn delete(root: &str, id: &str) -> Result<(), String> {
    let path = run_path(root, id)?;
    if !fs::symlink_metadata(&path)
        .map_err(|e| e.to_string())?
        .file_type()
        .is_file()
    {
        return Err("Invalid saved run path.".into());
    }
    fs::remove_file(path).map_err(|e| e.to_string())
}
#[tauri::command]
pub async fn simulation_delete(root: String, id: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || delete(&root, &id))
        .await
        .map_err(|e| e.to_string())?
}
fn load(root: &str, id: &str) -> Result<Run, String> {
    let path = run_path(root, id)?;
    if fs::metadata(&path).map_err(|e| e.to_string())?.len() > 120 * 1024 * 1024 {
        return Err("Run exceeds storage limit.".into());
    }
    serde_json::from_slice(&fs::read(path).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}
fn list(root: &str) -> Result<Vec<Summary>, String> {
    let mut runs = Vec::new();
    for entry in fs::read_dir(folder(root)?).map_err(|e| e.to_string())? {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            let id = path
                .file_stem()
                .and_then(|s| s.to_str())
                .ok_or("Invalid run filename.")?;
            runs.push(load(root, id)?.summary);
        }
    }
    runs.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(runs)
}
fn execute(root: &str, version_id: &str, python: &str, config: Value) -> Result<Run, String> {
    let path = crate::builder::snapshot_path(root, "versions", version_id)?;
    let project = fs::canonicalize(root).map_err(|e| e.to_string())?;
    if !fs::canonicalize(&path)
        .map_err(|e| e.to_string())?
        .starts_with(project)
    {
        return Err("Invalid robot version path.".into());
    }
    let snapshot: Value = serde_json::from_slice(&fs::read(path).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let robot = snapshot["robot"].clone();
    let duration = config["duration"].as_f64().ok_or("Invalid duration.")?;
    let gravity = config["gravity"].as_f64().ok_or("Invalid gravity.")?;
    if !(0.1..=10.0).contains(&duration) || !(0.0..=20.0).contains(&gravity) {
        return Err("Invalid simulation configuration.".into());
    }
    let temp = tempfile::tempdir().map_err(|e| e.to_string())?;
    let input = temp.path().join("input.json");
    let output = temp.path().join("output.json");
    fs::write(
        &input,
        serde_json::to_vec(&json!({"robot": robot, "config": config}))
            .map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    let result = (|| -> Result<Value, String> {
        let mut child = Command::new(python).arg("-I").arg("-c").arg(include_str!("simulation_worker.py")).arg(&input).arg(&output)
            .current_dir(temp.path()).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn()
            .map_err(|e| format!("Cannot start Python: {e}. Select a Python executable with MuJoCo installed."))?;
        let start = Instant::now();
        loop {
            if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
                if !status.success() { return Err(format!("MuJoCo worker exited with {status}. Check the Python environment and robot configuration.")); } break;
            }
            if start.elapsed() > Duration::from_secs(60) { let _ = child.kill(); let _ = child.wait(); return Err("Simulation timed out after 60 seconds.".into()); }
            std::thread::sleep(Duration::from_millis(30));
        }
        if fs::metadata(&output).map_err(|e| e.to_string())?.len() > 20 * 1024 * 1024 { return Err("Simulation output exceeds 20 MB.".into()); }
        serde_json::from_slice(&fs::read(output).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
    })().unwrap_or_else(|error| json!({"error":error}));
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?;
    let run = Run {
        summary: Summary {
            id: format!("{}-{}", now.as_nanos(), std::process::id()),
            version_id: version_id.into(),
            name: robot["name"].as_str().unwrap_or("Robot").into(),
            created_at: now.as_millis() as u64,
            status: if result["error"].is_string() {
                "failed"
            } else {
                "completed"
            }
            .into(),
        },
        robot,
        config,
        result,
    };
    let dir = folder(root)?;
    let mut file = tempfile::NamedTempFile::new_in(dir).map_err(|e| e.to_string())?;
    file.write_all(&serde_json::to_vec(&run).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    file.as_file().sync_all().map_err(|e| e.to_string())?;
    file.persist_noclobber(run_path(root, &run.summary.id)?)
        .map_err(|e| e.to_string())?;
    Ok(run)
}
#[tauri::command]
pub async fn simulation_list(root: String) -> Result<Vec<Summary>, String> {
    tauri::async_runtime::spawn_blocking(move || list(&root))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn simulation_load(root: String, id: String) -> Result<Run, String> {
    tauri::async_runtime::spawn_blocking(move || load(&root, &id))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn simulation_run(
    app: tauri::AppHandle,
    root: String,
    version_id: String,
    python: String,
    config: Value,
) -> Result<Run, String> {
    let tools_root = app
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?
        .join("simulation-tools");
    tauri::async_runtime::spawn_blocking(move || {
        let python = if python.trim().is_empty() {
            humaan_engine::simulation_environment::managed_python(&tools_root)
                .map_err(|e| e.to_string())?
                .to_string_lossy()
                .into_owned()
        } else {
            python
        };
        execute(&root, &version_id, &python, config)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires HUMAAN_TEST_PYTHON pointing to an environment with MuJoCo"]
    fn real_mujoco_worker_round_trip() {
        let python = std::env::var("HUMAAN_TEST_PYTHON").unwrap();
        let dir = tempfile::tempdir().unwrap();
        humaan_engine::project::init(dir.path(), "Physics").unwrap();
        let root = dir.path().to_str().unwrap();
        let path = crate::builder::snapshot_path(root, "versions", "123").unwrap();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let pose = json!({"xyz":[0,0,0],"rpy":[0,0,0]});
        let link = json!({"id":"base","name":"base","mass":1,"inertia":[0.01,0,0,0.01,0,0.01],"inertial":pose,"collisions":[{"kind":"box","size":[0.1,0.1,0.1],"origin":pose}]});
        let robot = json!({"name":"Physics","schema":1,"links":[link],"joints":[],"assets":{}});
        fs::write(path, serde_json::to_vec(&json!({"robot":robot})).unwrap()).unwrap();
        let run = execute(
            root,
            "123",
            &python,
            json!({"duration":0.1,"gravity":9.81,"targets":{}}),
        )
        .unwrap();
        assert_eq!(run.summary.status, "completed", "{}", run.result);
        assert_eq!(run.result["frames"].as_array().unwrap().len(), 3);
        assert_eq!(load(root, &run.summary.id).unwrap().robot, robot);
    }
    #[test]
    fn failed_runs_keep_the_exact_robot_and_configuration() {
        let dir = tempfile::tempdir().unwrap();
        humaan_engine::project::init(dir.path(), "Sim").unwrap();
        let root = dir.path().to_str().unwrap();
        let path = crate::builder::snapshot_path(root, "versions", "123").unwrap();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let robot = json!({"name":"Pinned robot","schema":1,"links":[],"joints":[],"assets":{}});
        fs::write(&path, serde_json::to_vec(&json!({"robot":robot})).unwrap()).unwrap();
        let config = json!({"duration":1,"gravity":9.81,"targets":{}});
        let run = execute(root, "123", "/nonexistent/humaan-python", config.clone()).unwrap();
        assert_eq!(run.summary.status, "failed");
        fs::remove_file(path).unwrap();
        let loaded = load(root, &run.summary.id).unwrap();
        assert_eq!(loaded.robot, robot);
        assert_eq!(loaded.config, config);
        assert_eq!(list(root).unwrap().len(), 1);
        assert!(delete(root, "../escape").is_err());
        delete(root, &run.summary.id).unwrap();
        assert!(list(root).unwrap().is_empty());
        assert!(delete(root, &run.summary.id).is_err());
        assert!(load(root, "../escape").is_err());
        assert!(execute(root, "../escape", "python3", config).is_err());
    }
}

#[tauri::command]
pub async fn simulation_tools_status(
    app: tauri::AppHandle,
) -> Result<humaan_engine::simulation_environment::Status, String> {
    let root = app
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?
        .join("simulation-tools");
    tauri::async_runtime::spawn_blocking(move || {
        humaan_engine::simulation_environment::status(&root).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn simulation_tools_install(
    app: tauri::AppHandle,
) -> Result<humaan_engine::simulation_environment::Status, String> {
    let root = app
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?
        .join("simulation-tools");
    tauri::async_runtime::spawn_blocking(move || {
        humaan_engine::simulation_environment::install(&root).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
