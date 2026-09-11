//! Offline operations on versioned recording directories.
pub mod annotations;
mod lerobot;
pub mod project;
#[cfg(feature = "rerun")]
pub mod replay;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    fs,
    io::{self, Read},
    path::{Component, Path},
};

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub version: u32,
    pub id: String,
    pub robot: String,
    pub task: String,
    pub artifacts: Vec<Artifact>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Artifact {
    pub path: String,
    pub format: String,
    pub bytes: u64,
    pub sha256: String,
}

fn invalid(message: impl Into<String>) -> Box<dyn std::error::Error> {
    io::Error::new(io::ErrorKind::InvalidData, message.into()).into()
}

fn validate_id(id: &str) -> Result<()> {
    if id.is_empty()
        || id.len() > 128
        || !id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
    {
        return Err(invalid(
            "experience id must contain 1–128 ASCII letters, digits, hyphens, or underscores",
        ));
    }
    Ok(())
}

fn regular_file(root: &Path, relative: &str) -> Result<std::path::PathBuf> {
    let mut path = root.to_path_buf();
    for component in Path::new(relative).components() {
        if !matches!(component, Component::Normal(_)) {
            return Err(invalid(
                "artifact paths must be relative and cannot contain traversal",
            ));
        }
        path.push(component);
        if fs::symlink_metadata(&path)?.file_type().is_symlink() {
            return Err(invalid("recordings cannot contain symbolic links"));
        }
    }
    if !path.is_file() {
        return Err(invalid("expected a regular recording file"));
    }
    Ok(path)
}

/// Read and verify every declared artifact before using a recording.
pub fn inspect(directory: &Path) -> Result<Manifest> {
    let manifest: Manifest =
        serde_json::from_slice(&fs::read(regular_file(directory, "manifest.json")?)?)?;
    validate_id(&manifest.id)?;
    if manifest.version != 1 {
        return Err(invalid("unsupported recording version (expected 1)"));
    }
    if manifest.robot.trim().is_empty()
        || manifest.task.trim().is_empty()
        || manifest.artifacts.is_empty()
    {
        return Err(invalid(
            "robot, task, and at least one artifact are required",
        ));
    }
    let mut paths = HashSet::new();
    for artifact in &manifest.artifacts {
        // Require portable, canonical paths, including when running on Unix.
        if artifact.path.is_empty()
            || artifact.path.contains('\\')
            || artifact.path.contains(':')
            || artifact
                .path
                .split('/')
                .any(|part| part.is_empty() || part == "." || part == "..")
            || artifact.path.eq_ignore_ascii_case("manifest.json")
            || !paths.insert(artifact.path.to_lowercase())
        {
            return Err(invalid(
                "artifact paths must be unique portable relative paths, excluding manifest.json",
            ));
        }
        if artifact.format.trim().is_empty() {
            return Err(invalid("artifact format is required"));
        }
        let path = regular_file(directory, &artifact.path)?;
        let mut file = fs::File::open(path)?;
        let mut digest = Sha256::new();
        let mut bytes = 0;
        let mut buffer = [0u8; 65536];
        loop {
            let count = file.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            digest.update(&buffer[..count]);
            bytes += count as u64;
        }
        if bytes != artifact.bytes || format!("{:x}", digest.finalize()) != artifact.sha256 {
            return Err(invalid(format!(
                "integrity check failed for {}",
                artifact.path
            )));
        }
    }
    Ok(manifest)
}

/// Copy only declared files, verify the copy, then publish without overwriting.
fn copy_recording(source: &Path, destination: &Path) -> Result<Manifest> {
    let manifest = inspect(source)?;
    let parent = destination
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent)?;
    let staging = tempfile::Builder::new()
        .prefix(".humaan-")
        .tempdir_in(parent)?;
    for artifact in &manifest.artifacts {
        let target = staging.path().join(&artifact.path);
        fs::create_dir_all(target.parent().unwrap())?;
        fs::copy(regular_file(source, &artifact.path)?, target)?;
    }
    fs::copy(
        regular_file(source, "manifest.json")?,
        staging.path().join("manifest.json"),
    )?;
    // Reviews are mutable metadata; preserve their complete validated history.
    if !manifest.artifacts.iter().any(|a| {
        a.path
            .split('/')
            .next()
            .is_some_and(|p| p.eq_ignore_ascii_case(annotations::FILE))
    }) {
        let reviews = annotations::read(source, &manifest.id)?;
        if !reviews.revisions.is_empty() {
            fs::write(
                staging.path().join(annotations::FILE),
                serde_json::to_vec_pretty(&reviews)?,
            )?;
        }
    }
    let copied = inspect(staging.path())?;
    if copied.id != manifest.id {
        return Err(invalid("source manifest changed during copy"));
    }
    // Reserve the destination atomically. Never replace an existing recording.
    fs::create_dir(destination)?;
    if let Err(error) = fs::rename(staging.path(), destination) {
        let _ = fs::remove_dir(destination);
        return Err(error.into());
    }
    Ok(copied)
}

pub fn import(store: &Path, source: &Path) -> Result<Manifest> {
    let manifest = inspect(source)?;
    copy_recording(source, &store.join(&manifest.id))
}

/// Detect a native recording or a complete LeRobot v3.0 dataset.
pub fn import_recordings(store: &Path, source: &Path) -> Result<Vec<Manifest>> {
    if !source.is_dir() {
        return Err(invalid(format!(
            "import source is not a directory: {}",
            source.display()
        )));
    }
    if source.join("manifest.json").try_exists()? {
        return Ok(vec![import(store, source)?]);
    }
    if source.join("meta/info.json").try_exists()? {
        return lerobot::import(store, source);
    }
    Err(invalid(
        "unrecognized recording format: expected manifest.json (Humaan v1) or meta/info.json (LeRobot v3.0)",
    ))
}

/// Read adapter provenance only after validating the recording's artifacts.
pub fn lerobot_details(store: &Path, manifest: &Manifest) -> Result<Option<serde_json::Value>> {
    if let Some(artifact) = manifest
        .artifacts
        .iter()
        .find(|a| a.format == "application/vnd.humaan.lerobot-experience+json")
    {
        validate_id(&manifest.id)?;
        return Ok(Some(serde_json::from_slice(&fs::read(regular_file(
            &store.join(&manifest.id),
            &artifact.path,
        )?)?)?));
    }
    Ok(None)
}

pub fn inspect_experience(store: &Path, id: &str) -> Result<Manifest> {
    validate_id(id)?;
    let manifest = inspect(&store.join(id))?;
    if manifest.id != id {
        return Err(invalid("catalog directory and experience id disagree"));
    }
    Ok(manifest)
}

pub fn export(store: &Path, id: &str, destination: &Path) -> Result<Manifest> {
    inspect_experience(store, id)?;
    copy_recording(&store.join(id), destination)
}

/// The initial catalog is the collection of recording manifests on disk.
pub fn list(store: &Path) -> Result<Vec<Manifest>> {
    list_entries(store, true)
}

/// Read catalog metadata without opening artifacts. This does not establish integrity.
pub fn list_metadata(store: &Path) -> Result<Vec<Manifest>> {
    list_entries(store, false)
}

fn list_entries(store: &Path, verify_artifacts: bool) -> Result<Vec<Manifest>> {
    let entries = match fs::read_dir(store) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error.into()),
    };
    let mut experiences = Vec::new();
    for entry in entries {
        let entry = entry?;
        let name = entry.file_name();
        let id = name
            .to_str()
            .ok_or_else(|| invalid("non-UTF-8 catalog entry"))?;
        if id.starts_with('.') {
            continue;
        }
        if !entry.file_type()?.is_dir() {
            return Err(invalid("unexpected entry in recording store"));
        }
        if verify_artifacts {
            experiences.push(inspect_experience(store, id)?);
        } else {
            validate_id(id)?;
            let manifest: Manifest = serde_json::from_slice(&fs::read(regular_file(
                &store.join(id),
                "manifest.json",
            )?)?)?;
            if manifest.version != 1
                || manifest.id != id
                || manifest.robot.trim().is_empty()
                || manifest.task.trim().is_empty()
                || manifest.artifacts.is_empty()
            {
                return Err(invalid(format!("invalid catalog metadata for {id}")));
            }
            experiences.push(manifest);
        }
    }
    experiences.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(experiences)
}

/// Integrity result for one catalog entry. A damaged entry does not hide others.
#[derive(Debug, Serialize)]
pub struct Verification {
    pub id: String,
    pub valid: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

pub fn verify(store: &Path) -> Result<Vec<Verification>> {
    let entries = match fs::read_dir(store) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error.into()),
    };
    let mut results = Vec::new();
    for entry in entries {
        let entry = entry?;
        let name = entry.file_name();
        if name.to_string_lossy().starts_with('.') {
            continue;
        }
        let result = (|| -> Result<()> {
            let id = name
                .to_str()
                .ok_or_else(|| invalid("non-UTF-8 catalog entry"))?;
            if !entry.file_type()?.is_dir() {
                return Err(invalid("unexpected entry in recording store"));
            }
            inspect_experience(store, id)?;
            Ok(())
        })();
        results.push(Verification {
            id: name.to_string_lossy().into_owned(),
            valid: result.is_ok(),
            error: result.err().map(|error| error.to_string()),
        });
    }
    results.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(results)
}
