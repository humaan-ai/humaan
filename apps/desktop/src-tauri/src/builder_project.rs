use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    fs,
    io::Write,
    path::{Component, Path, PathBuf},
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectRobot {
    id: String,
    name: String,
    source: Option<String>,
    #[serde(default)]
    missing: bool,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectRobots {
    active_robot_id: Option<String>,
    robots: Vec<ProjectRobot>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RobotFile {
    path: String,
    size: u64,
}

#[derive(Deserialize)]
pub struct ImportFile {
    path: String,
    data: Vec<u8>,
}

fn folder_name(name: &str) -> Result<&str, String> {
    let name = name.trim();
    if name.is_empty()
        || name.len() > 80
        || name.starts_with('.')
        || name.ends_with('.')
        || name
            .chars()
            .any(|c| c.is_control() || "\\/:*?\"<>|".contains(c))
    {
        return Err(
            "Use a folder name of 1–80 characters without path separators or special characters."
                .into(),
        );
    }
    Ok(name)
}

fn import_files(root: &Path, name: &str, files: Vec<ImportFile>) -> Result<ProjectRobots, String> {
    let name = folder_name(name)?;
    let destination = within(root, &format!("robots/{name}"))?;
    if destination.exists() {
        return Err(format!(
            "robots/{name} already exists. Choose another folder name or cancel."
        ));
    }
    if files.is_empty()
        || files.len() > 10_000
        || !files
            .iter()
            .any(|f| f.path.to_ascii_lowercase().ends_with(".urdf"))
    {
        return Err("Include at least one URDF and no more than 10,000 files.".into());
    }
    if files.iter().map(|f| f.data.len() as u64).sum::<u64>() > 128 * 1024 * 1024 {
        return Err("Import up to 128 MB of files at a time.".into());
    }
    let mut paths = std::collections::HashSet::new();
    for file in &files {
        if file.path.contains('\\')
            || file
                .path
                .split('/')
                .any(|part| part.is_empty() || part == "." || part == "..")
            || Path::new(&file.path)
                .components()
                .any(|c| !matches!(c, Component::Normal(_)))
            || !paths.insert(file.path.to_ascii_lowercase())
        {
            return Err(format!("Invalid or duplicate import path: {}", file.path));
        }
    }
    let robots = within(root, "robots")?;
    fs::create_dir_all(&robots).map_err(|e| e.to_string())?;
    let staging = tempfile::tempdir_in(&robots).map_err(|e| e.to_string())?;
    for file in files {
        let path = staging.path().join(file.path);
        fs::create_dir_all(path.parent().ok_or("Invalid import path.")?)
            .map_err(|e| e.to_string())?;
        let mut output = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|e| e.to_string())?;
        output.write_all(&file.data).map_err(|e| e.to_string())?;
        output.sync_all().map_err(|e| e.to_string())?;
    }
    // Reserve the destination exclusively; never merge with or replace another robot.
    fs::create_dir(&destination).map_err(|e| {
        if e.kind() == std::io::ErrorKind::AlreadyExists {
            format!("robots/{name} already exists. Choose another folder name or cancel.")
        } else {
            e.to_string()
        }
    })?;
    let result = (|| {
        for entry in fs::read_dir(staging.path()).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            fs::rename(entry.path(), destination.join(entry.file_name()))
                .map_err(|e| e.to_string())?;
        }
        // Remove the now-empty staging directory before discovery.
        staging.close().map_err(|e| e.to_string())?;
        discover(root)
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(&destination);
    }
    result
}

fn project(root: &str) -> Result<PathBuf, String> {
    let (root, _) = humaan_engine::project::discover(Path::new(root))
        .map_err(|e| e.to_string())?
        .ok_or("Open a Humaan project first.")?;
    fs::canonicalize(root).map_err(|e| e.to_string())
}

// Both reads and writes must remain inside the opened project, including symlinks.
fn within(root: &Path, relative: &str) -> Result<PathBuf, String> {
    let relative = Path::new(relative);
    if relative.as_os_str().is_empty()
        || relative
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err("Invalid project path.".into());
    }
    let path = root.join(relative);
    let mut ancestor = path.as_path();
    while !ancestor.exists() {
        if fs::symlink_metadata(ancestor).is_ok() {
            return Err("Invalid project symlink.".into());
        }
        ancestor = ancestor.parent().ok_or("Invalid project path.")?;
    }
    if !fs::canonicalize(ancestor)
        .map_err(|e| e.to_string())?
        .starts_with(root)
    {
        return Err("Robot files must stay inside the project.".into());
    }
    Ok(path)
}

fn read_registry(root: &Path) -> Result<ProjectRobots, String> {
    let path = within(root, ".humaan/builder/robots.json")?;
    if !path.exists() {
        return Ok(ProjectRobots::default());
    }
    serde_json::from_slice(&fs::read(path).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}

fn write_json(root: &Path, relative: &str, value: &impl Serialize) -> Result<(), String> {
    let path = within(root, relative)?;
    let folder = path.parent().ok_or("Invalid Builder path.")?;
    let bytes = serde_json::to_vec(value).map_err(|e| e.to_string())?;
    if bytes.len() > 100 * 1024 * 1024 {
        return Err("Builder draft exceeds 100 MB.".into());
    }
    fs::create_dir_all(folder).map_err(|e| e.to_string())?;
    let mut file = tempfile::NamedTempFile::new_in(folder).map_err(|e| e.to_string())?;
    file.write_all(&bytes).map_err(|e| e.to_string())?;
    file.as_file().sync_all().map_err(|e| e.to_string())?;
    file.persist(path).map_err(|e| e.to_string())?;
    Ok(())
}

fn scan(root: &Path, folder: &Path, files: &mut Vec<RobotFile>) -> Result<(), String> {
    if !folder.exists() {
        return Ok(());
    }
    if !fs::canonicalize(folder)
        .map_err(|e| e.to_string())?
        .starts_with(root)
    {
        return Err("Robot folder must stay inside the project.".into());
    }
    for entry in fs::read_dir(folder).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let kind = entry.file_type().map_err(|e| e.to_string())?;
        // Skip symlink entries to avoid loops and assets outside the project.
        if kind.is_symlink() {
            continue;
        }
        let path = entry.path();
        if kind.is_dir() && !entry.file_name().to_string_lossy().starts_with('.') {
            scan(root, &path, files)?;
        } else if kind.is_file()
            && matches!(
                path.extension()
                    .and_then(|s| s.to_str())
                    .map(str::to_ascii_lowercase)
                    .as_deref(),
                Some("urdf" | "stl" | "obj")
            )
        {
            files.push(RobotFile {
                path: path
                    .strip_prefix(root)
                    .map_err(|e| e.to_string())?
                    .to_string_lossy()
                    .replace('\\', "/"),
                size: entry.metadata().map_err(|e| e.to_string())?.len(),
            });
        }
    }
    Ok(())
}

fn discover(root: &Path) -> Result<ProjectRobots, String> {
    let mut state = read_registry(root)?;
    let mut files = vec![];
    scan(root, &within(root, "robots")?, &mut files)?;
    files.sort_by(|a, b| a.path.cmp(&b.path));
    for robot in &mut state.robots {
        robot.missing = robot
            .source
            .as_ref()
            .is_some_and(|source| !files.iter().any(|f| &f.path == source));
    }
    for file in files
        .into_iter()
        .filter(|f| f.path.to_ascii_lowercase().ends_with(".urdf"))
    {
        if state
            .robots
            .iter()
            .any(|r| r.source.as_deref() == Some(&file.path))
        {
            continue;
        }
        state.robots.push(ProjectRobot {
            id: uuid::Uuid::new_v4().to_string(),
            name: Path::new(&file.path)
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
            source: Some(file.path),
            missing: false,
        });
    }
    write_json(root, ".humaan/builder/robots.json", &state)?;
    Ok(state)
}

#[tauri::command]
pub async fn builder_project(root: String) -> Result<ProjectRobots, String> {
    tauri::async_runtime::spawn_blocking(move || discover(&project(&root)?))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn builder_add_robot(root: String, name: String) -> Result<ProjectRobot, String> {
    tauri::async_runtime::spawn_blocking(move || create_robot(&project(&root)?, &name))
        .await
        .map_err(|e| e.to_string())?
}

fn create_robot(root: &Path, name: &str) -> Result<ProjectRobot, String> {
    let name = folder_name(name)?;
    let escaped = name.replace('&', "&amp;").replace('\'', "&apos;");
    let xml = format!("<?xml version=\"1.0\"?>\n<robot name='{escaped}'>\n  <link name=\"base_link\"/>\n</robot>\n");
    let state = import_files(
        root,
        name,
        vec![ImportFile {
            path: "robot.urdf".into(),
            data: xml.into_bytes(),
        }],
    )?;
    let source = format!("robots/{name}/robot.urdf");
    state
        .robots
        .into_iter()
        .find(|entry| entry.source.as_deref() == Some(&source))
        .ok_or("Created robot was not discovered.".into())
}

#[tauri::command]
pub async fn builder_import_robot(
    root: String,
    name: String,
    files: Vec<ImportFile>,
) -> Result<ProjectRobots, String> {
    tauri::async_runtime::spawn_blocking(move || import_files(&project(&root)?, &name, files))
        .await
        .map_err(|e| e.to_string())?
}

fn draft_path(root: &Path, id: &str) -> Result<String, String> {
    if id != "legacy" && !read_registry(root)?.robots.iter().any(|r| r.id == id) {
        return Err("Unknown project robot.".into());
    }
    if id != "legacy" && uuid::Uuid::parse_str(id).is_err() {
        return Err("Invalid robot ID.".into());
    }
    Ok(format!(".humaan/builder/robots/{id}/draft.json"))
}

#[tauri::command]
pub async fn builder_draft_load(root: String, id: String) -> Result<Option<Value>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let root = project(&root)?;
        let path = within(&root, &draft_path(&root, &id)?)?;
        if !path.exists() {
            return Ok(None);
        }
        serde_json::from_slice(&fs::read(path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn builder_draft_save(root: String, id: String, draft: Value) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let root = project(&root)?;
        write_json(&root, &draft_path(&root, &id)?, &draft)?;
        let mut state = read_registry(&root)?;
        if let Some(entry) = state.robots.iter_mut().find(|entry| entry.id == id) {
            if let Some(name) = draft["robot"]["name"]
                .as_str()
                .filter(|name| !name.trim().is_empty())
            {
                if entry.name != name {
                    entry.name = name.to_owned();
                    write_json(&root, ".humaan/builder/robots.json", &state)?;
                }
            }
        }
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn builder_activate_robot(root: String, id: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let root = project(&root)?;
        draft_path(&root, &id)?;
        let mut state = read_registry(&root)?;
        state.active_robot_id = Some(id);
        write_json(&root, ".humaan/builder/robots.json", &state)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn builder_robot_files(root: String) -> Result<Vec<RobotFile>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let root = project(&root)?;
        let mut files = vec![];
        scan(&root, &within(&root, "robots")?, &mut files)?;
        Ok(files)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn builder_read_robot_file(root: String, path: String) -> Result<Vec<u8>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let root = project(&root)?;
        if !Path::new(&path).starts_with("robots") {
            return Err("Choose a file inside robots/.".into());
        }
        let path = within(&root, &path)?;
        if fs::metadata(&path).map_err(|e| e.to_string())?.len() > 64 * 1024 * 1024 {
            return Err("Robot file exceeds 64 MB.".into());
        }
        fs::read(path).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_source_urdf_and_refuses_to_overwrite_existing_robot() {
        let folder = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(folder.path()).unwrap();
        let robot = create_robot(&root, "arm & gripper").unwrap();
        assert_eq!(
            robot.source.as_deref(),
            Some("robots/arm & gripper/robot.urdf")
        );
        let source = fs::read_to_string(root.join(robot.source.unwrap())).unwrap();
        assert!(source.contains("arm &amp; gripper"));
        assert!(source.contains("<link name=\"base_link\"/>"));
        assert!(create_robot(&root, "arm & gripper")
            .unwrap_err()
            .contains("already exists"));
        assert_eq!(read_registry(&root).unwrap().robots.len(), 1);
        assert!(create_robot(&root, "../escape").is_err());
    }

    #[test]
    fn imports_nested_sources_and_assets_as_separate_robots() {
        let folder = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(folder.path()).unwrap();
        let files = vec![
            ImportFile {
                path: "arm.urdf".into(),
                data: b"<robot name=\"Arm\"><link name=\"base\"/></robot>".to_vec(),
            },
            ImportFile {
                path: "variants/camera.urdf".into(),
                data: b"<robot name=\"Camera\"><link name=\"base\"/></robot>".to_vec(),
            },
            ImportFile {
                path: "assets/mesh.stl".into(),
                data: vec![0, 128, 255],
            },
            ImportFile {
                path: "README.txt".into(),
                data: b"Source notes".to_vec(),
            },
        ];
        let state = import_files(&root, "SO101", files).unwrap();
        assert_eq!(state.robots.len(), 2);
        assert!(state
            .robots
            .iter()
            .any(|r| r.source.as_deref() == Some("robots/SO101/variants/camera.urdf")));
        assert_eq!(
            fs::read(root.join("robots/SO101/assets/mesh.stl")).unwrap(),
            vec![0, 128, 255]
        );
        assert!(root.join("robots/SO101/README.txt").exists());
        let original = fs::read(root.join("robots/SO101/arm.urdf")).unwrap();
        assert!(import_files(
            &root,
            "SO101",
            vec![ImportFile {
                path: "arm.urdf".into(),
                data: b"overwrite".to_vec()
            }]
        )
        .is_err());
        assert_eq!(
            fs::read(root.join("robots/SO101/arm.urdf")).unwrap(),
            original
        );
        assert_eq!(fs::read_dir(root.join("robots")).unwrap().count(), 1);
    }

    #[test]
    fn rejects_escaping_and_conflicting_import_paths_before_creating_destination() {
        let folder = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(folder.path()).unwrap();
        for paths in [
            vec!["../escape.urdf"],
            vec!["/absolute.urdf"],
            vec!["a/../../escape.urdf"],
            vec!["Arm.urdf", "arm.urdf"],
            vec!["robot.urdf", "robot.urdf/asset"],
        ] {
            let files = paths
                .into_iter()
                .map(|path| ImportFile {
                    path: path.into(),
                    data: vec![1],
                })
                .collect();
            assert!(import_files(&root, "test", files).is_err());
            assert!(!root.join("robots/test").exists());
        }
    }

    #[test]
    fn discovery_retains_ids_and_tracks_missing_sources() {
        let folder = tempfile::tempdir().unwrap();
        let canonical = fs::canonicalize(folder.path()).unwrap();
        let root = canonical.as_path();
        fs::create_dir_all(root.join("robots/arm")).unwrap();
        fs::write(root.join("robots/arm/one.urdf"), "<robot name=\"same\"/>").unwrap();
        fs::write(root.join("robots/arm/two.urdf"), "<robot name=\"same\"/>").unwrap();
        let first = discover(root).unwrap();
        assert_eq!(first.robots.len(), 2);
        let id = first.robots[0].id.clone();
        write_json(
            root,
            &draft_path(root, &id).unwrap(),
            &serde_json::json!({"name":"Draft"}),
        )
        .unwrap();
        fs::remove_file(root.join("robots/arm/one.urdf")).unwrap();
        let next = discover(root).unwrap();
        assert_eq!(next.robots[0].id, id);
        assert!(next.robots[0].missing);
        assert!(!next.robots[1].missing);
        assert!(within(root, "../escape").is_err());
        assert!(draft_path(root, "../escape").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn rejects_paths_and_storage_symlinks_outside_project() {
        let folder = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(folder.path()).unwrap();
        std::os::unix::fs::symlink(outside.path(), root.join("robots")).unwrap();
        assert!(within(&root, "robots/mesh.stl").is_err());
        std::os::unix::fs::symlink(outside.path(), root.join(".humaan")).unwrap();
        assert!(discover(&root).is_err());
    }
}
