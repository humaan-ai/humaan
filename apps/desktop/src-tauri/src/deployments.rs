//! Read-only, loopback telemetry and immutable project-local robot/session snapshots.
use serde_json::{json, Value};
use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    net::{SocketAddr, TcpStream},
    path::{Path, PathBuf},
    time::Duration,
};

fn folder(root: &str, kind: &str) -> Result<PathBuf, String> {
    if !matches!(kind, "robots" | "sessions") {
        return Err("Invalid deployment collection.".into());
    }
    let (root, _) = humaan_engine::project::discover(Path::new(root))
        .map_err(|e| e.to_string())?
        .ok_or("Open a project first.")?;
    Ok(root.join(".humaan/deployments").join(kind))
}
fn path(root: &str, kind: &str, id: &str) -> Result<PathBuf, String> {
    uuid::Uuid::parse_str(id).map_err(|_| "Invalid deployment ID.")?;
    Ok(folder(root, kind)?.join(format!("{id}.json")))
}
pub(crate) fn load(root: &str, kind: &str, id: &str) -> Result<Value, String> {
    serde_json::from_slice(&fs::read(path(root, kind, id)?).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}
pub(crate) fn validate_mapping(value: &Value, robot: &Value) -> Result<(), String> {
    let mappings = value["mappings"]
        .as_array()
        .ok_or("Joint mappings are required.")?;
    let joints: Vec<_> = robot["joints"]
        .as_array()
        .ok_or("Invalid robot.")?
        .iter()
        .filter(|j| j["type"] != "fixed")
        .collect();
    if joints.is_empty() || mappings.len() != joints.len() {
        return Err("Map every movable joint.".into());
    }
    let mut channels = std::collections::HashSet::new();
    for joint in joints {
        let entries: Vec<_> = mappings
            .iter()
            .filter(|m| m["jointId"] == joint["id"])
            .collect();
        if entries.len() != 1 {
            return Err("Each joint needs one mapping.".into());
        }
        let m = entries[0];
        let channel = m["channel"]
            .as_str()
            .filter(|s| !s.trim().is_empty() && s.len() <= 128)
            .ok_or("Enter a telemetry channel.")?;
        if !channels.insert(channel) {
            return Err("Telemetry channels must be unique.".into());
        }
        let unit = m["unit"].as_str().unwrap_or("");
        if (joint["type"] == "prismatic" && unit != "m")
            || (joint["type"] != "prismatic" && !matches!(unit, "rad" | "deg"))
        {
            return Err(
                "Use metres for prismatic joints and radians/degrees for rotating joints.".into(),
            );
        }
        if !matches!(m["direction"].as_f64(), Some(1.0) | Some(-1.0))
            || m["offset"].as_f64().filter(|n| n.is_finite()).is_none()
        {
            return Err("Invalid calibration.".into());
        }
    }
    Ok(())
}
fn save(root: &str, kind: &str, mut value: Value) -> Result<Value, String> {
    let dir = folder(root, kind)?;
    if value["name"]
        .as_str()
        .filter(|s| !s.trim().is_empty() && s.len() <= 200)
        .is_none()
    {
        return Err("Enter a name (up to 200 bytes).".into());
    }
    if kind == "robots" {
        let version = value["versionId"]
            .as_str()
            .ok_or("Select a saved robot version.")?;
        let snapshot: Value = serde_json::from_slice(
            &fs::read(crate::builder::snapshot_path(root, "versions", version)?)
                .map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        value["robot"] = snapshot["robot"].clone();
        validate_mapping(&value, &value["robot"])?;
        if value["port"]
            .as_u64()
            .filter(|p| *p > 0 && *p <= 65535)
            .is_none()
        {
            return Err("Port must be 1–65535.".into());
        }
    } else {
        let profile = load(
            root,
            "robots",
            value["profile"]["id"].as_str().ok_or("Select a robot.")?,
        )?;
        value["profile"] = profile;
        let frames = value["frames"]
            .as_array()
            .filter(|f| !f.is_empty() && f.len() <= 10000)
            .ok_or("Record 1–10,000 samples.")?;
        let mut previous = -1.0;
        for frame in frames {
            let time = frame["time"]
                .as_f64()
                .filter(|n| n.is_finite() && *n >= 0.0 && *n > previous)
                .ok_or("Invalid sample timestamps.")?;
            previous = time;
            let positions = frame["positions"]
                .as_object()
                .ok_or("Missing joint positions.")?;
            for mapping in value["profile"]["mappings"]
                .as_array()
                .ok_or("Invalid profile.")?
            {
                if positions
                    .get(mapping["jointId"].as_str().unwrap_or(""))
                    .and_then(Value::as_f64)
                    .filter(|n| n.is_finite())
                    .is_none()
                {
                    return Err("Invalid recorded joint position.".into());
                }
            }
        }
    }
    let id = uuid::Uuid::new_v4().to_string();
    value["id"] = json!(id);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let bytes = serde_json::to_vec(&value).map_err(|e| e.to_string())?;
    if bytes.len() > 120 * 1024 * 1024 {
        return Err("Deployment snapshot exceeds 120 MB.".into());
    }
    let mut file = tempfile::NamedTempFile::new_in(&dir).map_err(|e| e.to_string())?;
    file.write_all(&bytes).map_err(|e| e.to_string())?;
    file.as_file().sync_all().map_err(|e| e.to_string())?;
    file.persist_noclobber(path(root, kind, &id)?)
        .map_err(|e| e.to_string())?;
    Ok(value)
}
#[tauri::command]
pub async fn deployment_save(root: String, kind: String, value: Value) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || save(&root, &kind, value))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn deployment_load(root: String, kind: String, id: String) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || load(&root, &kind, &id))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn deployment_list(root: String, kind: String) -> Result<Vec<Value>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let dir = folder(&root, &kind)?;
        if !dir.exists() {
            return Ok(vec![]);
        }
        let mut entries = vec![];
        for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
            let path = entry.map_err(|e| e.to_string())?.path();
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            let v: Value = serde_json::from_slice(&fs::read(path).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
            entries.push(json!({"id":v["id"], "name":v["name"], "createdAt":v["createdAt"]}));
        }
        entries.sort_by(|a, b| b["createdAt"].as_u64().cmp(&a["createdAt"].as_u64()));
        Ok(entries)
    })
    .await
    .map_err(|e| e.to_string())?
}
fn sample(port: u16) -> Result<Value, String> {
    let stream = TcpStream::connect_timeout(
        &SocketAddr::from(([127, 0, 0, 1], port)),
        Duration::from_millis(750),
    )
    .map_err(|e| format!("Telemetry bridge unavailable: {e}"))?;
    stream
        .set_read_timeout(Some(Duration::from_millis(750)))
        .map_err(|e| e.to_string())?;
    let mut bytes = Vec::new();
    BufReader::new(stream)
        .take(65537)
        .read_until(b'\n', &mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 65536 || bytes.last() != Some(&b'\n') {
        return Err("Expected a newline-terminated telemetry packet under 64 KB.".into());
    }
    let value: Value =
        serde_json::from_slice(&bytes).map_err(|e| format!("Invalid telemetry JSON: {e}"))?;
    if value["sequence"].as_u64().is_none()
        || value["positions"]
            .as_object()
            .filter(|p| {
                !p.is_empty()
                    && p.len() <= 128
                    && p.values().all(|v| v.as_f64().is_some_and(f64::is_finite))
            })
            .is_none()
    {
        return Err("Expected sequence and finite positions in telemetry packet.".into());
    }
    Ok(value)
}
#[tauri::command]
pub async fn deployment_sample(port: u16) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || sample(port))
        .await
        .map_err(|e| e.to_string())?
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_invalid_mapping() {
        let robot = json!({"joints":[{"id":"a","type":"prismatic"}]});
        let mut profile = json!({"mappings":[{"jointId":"a","channel":"motor1","unit":"rad","direction":1,"offset":0}]});
        assert!(validate_mapping(&profile, &robot).is_err());
        profile["mappings"][0]["unit"] = json!("m");
        assert!(validate_mapping(&profile, &robot).is_ok());
        profile["mappings"][0]["direction"] = json!(0);
        assert!(validate_mapping(&profile, &robot).is_err());
    }
    #[test]
    fn session_pins_saved_robot_and_rejects_invalid_timing() {
        let dir = tempfile::tempdir().unwrap();
        humaan_engine::project::init(dir.path(), "Live Link").unwrap();
        let root = dir.path().to_str().unwrap();
        let version_path = crate::builder::snapshot_path(root, "versions", "1-2").unwrap();
        fs::create_dir_all(version_path.parent().unwrap()).unwrap();
        let robot = json!({"name":"Pinned", "joints":[{"id":"a", "type":"revolute"}], "assets":{}});
        fs::write(
            &version_path,
            serde_json::to_vec(&json!({"robot":robot})).unwrap(),
        )
        .unwrap();
        let profile = save(root, "robots", json!({"name":"Physical", "versionId":"1-2", "port":9876, "mappings":[{"jointId":"a", "channel":"motor", "unit":"rad", "direction":1, "offset":0}]})).unwrap();
        fs::remove_file(version_path).unwrap();
        let mut session = json!({"name":"Session", "profile":{"id":profile["id"], "robot":{"name":"Tampered"}}, "frames":[{"time":0.0, "positions":{"a":0.1}}, {"time":0.1,"positions":{"a":0.2}}]});
        let saved = save(root, "sessions", session.clone()).unwrap();
        assert_eq!(saved["profile"]["robot"], robot);
        assert_eq!(
            load(root, "sessions", saved["id"].as_str().unwrap()).unwrap(),
            saved
        );
        session["frames"][1]["time"] = json!(0.0);
        assert!(save(root, "sessions", session).is_err());
    }
    #[test]
    fn telemetry_reads_finite_newline_packet_without_sending_commands() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .write_all(b"{\"sequence\":1,\"positions\":{\"motor\":0.2}}\n")
                .unwrap();
        });
        assert_eq!(sample(port).unwrap()["positions"]["motor"], json!(0.2));
        handle.join().unwrap();
    }
    #[test]
    fn rejects_invalid_paths() {
        assert!(path("/tmp", "robots", "../x").is_err());
        assert!(folder("/tmp", "other").is_err());
    }
}
