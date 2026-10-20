//! Local policy registry and assignment history. Never sends commands to hardware.
use serde_json::{json, Value};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::Mutex,
};
static MUTATION: Mutex<()> = Mutex::new(());
fn folder(root: &str, kind: &str) -> Result<PathBuf, String> {
    if !matches!(kind, "policies" | "policy-assignments") {
        return Err("Invalid policy collection.".into());
    }
    let (root, _) = humaan_engine::project::discover(Path::new(root))
        .map_err(|e| e.to_string())?
        .ok_or("Open a project first.")?;
    let mut path = fs::canonicalize(root).map_err(|e| e.to_string())?;
    for part in [".humaan", "deployments", kind] {
        path.push(part);
        if path.exists() || fs::symlink_metadata(&path).is_ok() {
            let meta = fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
            if meta.file_type().is_symlink() || !meta.is_dir() {
                return Err("Policy storage must use project directories.".into());
            }
        } else {
            fs::create_dir(&path).map_err(|e| e.to_string())?;
        }
    }
    Ok(path)
}
fn path(root: &str, kind: &str, id: &str) -> Result<PathBuf, String> {
    uuid::Uuid::parse_str(id).map_err(|_| "Invalid policy or robot ID.")?;
    let path = folder(root, kind)?.join(format!("{id}.json"));
    if let Ok(meta) = fs::symlink_metadata(&path) {
        if !meta.file_type().is_file() {
            return Err("Invalid policy file.".into());
        }
    }
    Ok(path)
}
fn read(path: &Path) -> Result<Value, String> {
    if fs::metadata(path).map_err(|e| e.to_string())?.len() > 120 * 1024 * 1024 {
        return Err("Policy snapshot exceeds 120 MB.".into());
    }
    serde_json::from_slice(&fs::read(path).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}
fn write(root: &str, kind: &str, id: &str, value: &Value, replace: bool) -> Result<(), String> {
    let destination = path(root, kind, id)?;
    let data = serde_json::to_vec(value).map_err(|e| e.to_string())?;
    if data.len() > 120 * 1024 * 1024 {
        return Err("Policy snapshot exceeds 120 MB.".into());
    }
    let mut file = tempfile::NamedTempFile::new_in(destination.parent().unwrap())
        .map_err(|e| e.to_string())?;
    file.write_all(&data).map_err(|e| e.to_string())?;
    file.as_file().sync_all().map_err(|e| e.to_string())?;
    if replace {
        file.persist(destination).map_err(|e| e.to_string())?;
    } else {
        file.persist_noclobber(destination)
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}
fn summary(value: &Value) -> Value {
    json!({"id":value["id"], "name":value["name"], "createdAt":value["createdAt"], "runId":value["runId"], "versionId":value["versionId"], "robotName":value["run"]["experiment"]["robot"]["name"], "evaluation":value["run"]["result"]["evaluation"]})
}
fn list(root: &str) -> Result<Vec<Value>, String> {
    let mut result = vec![];
    for entry in fs::read_dir(folder(root, "policies")?).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        if entry.path().extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let id = entry
            .path()
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or("Invalid policy ID.")?
            .to_owned();
        result.push(summary(&read(&path(root, "policies", &id)?)?));
    }
    result.sort_by(|a, b| b["createdAt"].as_u64().cmp(&a["createdAt"].as_u64()));
    Ok(result)
}
fn validate(run: &Value) -> Result<(), String> {
    if run["status"] != "completed" {
        return Err("Only completed training runs can become policies.".into());
    }
    let policy = &run["result"]["policy"];
    if policy["format"] != "humaan-joint-feedback-v1"
        || policy["actions"] != "position_servo_target"
        || policy["observations"] != json!(["joint_position", "joint_velocity"])
    {
        return Err("Unsupported controller format.".into());
    }
    let ids = policy["jointIds"]
        .as_array()
        .filter(|a| !a.is_empty() && a.len() <= 32)
        .ok_or("Invalid policy joints.")?;
    let joints: Vec<_> = run["experiment"]["robot"]["joints"]
        .as_array()
        .ok_or("Missing robot snapshot.")?
        .iter()
        .filter(|j| j["type"] != "fixed")
        .collect();
    if joints.len() != ids.len() {
        return Err("Policy must control every movable joint.".into());
    }
    for key in ["gain", "damping", "target", "lower", "upper"] {
        if !policy[key].as_array().is_some_and(|a| {
            a.len() == ids.len() && a.iter().all(|v| v.as_f64().is_some_and(f64::is_finite))
        }) {
            return Err(format!("Invalid policy {key}."));
        }
    }
    let mut unique = std::collections::HashSet::new();
    for (i, id) in ids.iter().enumerate() {
        let id = id.as_str().ok_or("Invalid joint ID.")?;
        if !unique.insert(id) {
            return Err("Duplicate policy joint.".into());
        }
        let joint = joints
            .iter()
            .find(|j| j["id"] == id)
            .ok_or("Policy joint is absent from robot.")?;
        if !matches!(joint["type"].as_str(), Some("revolute" | "prismatic")) {
            return Err("Controller requires finite revolute or prismatic joints.".into());
        }
        let lo = policy["lower"][i].as_f64().unwrap();
        let hi = policy["upper"][i].as_f64().unwrap();
        let target = policy["target"][i].as_f64().unwrap();
        let same_limit = |key: &str, value: f64| {
            joint[key]
                .as_f64()
                .is_some_and(|n| (n - value).abs() < 1e-9)
        };
        if lo >= hi
            || target < lo
            || target > hi
            || !same_limit("lower", lo)
            || !same_limit("upper", hi)
            || policy["gain"][i].as_f64().unwrap() < 0.0
            || policy["damping"][i].as_f64().unwrap() < 0.0
        {
            return Err("Controller limits or gains do not match its robot.".into());
        }
    }
    Ok(())
}
fn promote(root: &str, run_id: &str, name: &str) -> Result<Value, String> {
    let _guard = MUTATION
        .lock()
        .map_err(|_| "Policy storage is unavailable.")?;
    for item in list(root)? {
        if item["runId"] == run_id {
            return read(&path(root, "policies", item["id"].as_str().unwrap())?);
        }
    }
    if name.trim().is_empty() || name.len() > 200 {
        return Err("Enter a policy name (up to 200 bytes).".into());
    }
    let run = crate::training::load(root, "runs", run_id)?;
    validate(&run)?;
    let id = uuid::Uuid::new_v4().to_string();
    let value = json!({"id":id,"name":name.trim(),"createdAt":crate::training::timestamp(),"runId":run_id,"versionId":run["experiment"]["versionId"],"run":run});
    write(root, "policies", &id, &value, false)?;
    Ok(value)
}
fn compatibility(policy: &Value, profile: &Value) -> Vec<String> {
    let mut issues = vec![];
    if let Err(error) = validate(&policy["run"]) {
        issues.push(error);
    }
    if policy["versionId"] != profile["versionId"] {
        issues.push("Robot version differs from the training version.".into());
    }
    if policy["run"]["experiment"]["robot"] != profile["robot"] {
        issues.push(
            "Robot geometry, joints, limits, or physical configuration differ from training."
                .into(),
        );
    }
    if let Err(error) = crate::deployments::validate_mapping(profile, &profile["robot"]) {
        issues.push(error);
    }
    issues
}
fn assignment(root: &str, robot_id: &str) -> Result<Value, String> {
    let path = path(root, "policy-assignments", robot_id)?;
    if !path.exists() {
        return Ok(json!({"robotId":robot_id,"history":[]}));
    }
    read(&path)
}
fn assign(
    root: &str,
    robot_id: &str,
    policy_id: Option<String>,
    expected_id: Option<String>,
    restore_id: Option<String>,
) -> Result<Value, String> {
    let _guard = MUTATION
        .lock()
        .map_err(|_| "Policy storage is unavailable.")?;
    let profile = crate::deployments::load(root, "robots", robot_id)?;
    let mut state = assignment(root, robot_id)?;
    let history = state["history"]
        .as_array()
        .ok_or("Invalid assignment history.")?;
    if history.last().and_then(|e| e["id"].as_str()) != expected_id.as_deref() {
        return Err("Assignment changed. Refresh before trying again.".into());
    }
    let next = if let Some(ref restore) = restore_id {
        let event = history
            .iter()
            .find(|e| e["id"] == *restore)
            .ok_or("Assignment revision not found.")?;
        event["policyId"].as_str().map(str::to_owned)
    } else {
        policy_id
    };
    if history.last().and_then(|e| e["policyId"].as_str()) == next.as_deref() {
        return Err("This policy assignment is already current.".into());
    }
    let mut name = Value::Null;
    if let Some(ref id) = next {
        let policy = read(&path(root, "policies", id)?)?;
        let issues = compatibility(&policy, &profile);
        if !issues.is_empty() {
            return Err(issues.join(" "));
        }
        name = policy["name"].clone();
    }
    state["history"].as_array_mut().unwrap().push(json!({"id":uuid::Uuid::new_v4().to_string(),"policyId":next,"policyName":name,"createdAt":crate::training::timestamp(),"action":if restore_id.is_some() {"rollback"} else if next.is_some() {"assign"} else {"unassign"},"restoredFrom":restore_id}));
    write(root, "policy-assignments", robot_id, &state, true)?;
    Ok(state)
}
#[tauri::command]
pub async fn policy_list(root: String) -> Result<Vec<Value>, String> {
    tauri::async_runtime::spawn_blocking(move || list(&root))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn policy_load(root: String, id: String) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || read(&path(&root, "policies", &id)?))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn policy_promote(root: String, run_id: String, name: String) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || promote(&root, &run_id, &name))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn policy_check(
    root: String,
    policy_id: String,
    robot_id: String,
) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let policy = read(&path(&root, "policies", &policy_id)?)?;
        let profile = crate::deployments::load(&root, "robots", &robot_id)?;
        let issues = compatibility(&policy, &profile);
        Ok(json!({"compatible":issues.is_empty(),"issues":issues,"runtime":"unavailable"}))
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn policy_assignment(root: String, robot_id: String) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || assignment(&root, &robot_id))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn policy_assign(
    root: String,
    robot_id: String,
    policy_id: Option<String>,
    expected_id: Option<String>,
    restore_id: Option<String>,
) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        assign(&root, &robot_id, policy_id, expected_id, restore_id)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;
    fn run(id: &str) -> Value {
        json!({"id":id,"name":"Reach target","status":"completed","experiment":{"versionId":"1-2","robot":{"name":"Arm","joints":[{"id":"joint","type":"revolute","lower":-1.0,"upper":1.0}]}},"result":{"policy":{"format":"humaan-joint-feedback-v1","observations":["joint_position","joint_velocity"],"actions":"position_servo_target","jointIds":["joint"],"gain":[1.0],"damping":[0.1],"target":[0.5],"lower":[-1.0],"upper":[1.0]},"evaluation":{"successRate":1.0}}})
    }
    fn setup() -> (tempfile::TempDir, String, String) {
        let dir = tempfile::tempdir().unwrap();
        humaan_engine::project::init(dir.path(), "Policies").unwrap();
        let root = dir.path().to_str().unwrap().to_owned();
        let robot_id = uuid::Uuid::new_v4().to_string();
        let profile = json!({"id":robot_id,"name":"Physical arm","versionId":"1-2","robot":run("1-2")["experiment"]["robot"],"mappings":[{"jointId":"joint","channel":"motor","unit":"rad","direction":1,"offset":0}]});
        fs::create_dir_all(dir.path().join(".humaan/deployments/robots")).unwrap();
        fs::write(
            dir.path()
                .join(format!(".humaan/deployments/robots/{robot_id}.json")),
            serde_json::to_vec(&profile).unwrap(),
        )
        .unwrap();
        crate::training::save(&root, "runs", &run("1-2"), false).unwrap();
        (dir, root, robot_id)
    }
    #[test]
    fn rejects_failed_malformed_and_unsupported_controllers() {
        let valid = run("1-2");
        validate(&valid).unwrap();
        for (pointer, value) in [
            ("/status", json!("failed")),
            ("/result/policy/format", json!("unknown")),
            ("/result/policy/gain", json!([])),
            ("/result/policy/target/0", json!(2)),
            ("/result/policy/jointIds/0", json!("other")),
            ("/result/policy/lower/0", json!(-2)),
        ] {
            let mut invalid = valid.clone();
            *invalid.pointer_mut(pointer).unwrap() = value;
            assert!(validate(&invalid).is_err(), "{pointer}");
        }
    }
    #[test]
    fn promotion_is_idempotent_and_preserves_deleted_source_run() {
        let (_dir, root, _) = setup();
        let policy = promote(&root, "1-2", "Controller v1").unwrap();
        assert_eq!(promote(&root, "1-2", "Other name").unwrap(), policy);
        fs::remove_file(
            crate::training::folder(&root, "runs")
                .unwrap()
                .join("1-2.json"),
        )
        .unwrap();
        assert_eq!(
            read(&path(&root, "policies", policy["id"].as_str().unwrap()).unwrap()).unwrap()["run"],
            run("1-2")
        );
        assert_eq!(list(&root).unwrap().len(), 1);
    }
    #[test]
    fn assignment_checks_compatibility_stale_revision_and_rollback() {
        let (_dir, root, robot) = setup();
        let first = promote(&root, "1-2", "First").unwrap();
        let first_id = first["id"].as_str().unwrap().to_owned();
        crate::training::save(&root, "runs", &run("2-3"), false).unwrap();
        let second = promote(&root, "2-3", "Second").unwrap();
        let second_id = second["id"].as_str().unwrap().to_owned();
        let one = assign(&root, &robot, Some(first_id.clone()), None, None).unwrap();
        let first_event = one["history"][0]["id"].as_str().unwrap().to_owned();
        assert!(assign(&root, &robot, Some(second_id.clone()), None, None).is_err());
        let two = assign(
            &root,
            &robot,
            Some(second_id),
            Some(first_event.clone()),
            None,
        )
        .unwrap();
        let back = assign(
            &root,
            &robot,
            None,
            Some(two["history"][1]["id"].as_str().unwrap().into()),
            Some(first_event),
        )
        .unwrap();
        assert_eq!(back["history"][2]["policyId"], first_id);
        assert_eq!(back["history"][2]["action"], "rollback");
        let cleared = assign(
            &root,
            &robot,
            None,
            Some(back["history"][2]["id"].as_str().unwrap().into()),
            None,
        )
        .unwrap();
        assert!(cleared["history"][3]["policyId"].is_null());
        let mut incompatible = run("3-4");
        incompatible["experiment"]["versionId"] = json!("9-9");
        crate::training::save(&root, "runs", &incompatible, false).unwrap();
        let wrong = promote(&root, "3-4", "Wrong version").unwrap();
        assert!(assign(
            &root,
            &robot,
            Some(wrong["id"].as_str().unwrap().into()),
            Some(cleared["history"][3]["id"].as_str().unwrap().into()),
            None
        )
        .is_err());
        assert_eq!(assignment(&root, &robot).unwrap(), cleared);
    }
    #[test]
    fn same_version_with_changed_robot_or_mapping_is_incompatible() {
        let (_dir, root, robot) = setup();
        let policy = promote(&root, "1-2", "Policy").unwrap();
        let mut profile = crate::deployments::load(&root, "robots", &robot).unwrap();
        assert!(compatibility(&policy, &profile).is_empty());
        profile["robot"]["joints"][0]["upper"] = json!(2.0);
        assert!(!compatibility(&policy, &profile).is_empty());
        profile["robot"] = policy["run"]["experiment"]["robot"].clone();
        profile["mappings"] = json!([]);
        assert!(!compatibility(&policy, &profile).is_empty());
        assert!(path(&root, "policies", "../file").is_err());
    }
}
