//! Local project identity and discovery, independent of cloud workspaces.
use crate::{Result, invalid};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Project {
    pub version: u32,
    pub id: String,
    pub name: String,
}

/// Initialize the current directory, preserving any existing recordings.
pub fn init(root: &Path, name: &str) -> Result<Project> {
    if name.trim().is_empty() || name.chars().any(char::is_control) {
        return Err(invalid(
            "project name must be nonempty and contain no control characters",
        ));
    }
    let directory = root.join(".humaan");
    fs::create_dir_all(&directory)?;
    let project = Project {
        version: 1,
        id: uuid::Uuid::new_v4().to_string(),
        name: name.trim().to_owned(),
    };
    let mut file = tempfile::NamedTempFile::new_in(&directory)?;
    serde_json::to_writer_pretty(&mut file, &project)?;
    file.write_all(b"\n")?;
    file.as_file().sync_all()?;
    file.persist_noclobber(directory.join("project.json"))
        .map_err(|error| {
            invalid(format!(
                "cannot initialize project (it may already exist): {error}"
            ))
        })?;
    Ok(project)
}

/// Find the nearest project, rejecting malformed metadata instead of skipping it.
pub fn discover(start: &Path) -> Result<Option<(PathBuf, Project)>> {
    for root in start.canonicalize()?.ancestors() {
        let path = root.join(".humaan/project.json");
        let bytes = match fs::read(path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error.into()),
        };
        let project: Project = serde_json::from_slice(&bytes)?;
        if project.version != 1
            || uuid::Uuid::parse_str(&project.id).is_err()
            || project.name.trim().is_empty()
            || project.name.chars().any(char::is_control)
        {
            return Err(invalid("invalid or unsupported project metadata"));
        }
        return Ok(Some((root.to_path_buf(), project)));
    }
    Ok(None)
}

pub fn store(start: &Path) -> Result<PathBuf> {
    Ok(discover(start)?
        .map(|(root, _)| root)
        .unwrap_or(start.to_path_buf())
        .join(".humaan/recordings"))
}
