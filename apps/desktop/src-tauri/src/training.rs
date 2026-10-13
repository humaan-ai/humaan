//! Immutable experiments and bounded local/SSH training. Cloud provisioning is separate.
use serde_json::{json, Value};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
    sync::{Mutex, OnceLock},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tauri::Manager;

fn active() -> &'static Mutex<Vec<String>> {
    static ACTIVE: OnceLock<Mutex<Vec<String>>> = OnceLock::new();
    ACTIVE.get_or_init(|| Mutex::new(Vec::new()))
}
pub(crate) fn timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
pub(crate) fn id() -> String {
    format!(
        "{}-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos(),
        std::process::id()
    )
}
pub(crate) fn folder(root: &str, kind: &str) -> Result<PathBuf, String> {
    if ![
        "cloud-jobs",
        "cloud-quotes",
        "experiences",
        "loops",
        "environments",
        "tasks",
        "experiments",
        "runs",
    ]
    .contains(&kind)
    {
        return Err("Invalid training collection.".into());
    }
    let (root, _) = humaan_engine::project::discover(Path::new(root))
        .map_err(|e| e.to_string())?
        .ok_or("Open a Humaan project first.")?;
    let mut path = fs::canonicalize(root).map_err(|e| e.to_string())?;
    for part in [".humaan", "training", kind] {
        path.push(part);
        if let Ok(meta) = fs::symlink_metadata(&path) {
            if meta.file_type().is_symlink() || !meta.is_dir() {
                return Err("Training storage must use project directories, not symlinks.".into());
            }
        } else {
            fs::create_dir(&path).map_err(|e| e.to_string())?;
        }
    }
    Ok(path)
}
fn path(root: &str, kind: &str, id: &str) -> Result<PathBuf, String> {
    if id.is_empty() || id.len() > 80 || !id.bytes().all(|b| b.is_ascii_digit() || b == b'-') {
        return Err("Invalid training ID.".into());
    }
    let path = folder(root, kind)?.join(format!("{id}.json"));
    if let Ok(meta) = fs::symlink_metadata(&path) {
        if !meta.file_type().is_file() {
            return Err("Invalid training file.".into());
        }
    }
    Ok(path)
}
fn read_file(path: &Path) -> Result<Value, String> {
    if fs::metadata(path).map_err(|e| e.to_string())?.len() > 120 * 1024 * 1024 {
        return Err("Training file exceeds 120 MB.".into());
    }
    serde_json::from_slice(&fs::read(path).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}
pub(crate) fn load(root: &str, kind: &str, id: &str) -> Result<Value, String> {
    read_file(&path(root, kind, id)?)
}
pub(crate) fn save(root: &str, kind: &str, value: &Value, replace: bool) -> Result<(), String> {
    let destination = path(root, kind, string(value, "id")?)?;
    let bytes = serde_json::to_vec(value).map_err(|e| e.to_string())?;
    if bytes.len() > 120 * 1024 * 1024 {
        return Err("Training file exceeds 120 MB.".into());
    }
    let mut file = tempfile::NamedTempFile::new_in(destination.parent().unwrap())
        .map_err(|e| e.to_string())?;
    file.write_all(&bytes).map_err(|e| e.to_string())?;
    file.as_file().sync_all().map_err(|e| e.to_string())?;
    if replace {
        file.persist(destination).map_err(|e| e.to_string())?;
    } else {
        file.persist_noclobber(destination)
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}
fn string<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    value[key]
        .as_str()
        .filter(|s| !s.trim().is_empty() && s.len() <= 200)
        .ok_or_else(|| format!("{key} is required (up to 200 bytes)."))
}
fn number(value: &Value, key: &str, min: f64, max: f64) -> Result<f64, String> {
    value[key]
        .as_f64()
        .filter(|n| n.is_finite() && (min..=max).contains(n))
        .ok_or_else(|| format!("{key} must be between {min} and {max}."))
}
fn create(root: &str, kind: &str, input: Value) -> Result<Value, String> {
    let mut value = json!({"id":id(), "name":string(&input,"name")?.trim(), "createdAt":timestamp(), "schema":1});
    match kind {
        "experiences" => {}
        "loops" => {
            let parent = load(root, "experiences", string(&input, "experienceId")?)?;
            value["experienceId"] = parent["id"].clone();
        }
        "environments" => {
            value["kind"] = json!("fixed-base-floor");
            value["gravity"] = json!(number(&input, "gravity", 0., 20.)?);
            value["duration"] = json!(number(&input, "duration", 0.1, 5.)?);
        }
        "tasks" => {
            value["kind"] = json!("joint-reach");
            value["targetFraction"] = json!(number(&input, "targetFraction", 0.05, 0.95)?);
            value["tolerance"] = json!(number(&input, "tolerance", 0.001, 0.25)?);
        }
        "experiments" => {
            let loop_def = load(root, "loops", string(&input, "loopId")?)?;
            value["experience"] = load(root, "experiences", string(&loop_def, "experienceId")?)?;
            value["loop"] = loop_def;
            value["environment"] = load(root, "environments", string(&input, "environmentId")?)?;
            value["task"] = load(root, "tasks", string(&input, "taskId")?)?;
            let version_id = string(&input, "versionId")?;
            let snapshot_path = crate::builder::snapshot_path(root, "versions", version_id)?;
            let project = humaan_engine::project::discover(Path::new(root))
                .map_err(|e| e.to_string())?
                .ok_or("Project missing")?
                .0;
            if !fs::canonicalize(&snapshot_path)
                .map_err(|e| e.to_string())?
                .starts_with(fs::canonicalize(project).map_err(|e| e.to_string())?)
            {
                return Err("Robot snapshot must stay inside project.".into());
            }
            value["robot"] = read_file(&snapshot_path)?["robot"].clone();
            let joints = value["robot"]["joints"]
                .as_array()
                .ok_or("Invalid robot joints")?;
            let movable: Vec<_> = joints.iter().filter(|j| j["type"] != "fixed").collect();
            if movable.is_empty()
                || movable.len() > 32
                || movable
                    .iter()
                    .any(|j| j["type"] != "revolute" && j["type"] != "prismatic")
            {
                return Err("Training needs 1–32 revolute or prismatic joints.".into());
            }
            value["versionId"] = json!(version_id);
            let mut config = json!({"algorithm":"cem-joint-feedback-v1"});
            for (key, min, max) in [
                ("iterations", 1., 20.),
                ("population", 4., 32.),
                ("seed", 0., 99999.),
            ] {
                let n = number(&input["config"], key, min, max)?;
                if n.fract() != 0. {
                    return Err(format!("{key} must be an integer."));
                }
                config[key] = json!(n as u64);
            }
            value["config"] = config;
        }
        _ => return Err("Cannot create this training collection.".into()),
    }
    save(root, kind, &value, false)?;
    Ok(value)
}
fn summary(mut value: Value, kind: &str) -> Value {
    if kind == "experiments" {
        value["robotName"] = value["robot"]["name"].clone();
        value.as_object_mut().unwrap().remove("robot");
    }
    if kind == "runs" {
        if value["status"] == "running"
            && !active()
                .lock()
                .unwrap()
                .contains(&value["id"].as_str().unwrap_or_default().to_string())
        {
            value["status"] = json!("interrupted");
            value["error"] =
                json!("The app stopped before results were saved. Launch this experiment again.");
        }
        value["evaluation"] = value["result"]["evaluation"].clone();
        value["comparisonKey"] = value["result"]["comparisonKey"].clone();
        value.as_object_mut().unwrap().remove("result");
        value.as_object_mut().unwrap().remove("experiment");
    }
    value
}
pub(crate) fn list(root: &str, kind: &str) -> Result<Vec<Value>, String> {
    let mut values = Vec::new();
    for entry in fs::read_dir(folder(root, kind)?).map_err(|e| e.to_string())? {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            values.push(summary(
                load(
                    root,
                    kind,
                    path.file_stem()
                        .and_then(|s| s.to_str())
                        .ok_or("Invalid filename")?,
                )?,
                kind,
            ));
        }
    }
    values.sort_by_key(|v| std::cmp::Reverse(v["createdAt"].as_u64().unwrap_or_default()));
    Ok(values)
}
pub(crate) fn worker_source() -> String {
    format!(
        "{}\n{}",
        include_str!("simulation_worker.py")
            .split("if __name__ == '__main__':")
            .next()
            .unwrap(),
        include_str!("training_worker.py")
    )
}
fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}
fn cloud_command(execution: &Value, source: &str) -> Result<Command, String> {
    let host = string(execution, "host")?;
    if host.starts_with('-')
        || !host
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b".-_@".contains(&b))
    {
        return Err("Use an SSH config alias or user@hostname.".into());
    }
    let python = string(execution, "python")?;
    if !python.starts_with('/')
        || !python
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"/._-".contains(&b))
    {
        return Err("Cloud Python must be an absolute executable path without spaces.".into());
    }
    let mut cmd = Command::new("ssh");
    cmd.args([
        "-T",
        "-o",
        "BatchMode=yes",
        "-o",
        "StrictHostKeyChecking=yes",
        "-o",
        "ConnectTimeout=10",
        "-o",
        "ServerAliveInterval=15",
        "-o",
        "ServerAliveCountMax=2",
        host,
    ]);
    cmd.arg(format!(
        "{} -I -c {}",
        shell_quote(python),
        shell_quote(source)
    ));
    Ok(cmd)
}
fn worker(experiment: &Value, execution: &Value, python: &str) -> Result<Value, String> {
    let source = worker_source();
    let temp = tempfile::tempdir().map_err(|e| e.to_string())?;
    let input = temp.path().join("input.json");
    let output = temp.path().join("output.txt");
    let errors = temp.path().join("errors.txt");
    fs::write(
        &input,
        serde_json::to_vec(experiment).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    let mut cmd = if execution["kind"] == "cloud" {
        cloud_command(execution, &source)?
    } else {
        let mut c = Command::new(python);
        c.args(["-I", "-c", &source]);
        c
    };
    let mut child = cmd
        .current_dir(temp.path())
        .stdin(fs::File::open(input).map_err(|e| e.to_string())?)
        .stdout(fs::File::create(&output).map_err(|e| e.to_string())?)
        .stderr(fs::File::create(&errors).map_err(|e| e.to_string())?)
        .spawn()
        .map_err(|e| format!("Could not start training: {e}"))?;
    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            break status;
        }
        if start.elapsed() > Duration::from_secs(240)
            || fs::metadata(&output)
                .map(|m| m.len() > 30 * 1024 * 1024)
                .unwrap_or(false)
            || fs::metadata(&errors)
                .map(|m| m.len() > 1024 * 1024)
                .unwrap_or(false)
        {
            let _ = child.kill();
            let _ = child.wait();
            return Err("Training exceeded its time or output limit. Cloud workers also stop after their own 180-second training limit.".into());
        }
        std::thread::sleep(Duration::from_millis(100));
    };
    if !status.success() {
        let text = fs::read_to_string(errors).unwrap_or_default();
        return Err(format!(
            "Training process failed ({status}): {}",
            text.chars().take(2000).collect::<String>()
        ));
    }
    let bytes = fs::read(output).map_err(|e| e.to_string())?;
    if bytes.len() > 30 * 1024 * 1024 {
        return Err("Training output exceeds 30 MB.".into());
    }
    let text = String::from_utf8(bytes).map_err(|e| e.to_string())?;
    let result: Value = serde_json::from_str(
        text.rsplit("HUMAAN_RESULT:")
            .next()
            .ok_or("Training returned no result")?
            .trim(),
    )
    .map_err(|e| format!("Invalid training result: {e}"))?;
    if result["error"].is_string() {
        return Ok(result);
    }
    if result["experimentId"] != experiment["id"]
        || !result["policy"].is_object()
        || !result["evaluation"].is_object()
    {
        return Err("Training result does not match this experiment.".into());
    }
    Ok(result)
}
fn execute(
    root: &str,
    experiment_id: &str,
    execution: Value,
    python: Result<String, String>,
) -> Result<Value, String> {
    if execution["kind"] != "local" && execution["kind"] != "cloud" {
        return Err("Choose local or cloud execution.".into());
    }
    if execution["kind"] == "cloud" {
        cloud_command(&execution, "")?;
    }
    let experiment = load(root, "experiments", experiment_id)?;
    let run_id = id();
    let mut run = json!({"id":run_id,"name":experiment["name"],"createdAt":timestamp(),"status":"running","experimentId":experiment_id,"versionId":experiment["versionId"],"robotName":experiment["robot"]["name"],"experienceId":experiment["experience"]["id"],"loopId":experiment["loop"]["id"],"execution":execution,"experiment":experiment});
    // Reserve before publishing so concurrent list requests do not mark this run interrupted.
    active().lock().unwrap().push(run_id.clone());
    let outcome = (|| {
        save(root, "runs", &run, false)?;
        let result = python
            .and_then(|python| worker(&run["experiment"], &run["execution"], &python))
            .unwrap_or_else(|e| json!({"error":e}));
        run["status"] = json!(if result["error"].is_string() {
            "failed"
        } else {
            "completed"
        });
        run["finishedAt"] = json!(timestamp());
        run["result"] = result;
        save(root, "runs", &run, true)?;
        Ok(run)
    })();
    active().lock().unwrap().retain(|id| id != &run_id);
    outcome
}
#[tauri::command]
pub async fn training_catalog(root: String) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut catalog = json!({});
        for kind in [
            "experiences",
            "loops",
            "environments",
            "tasks",
            "experiments",
            "runs",
        ] {
            catalog[kind] = json!(list(&root, kind)?);
        }
        Ok(catalog)
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn training_create(root: String, kind: String, input: Value) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || create(&root, &kind, input))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn training_load(root: String, kind: String, id: String) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut value = load(&root, &kind, &id)?;
        if kind == "runs" {
            let s = summary(value.clone(), &kind);
            value["status"] = s["status"].clone();
            value["error"] = s["error"].clone();
        }
        Ok(value)
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn training_run(
    app: tauri::AppHandle,
    root: String,
    experiment_id: String,
    execution: Value,
    python: String,
) -> Result<Value, String> {
    let tools = app
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?
        .join("simulation-tools");
    tauri::async_runtime::spawn_blocking(move || {
        let python = if execution["kind"] == "cloud" || !python.trim().is_empty() {
            Ok(python)
        } else {
            humaan_engine::simulation_environment::managed_python(&tools)
                .map(|p| p.to_string_lossy().into_owned())
                .map_err(|e| e.to_string())
        };
        execute(&root, &experiment_id, execution, python)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    pub(crate) fn fixture(root: &str) -> Value {
        let experience = create(root, "experiences", json!({"name":"Control"})).unwrap();
        let loop_def = create(
            root,
            "loops",
            json!({"name":"Baseline","experienceId":experience["id"]}),
        )
        .unwrap();
        let environment = create(
            root,
            "environments",
            json!({"name":"Bench","gravity":0,"duration":0.2}),
        )
        .unwrap();
        let task = create(
            root,
            "tasks",
            json!({"name":"Reach","targetFraction":0.7,"tolerance":0.05}),
        )
        .unwrap();
        let pose = json!({"xyz":[0,0,0],"rpy":[0,0,0]});
        let link = |id| json!({"id":id,"name":id,"mass":1,"inertia":[0.01,0,0,0.01,0,0.01],"inertial":pose,"collisions":[]});
        let robot = json!({"name":"Arm","schema":1,"links":[link("base"),link("arm")],"joints":[{"id":"joint","name":"joint","type":"revolute","parent":"base","child":"arm","origin":pose,"axis":[0,0,1],"lower":-1,"upper":1,"effort":10,"velocity":2}],"assets":{}});
        let path = crate::builder::snapshot_path(root, "versions", "123").unwrap();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, serde_json::to_vec(&json!({"robot":robot})).unwrap()).unwrap();
        create(root,"experiments",json!({"name":"Reach arm","loopId":loop_def["id"],"environmentId":environment["id"],"taskId":task["id"],"versionId":"123","config":{"iterations":1,"population":4,"seed":42}})).unwrap()
    }
    #[test]
    fn pinned_experiment_and_failed_run_survive_source_removal() {
        let dir = tempfile::tempdir().unwrap();
        humaan_engine::project::init(dir.path(), "Training").unwrap();
        let root = dir.path().to_str().unwrap();
        let experiment = fixture(root);
        fs::remove_file(crate::builder::snapshot_path(root, "versions", "123").unwrap()).unwrap();
        fs::remove_file(path(root, "tasks", experiment["task"]["id"].as_str().unwrap()).unwrap())
            .unwrap();
        let run = execute(
            root,
            experiment["id"].as_str().unwrap(),
            json!({"kind":"local"}),
            Ok("/missing/python".into()),
        )
        .unwrap();
        assert_eq!(run["status"], "failed");
        assert_eq!(run["experiment"], experiment);
        assert!(run["result"]["error"]
            .as_str()
            .unwrap()
            .contains("Could not start"));
        assert_eq!(list(root, "runs").unwrap().len(), 1);
        assert_eq!(
            load(root, "runs", run["id"].as_str().unwrap()).unwrap()["experiment"]["versionId"],
            "123"
        );
        assert!(save(root, "experiments", &experiment, false).is_err());
    }
    #[test]
    fn validates_definitions_paths_and_ssh_arguments() {
        let dir = tempfile::tempdir().unwrap();
        humaan_engine::project::init(dir.path(), "Training").unwrap();
        let root = dir.path().to_str().unwrap();
        assert!(create(
            root,
            "environments",
            json!({"name":"Bad","duration":100,"gravity":9.81})
        )
        .is_err());
        assert!(create(
            root,
            "tasks",
            json!({"name":"Bad","targetFraction":2,"tolerance":0.1})
        )
        .is_err());
        assert!(create(root, "loops", json!({"name":"Bad","experienceId":"123"})).is_err());
        assert!(load(root, "tasks", "../escape").is_err());
        assert!(load(root, "../escape", "123").is_err());
        for host in [
            "-oProxyCommand=bad",
            "host;touch /tmp/test",
            "host\ncommand",
            "user@host -p 2",
        ] {
            assert!(
                cloud_command(&json!({"host":host,"python":"/usr/bin/python3"}), "code").is_err()
            );
        }
        assert!(cloud_command(
            &json!({"host":"user@host","python":"/bin/python;bad"}),
            "code"
        )
        .is_err());
        let command = cloud_command(
            &json!({"host":"user@host","python":"/usr/bin/python3"}),
            "print('safe')",
        )
        .unwrap();
        let args: Vec<_> = command
            .get_args()
            .map(|a| a.to_string_lossy().to_string())
            .collect();
        assert!(args.contains(&"StrictHostKeyChecking=yes".into()));
        assert!(args.contains(&"BatchMode=yes".into()));
        assert_eq!(
            args.last().unwrap(),
            "'/usr/bin/python3' -I -c 'print('\"'\"'safe'\"'\"')'"
        );
        let interrupted = summary(json!({"id":"123","status":"running"}), "runs");
        assert_eq!(interrupted["status"], "interrupted");
    }
    #[cfg(unix)]
    #[test]
    fn rejects_symlink_storage_and_files() {
        let dir = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        humaan_engine::project::init(dir.path(), "Training").unwrap();
        let root = dir.path().to_str().unwrap();
        let folder = folder(root, "tasks").unwrap();
        let target = outside.path().join("task.json");
        fs::write(&target, b"{}").unwrap();
        std::os::unix::fs::symlink(&target, folder.join("123.json")).unwrap();
        assert!(load(root, "tasks", "123").is_err());
        std::os::unix::fs::symlink(outside.path(), dir.path().join(".humaan/training/runs"))
            .unwrap();
        assert!(list(root, "runs").is_err());
    }
    #[test]
    #[ignore = "requires HUMAAN_TEST_PYTHON with MuJoCo 3.2.7"]
    fn real_training_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        humaan_engine::project::init(dir.path(), "Training").unwrap();
        let root = dir.path().to_str().unwrap();
        let experiment = fixture(root);
        let run = execute(
            root,
            experiment["id"].as_str().unwrap(),
            json!({"kind":"local"}),
            Ok(std::env::var("HUMAAN_TEST_PYTHON").unwrap()),
        )
        .unwrap();
        assert_eq!(run["status"], "completed", "{}", run["result"]);
        assert_eq!(run["result"]["policy"]["jointIds"], json!(["joint"]));
        assert_eq!(
            run["result"]["evaluation"]["seeds"]
                .as_array()
                .unwrap()
                .len(),
            5
        );
        let summary = list(root, "runs").unwrap().remove(0);
        assert!(summary["experiment"].is_null());
        assert!(summary["evaluation"]["successRate"].is_number());
        assert_eq!(
            load(root, "runs", run["id"].as_str().unwrap()).unwrap()["result"],
            run["result"]
        );
    }
}
