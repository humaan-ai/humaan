use serde::Serialize;
use std::{
    fs,
    path::{Component, Path},
};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    name: String,
    path: String,
    directory: bool,
    size: u64,
    kind: String,
}
#[derive(Serialize)]
pub struct Listing {
    entries: Vec<Entry>,
    truncated: bool,
}

fn asset_kind(path: &Path) -> Option<&'static str> {
    match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
        "urdf" | "xacro" | "mjcf" | "usd" | "usda" | "usdc" => Some("Robot / scene"),
        "stl" | "obj" | "dae" | "gltf" | "glb" | "ply" => Some("Mesh"),
        "png" | "jpg" | "jpeg" | "webp" | "exr" | "hdr" => Some("Image"),
        "mp4" | "webm" | "rrd" | "bag" | "mcap" | "parquet" => Some("Recording"),
        _ => None,
    }
}
fn system_metadata(name: &str) -> bool {
    name.starts_with("._")
        || matches!(
            name.to_ascii_lowercase().as_str(),
            ".ds_store"
                | ".appledouble"
                | ".lsoverride"
                | ".spotlight-v100"
                | ".trashes"
                | ".fseventsd"
                | "thumbs.db"
                | "ehthumbs.db"
                | "desktop.ini"
                | "$recycle.bin"
                | "system volume information"
        )
}
fn list(root: &str, directory: &str, assets: bool) -> Result<Listing, String> {
    let (root, _) = humaan_engine::project::discover(Path::new(root))
        .map_err(|e| e.to_string())?
        .ok_or("Open a Humaan project first.")?;
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    if Path::new(directory)
        .components()
        .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err("Choose a folder inside the project.".into());
    }
    let folder = root
        .join(directory)
        .canonicalize()
        .map_err(|e| e.to_string())?;
    if !folder.starts_with(&root) {
        return Err("Folder is outside the project.".into());
    }
    let mut pending = vec![folder];
    let mut entries = Vec::new();
    let mut visited = 0;
    let mut truncated = false;
    'scan: while let Some(folder) = pending.pop() {
        for item in
            fs::read_dir(&folder).map_err(|e| format!("Cannot read {}: {e}", folder.display()))?
        {
            visited += 1;
            if visited > 20_000 {
                truncated = true;
                break 'scan;
            }
            let item = item.map_err(|e| e.to_string())?;
            let ty = item.file_type().map_err(|e| e.to_string())?;
            // Never traverse symlinks or read their targets outside the project.
            if ty.is_symlink() || (!ty.is_dir() && !ty.is_file()) {
                continue;
            }
            let path = item.path();
            let name = item.file_name().to_string_lossy().into_owned();
            if system_metadata(&name) {
                continue;
            }
            if assets && ty.is_dir() {
                if !matches!(name.as_str(), ".git" | "node_modules" | "target" | ".venv") {
                    pending.push(path);
                }
                continue;
            }
            let kind = asset_kind(&path);
            if assets && kind.is_none() {
                continue;
            }
            entries.push(Entry {
                name,
                path: path
                    .strip_prefix(&root)
                    .map_err(|e| e.to_string())?
                    .to_string_lossy()
                    .replace('\\', "/"),
                directory: ty.is_dir(),
                size: if ty.is_file() {
                    item.metadata().map_err(|e| e.to_string())?.len()
                } else {
                    0
                },
                kind: if ty.is_dir() {
                    "Folder"
                } else {
                    kind.unwrap_or("File")
                }
                .into(),
            });
        }
    }
    entries.sort_by(|a, b| {
        b.directory
            .cmp(&a.directory)
            .then(a.path.to_lowercase().cmp(&b.path.to_lowercase()))
    });
    Ok(Listing { entries, truncated })
}
#[tauri::command]
pub async fn project_entries(
    root: String,
    directory: String,
    assets: bool,
) -> Result<Listing, String> {
    tauri::async_runtime::spawn_blocking(move || list(&root, &directory, assets))
        .await
        .map_err(|e| e.to_string())?
}

const TEXT_LIMIT: u64 = 2 * 1024 * 1024;
fn file_path(root: &str, path: &str) -> Result<std::path::PathBuf, String> {
    if path.is_empty()
        || Path::new(path)
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err("Choose a file inside the project.".into());
    }
    let (root, _) = humaan_engine::project::discover(Path::new(root))
        .map_err(|e| e.to_string())?
        .ok_or("Open a Humaan project first.")?;
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    let mut target = root.clone();
    for part in Path::new(path).components() {
        target.push(part);
        if fs::symlink_metadata(&target)
            .map_err(|e| e.to_string())?
            .file_type()
            .is_symlink()
        {
            return Err("Symbolic links cannot be opened in the editor.".into());
        }
    }
    if !target.is_file() {
        return Err("Select a regular file.".into());
    }
    Ok(target)
}
fn read_text(path: &Path) -> Result<String, String> {
    use std::io::Read;
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(TEXT_LIMIT + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > TEXT_LIMIT {
        return Err("This file exceeds the 2 MB text editor limit.".into());
    }
    if bytes.contains(&0) {
        return Err("This is a binary file. A text editor cannot display it.".into());
    }
    String::from_utf8(bytes)
        .map_err(|_| "This file is not UTF-8 text. Binary files need a dedicated viewer.".into())
}
fn managed(path: &str) -> bool {
    Path::new(path).components().any(|c| {
        let name = c.as_os_str().to_string_lossy();
        name.eq_ignore_ascii_case(".humaan") || name.eq_ignore_ascii_case(".git")
    })
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TextFile {
    content: String,
    read_only: bool,
}
#[tauri::command]
pub async fn project_read_file(root: String, path: String) -> Result<TextFile, String> {
    tauri::async_runtime::spawn_blocking(move || {
        Ok(TextFile {
            content: read_text(&file_path(&root, &path)?)?,
            read_only: managed(&path),
        })
    })
    .await
    .map_err(|e| e.to_string())?
}
fn save_text(root: &str, path: &str, content: &str, expected: &str) -> Result<(), String> {
    use std::io::Write;
    if managed(path) {
        return Err("Project-managed files are read-only.".into());
    }
    if content.len() as u64 > TEXT_LIMIT || content.contains('\0') {
        return Err("Only UTF-8 text up to 2 MB can be saved.".into());
    }
    let target = file_path(root, path)?;
    if read_text(&target)? != expected {
        return Err("The file changed on disk. Your edits are preserved. Close and reopen the file to load the latest version before saving.".into());
    }
    let permissions = fs::metadata(&target)
        .map_err(|e| e.to_string())?
        .permissions();
    if permissions.readonly() {
        return Err("This file is read-only on disk.".into());
    }
    let mut temp = tempfile::NamedTempFile::new_in(target.parent().ok_or("Invalid file path")?)
        .map_err(|e| e.to_string())?;
    temp.write_all(content.as_bytes())
        .map_err(|e| e.to_string())?;
    temp.as_file()
        .set_permissions(permissions)
        .map_err(|e| e.to_string())?;
    temp.as_file().sync_all().map_err(|e| e.to_string())?;
    temp.persist(&target).map_err(|e| e.to_string())?;
    Ok(())
}
#[tauri::command]
pub async fn project_save_file(
    root: String,
    path: String,
    content: String,
    expected: String,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || save_text(&root, &path, &content, &expected))
        .await
        .map_err(|e| e.to_string())?
}

fn delete_file(root: &str, path: &str) -> Result<(), String> {
    if managed(path) {
        return Err("Project-managed files cannot be deleted from the file explorer.".into());
    }
    // Use the same project boundary and symlink checks as the editor. Only
    // regular files can be deleted; this command never removes directories.
    let target = file_path(root, path)?;
    fs::remove_file(target).map_err(|e| e.to_string())
}
#[tauri::command]
pub async fn project_delete_file(root: String, path: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || delete_file(&root, &path))
        .await
        .map_err(|e| e.to_string())?
}

fn create_entry(root: &str, name: &str, directory: bool) -> Result<(), String> {
    if name.is_empty()
        || name.trim() != name
        || name == "."
        || name == ".."
        || name.ends_with('.')
        || name
            .chars()
            .any(|c| c.is_control() || "<>:\"/\\|?*".contains(c))
        || managed(name)
    {
        return Err(
            "Enter a file or folder name without path separators or reserved characters.".into(),
        );
    }
    let (root, _) = humaan_engine::project::discover(Path::new(root))
        .map_err(|e| e.to_string())?
        .ok_or("Open a Humaan project first.")?;
    let target = root.canonicalize().map_err(|e| e.to_string())?.join(name);
    let result = if directory {
        fs::create_dir(target)
    } else {
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(target)
            .map(|_| ())
    };
    result.map_err(|e| {
        if e.kind() == std::io::ErrorKind::AlreadyExists {
            "A file or folder with this name already exists.".into()
        } else {
            e.to_string()
        }
    })
}
#[tauri::command]
pub async fn project_create_entry(
    root: String,
    name: String,
    directory: bool,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || create_entry(&root, &name, directory))
        .await
        .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hides_system_metadata_but_keeps_project_dotfiles() {
        let temp = tempfile::tempdir().unwrap();
        humaan_engine::project::init(temp.path(), "Metadata").unwrap();
        let root = temp.path().to_str().unwrap();
        fs::create_dir(temp.path().join("meshes")).unwrap();
        for name in [".DS_Store", "._base.stl", "Thumbs.db", "desktop.ini"] {
            fs::write(temp.path().join(name), "metadata").unwrap();
            fs::write(temp.path().join("meshes").join(name), "metadata").unwrap();
        }
        for name in [".Spotlight-V100", "$RECYCLE.BIN", ".AppleDouble"] {
            fs::create_dir(temp.path().join(name)).unwrap();
            fs::write(temp.path().join(name).join("cached.stl"), "metadata").unwrap();
        }
        for name in [".gitignore", ".env", "meshes/base.stl"] {
            fs::write(temp.path().join(name), "project file").unwrap();
        }
        let files = list(root, "", false).unwrap();
        assert!(files
            .entries
            .iter()
            .all(|entry| !system_metadata(&entry.name)));
        for name in [".gitignore", ".env", ".humaan"] {
            assert!(files.entries.iter().any(|entry| entry.name == name));
        }
        let nested = list(root, "meshes", false).unwrap();
        assert_eq!(nested.entries.len(), 1);
        assert_eq!(nested.entries[0].path, "meshes/base.stl");
        let assets = list(root, "", true).unwrap();
        assert_eq!(assets.entries.len(), 1);
        assert_eq!(assets.entries[0].path, "meshes/base.stl");
    }
    #[test]
    fn deletes_only_regular_unmanaged_files_inside_project() {
        let temp = tempfile::tempdir().unwrap();
        humaan_engine::project::init(temp.path(), "Delete").unwrap();
        let root = temp.path().to_str().unwrap();
        fs::create_dir(temp.path().join("meshes")).unwrap();
        fs::write(temp.path().join("meshes/base.stl"), [0, 1, 2]).unwrap();
        fs::write(temp.path().join("keep.txt"), "keep").unwrap();
        fs::create_dir(temp.path().join(".git")).unwrap();
        fs::write(temp.path().join(".git/config"), "keep").unwrap();
        delete_file(root, "meshes/base.stl").unwrap();
        assert!(!temp.path().join("meshes/base.stl").exists());
        let outside = tempfile::NamedTempFile::new().unwrap();
        let traversal = format!(
            "../{}",
            outside.path().file_name().unwrap().to_str().unwrap()
        );
        for path in [
            "",
            ".",
            "meshes",
            ".humaan/project.json",
            ".HUMAAN/project.json",
            ".git/config",
            ".GIT/config",
            "meshes/../keep.txt",
            &traversal,
            outside.path().to_str().unwrap(),
        ] {
            assert!(delete_file(root, path).is_err(), "must reject {path}");
        }
        assert!(outside.path().exists());
        assert!(temp.path().join(".humaan/project.json").exists());
        assert!(temp.path().join("meshes").is_dir());
        assert_eq!(
            fs::read_to_string(temp.path().join("keep.txt")).unwrap(),
            "keep"
        );
        assert_eq!(
            fs::read_to_string(temp.path().join(".git/config")).unwrap(),
            "keep"
        );
        assert!(delete_file(root, "meshes/base.stl").is_err());
        #[cfg(unix)]
        {
            let external = tempfile::tempdir().unwrap();
            fs::write(external.path().join("keep.txt"), "external").unwrap();
            std::os::unix::fs::symlink(external.path(), temp.path().join("outside")).unwrap();
            std::os::unix::fs::symlink(
                external.path().join("keep.txt"),
                temp.path().join("link.txt"),
            )
            .unwrap();
            std::os::unix::fs::symlink(
                temp.path().join("keep.txt"),
                temp.path().join("internal.txt"),
            )
            .unwrap();
            for path in ["outside/keep.txt", "link.txt", "internal.txt"] {
                assert!(delete_file(root, path).is_err());
            }
            assert_eq!(
                fs::read_to_string(external.path().join("keep.txt")).unwrap(),
                "external"
            );
            assert_eq!(
                fs::read_to_string(temp.path().join("keep.txt")).unwrap(),
                "keep"
            );
        }
    }
    #[test]
    fn creates_without_overwriting_or_escaping_project() {
        let temp = tempfile::tempdir().unwrap();
        humaan_engine::project::init(temp.path(), "Create").unwrap();
        let root = temp.path().to_str().unwrap();
        create_entry(root, "robot.urdf", false).unwrap();
        create_entry(root, "meshes", true).unwrap();
        assert!(temp.path().join("robot.urdf").is_file());
        assert!(temp.path().join("meshes").is_dir());
        fs::write(temp.path().join("robot.urdf"), "preserve").unwrap();
        assert!(create_entry(root, "robot.urdf", false).is_err());
        assert_eq!(
            fs::read_to_string(temp.path().join("robot.urdf")).unwrap(),
            "preserve"
        );
        assert!(create_entry(root, "../outside", false).is_err());
        assert!(create_entry(root, ".humaan", true).is_err());
    }
    #[test]
    fn edits_text_without_overwriting_external_changes() {
        let temp = tempfile::tempdir().unwrap();
        humaan_engine::project::init(temp.path(), "Editor").unwrap();
        let root = temp.path().to_str().unwrap();
        fs::write(temp.path().join("robot.urdf"), "<robot/>\r\n").unwrap();
        save_text(
            root,
            "robot.urdf",
            "<robot name='test'/>\r\n",
            "<robot/>\r\n",
        )
        .unwrap();
        assert_eq!(
            read_text(&file_path(root, "robot.urdf").unwrap()).unwrap(),
            "<robot name='test'/>\r\n"
        );
        assert!(save_text(root, "robot.urdf", "lost", "<robot/>").is_err());
        assert!(save_text(root, ".humaan/project.json", "lost", "").is_err());
        assert!(file_path(root, "../outside").is_err());
        fs::write(temp.path().join("binary"), [0, 1, 2]).unwrap();
        assert!(read_text(&temp.path().join("binary")).is_err());
    }
    #[test]
    fn browses_files_and_filters_assets_with_project_boundaries() {
        let temp = tempfile::tempdir().unwrap();
        humaan_engine::project::init(temp.path(), "Browser").unwrap();
        fs::create_dir(temp.path().join("meshes")).unwrap();
        fs::write(temp.path().join("meshes/base.stl"), "mesh").unwrap();
        fs::write(temp.path().join("notes.txt"), "notes").unwrap();
        let root = temp.path().to_str().unwrap();
        let files = list(root, "meshes", false).unwrap();
        assert_eq!(files.entries[0].path, "meshes/base.stl");
        assert_eq!(files.entries[0].size, 4);
        assert_eq!(list(root, "", true).unwrap().entries.len(), 1);
        assert!(list(root, "../", false).is_err());
        #[cfg(unix)]
        {
            let outside = tempfile::tempdir().unwrap();
            std::os::unix::fs::symlink(outside.path(), temp.path().join("outside")).unwrap();
            assert!(list(root, "outside", false).is_err());
            assert!(!list(root, "", false)
                .unwrap()
                .entries
                .iter()
                .any(|e| e.name == "outside"));
        }
    }
}
