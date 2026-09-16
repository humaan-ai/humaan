//! Human reviews are versioned separately from immutable recording artifacts.
use crate::{Result, invalid, regular_file, validate_id};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Write,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

pub const FILE: &str = "annotations.json";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Success,
    Failure,
    Timeout,
    Aborted,
    Unknown,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Assistance {
    Yes,
    No,
    Unknown,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewState {
    Unreviewed,
    Reviewed,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewInput {
    pub outcome: Outcome,
    pub assistance: Assistance,
    pub notes: String,
    pub reviewer: String,
    pub review_state: ReviewState,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Revision {
    pub revision: u64,
    pub saved_at_unix_ms: u64,
    pub source: String,
    pub review: ReviewInput,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Annotations {
    pub version: u32,
    pub experience_id: String,
    pub revisions: Vec<Revision>,
}
fn validate_input(review: &ReviewInput) -> Result<()> {
    if review.reviewer.trim().is_empty() || review.reviewer.len() > 200 {
        return Err(invalid("Reviewer must contain 1–200 bytes."));
    }
    if review.notes.len() > 10000 {
        return Err(invalid("Notes must not exceed 10,000 bytes."));
    }
    Ok(())
}
/// Read small review metadata without scanning recording artifacts.
pub fn read(directory: &Path, id: &str) -> Result<Annotations> {
    validate_id(id)?;
    match fs::symlink_metadata(directory.join(FILE)) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Annotations {
                version: 1,
                experience_id: id.into(),
                revisions: vec![],
            });
        }
        Err(e) => return Err(e.into()),
        Ok(_) => {}
    }
    let annotations: Annotations =
        serde_json::from_slice(&fs::read(regular_file(directory, FILE)?)?)?;
    if annotations.version != 1 || annotations.experience_id != id {
        return Err(invalid("Unsupported or mismatched annotation metadata."));
    }
    for (index, revision) in annotations.revisions.iter().enumerate() {
        if revision.revision != index as u64 + 1 || revision.source != "human" {
            return Err(invalid("Invalid annotation revision history."));
        }
        validate_input(&revision.review)?;
    }
    Ok(annotations)
}
/// Serialize writers and reject stale edits, then atomically publish the full history.
pub fn save(
    store: &Path,
    id: &str,
    expected_revision: u64,
    review: ReviewInput,
) -> Result<Annotations> {
    validate_id(id)?;
    validate_input(&review)?;
    let directory = store.join(id);
    regular_file(&directory, "manifest.json")?;
    let lock_path = directory.join(".annotation-lock");
    // Keep the file in place: deleting it would let writers lock different inodes.
    if fs::symlink_metadata(&lock_path).is_ok() {
        regular_file(&directory, ".annotation-lock")?;
    }
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock_path)
        .map_err(|e| invalid(format!("Cannot open annotation lock: {e}")))?;
    lock.try_lock()
        .map_err(|e| invalid(format!("Cannot lock annotation for writing: {e}")))?;
    // Closing this handle releases the OS lock, including on process termination.
    let _lock = lock;
    let manifest: crate::Manifest =
        serde_json::from_slice(&fs::read(regular_file(&directory, "manifest.json")?)?)?;
    if manifest.id != id
        || manifest.version != 1
        || manifest.artifacts.iter().any(|a| {
            a.path
                .split('/')
                .next()
                .is_some_and(|p| p.eq_ignore_ascii_case(FILE))
        })
    {
        return Err(invalid(
            "Invalid recording or annotation filename conflicts with an artifact.",
        ));
    }
    let mut annotations = read(&directory, id)?;
    if annotations.revisions.len() as u64 != expected_revision {
        return Err(invalid(
            "This review changed since it was opened. Refresh and reopen the experience before saving.",
        ));
    }
    annotations.revisions.push(Revision {
        revision: expected_revision + 1,
        saved_at_unix_ms: SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_millis()
            .try_into()?,
        source: "human".into(),
        review,
    });
    let mut temporary = tempfile::NamedTempFile::new_in(&directory)?;
    temporary.write_all(&serde_json::to_vec_pretty(&annotations)?)?;
    temporary.as_file().sync_all()?;
    temporary.persist(directory.join(FILE))?;
    Ok(annotations)
}
