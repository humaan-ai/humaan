//! Immutable, local selections with recording and review provenance.
use crate::{Manifest, Result, annotations, invalid, project, regular_file, validate_id};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    fs,
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Selection {
    pub experience_id: String,
    pub review_revision: u64,
    pub reason: String,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateSet {
    pub name: String,
    pub learning_goal: String,
    pub curator: String,
    pub selections: Vec<Selection>,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Member {
    pub experience_id: String,
    pub reason: String,
    pub manifest_sha256: String,
    pub manifest: Manifest,
    pub review: annotations::Revision,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LearningSet {
    pub version: u32,
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub learning_goal: String,
    pub curator: String,
    pub created_at_unix_ms: u64,
    pub members: Vec<Member>,
}
fn context(start: &Path) -> Result<(PathBuf, project::Project)> {
    project::discover(start)?.ok_or_else(|| invalid("Open a Humaan project first."))
}
fn field(value: &str, label: &str, limit: usize) -> Result<()> {
    if value.trim().is_empty() || value.len() > limit {
        return Err(invalid(format!(
            "{label} is required and must fit within {limit} UTF-8 bytes."
        )));
    }
    Ok(())
}
/// Verify every selection before publishing anything. Later review edits do not
/// silently change the evidence used to curate this set.
pub fn create(start: &Path, input: CreateSet) -> Result<LearningSet> {
    let (root, project) = context(start)?;
    field(&input.name, "Name", 200)?;
    field(&input.learning_goal, "Learning goal", 4000)?;
    field(&input.curator, "Curator", 200)?;
    if input.selections.is_empty() {
        return Err(invalid("Select at least one reviewed experience."));
    }
    let store = root.join(".humaan/recordings");
    let mut ids = HashSet::new();
    let mut members = Vec::new();
    for selection in input.selections {
        validate_id(&selection.experience_id)?;
        if !ids.insert(selection.experience_id.clone()) {
            return Err(invalid(
                "An experience may appear only once in a learning set.",
            ));
        }
        field(&selection.reason, "Selection reason", 4000)?;
        let manifest = crate::inspect_experience(&store, &selection.experience_id)?;
        let directory = store.join(&selection.experience_id);
        let bytes = fs::read(regular_file(&directory, "manifest.json")?)?;
        // Bind the fingerprint to the exact manifest whose artifacts were verified.
        let fingerprint_manifest: Manifest = serde_json::from_slice(&bytes)?;
        if serde_json::to_value(&manifest)? != serde_json::to_value(&fingerprint_manifest)? {
            return Err(invalid(
                "Recording changed during selection. Retry with stable recordings.",
            ));
        }
        let history = annotations::read(&directory, &selection.experience_id)?;
        let review = history
            .revisions
            .last()
            .ok_or_else(|| invalid("Review every selected experience before adding it."))?;
        if review.revision != selection.review_revision {
            return Err(invalid(
                "A selected review changed. Refresh the queue and select it again.",
            ));
        }
        if review.review.review_state != annotations::ReviewState::Reviewed {
            return Err(invalid(
                "Only reviewed experiences can enter a learning set.",
            ));
        }
        members.push(Member {
            experience_id: selection.experience_id,
            reason: selection.reason.trim().into(),
            manifest_sha256: hex::encode(Sha256::digest(&bytes)),
            manifest,
            review: review.clone(),
        });
    }
    let set = LearningSet {
        version: 1,
        id: uuid::Uuid::new_v4().to_string(),
        project_id: project.id,
        name: input.name.trim().into(),
        learning_goal: input.learning_goal.trim().into(),
        curator: input.curator.trim().into(),
        created_at_unix_ms: SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_millis()
            .try_into()?,
        members,
    };
    let directory = root.join(".humaan/learning-sets");
    fs::create_dir_all(&directory)?;
    write_new(
        &directory,
        &directory.join(format!("{}.json", set.id)),
        &set,
    )?;
    Ok(set)
}
fn write_new(parent: &Path, destination: &Path, set: &LearningSet) -> Result<()> {
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    serde_json::to_writer_pretty(&mut temporary, set)?;
    temporary.write_all(b"\n")?;
    temporary.as_file().sync_all()?;
    temporary.persist_noclobber(destination)?;
    Ok(())
}
pub fn list(start: &Path) -> Result<Vec<LearningSet>> {
    let (root, project) = context(start)?;
    let directory = root.join(".humaan/learning-sets");
    let entries = match fs::read_dir(&directory) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
        Err(e) => return Err(e.into()),
    };
    let mut sets = Vec::new();
    for entry in entries {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        let set: LearningSet =
            serde_json::from_slice(&fs::read(regular_file(&directory, &name)?)?)?;
        validate_id(&set.id)?;
        if set.version != 1
            || set.project_id != project.id
            || name != format!("{}.json", set.id)
            || set.members.is_empty()
        {
            return Err(invalid("Invalid learning set metadata."));
        }
        sets.push(set);
    }
    sets.sort_by(|a, b| {
        b.created_at_unix_ms
            .cmp(&a.created_at_unix_ms)
            .then(a.id.cmp(&b.id))
    });
    Ok(sets)
}
/// Export a selection manifest, not a reconstructed training dataset.
pub fn export(start: &Path, id: &str, parent: &Path) -> Result<PathBuf> {
    validate_id(id)?;
    let set = list(start)?
        .into_iter()
        .find(|s| s.id == id)
        .ok_or_else(|| invalid("Learning set not found."))?;
    let destination = parent.join(format!("learning-set-{}.json", set.id));
    write_new(parent, &destination, &set)?;
    Ok(destination)
}

/// Export all selected recordings as a single verified, portable directory.
/// The learning-set snapshot remains authoritative for curation labels; recordings
/// also carry their current annotation history through the ordinary export path.
pub fn export_files(start: &Path, id: &str, parent: &Path) -> Result<PathBuf> {
    validate_id(id)?;
    let (root, _) = context(start)?;
    let set = list(&root)?
        .into_iter()
        .find(|s| s.id == id)
        .ok_or_else(|| invalid("Learning set not found."))?;
    let destination = parent.join(format!("learning-set-{}", set.id));
    if fs::symlink_metadata(&destination).is_ok() {
        return Err(invalid(
            "Export destination already exists. Choose another folder.",
        ));
    }
    let staging = tempfile::Builder::new()
        .prefix(".humaan-learning-set-")
        .tempdir_in(parent)?;
    let recordings = staging.path().join("recordings");
    fs::create_dir(&recordings)?;
    let store = root.join(".humaan/recordings");
    let mut ids = HashSet::new();
    for member in &set.members {
        validate_id(&member.experience_id)?;
        if !ids.insert(&member.experience_id) {
            return Err(invalid("Duplicate experience in learning set."));
        }
        let source = store.join(&member.experience_id);
        check_snapshot(&source, member)?;
        let target = recordings.join(&member.experience_id);
        crate::export(&store, &member.experience_id, &target)
            .map_err(|e| invalid(format!("Cannot export {}: {e}", member.experience_id)))?;
        // Check the published copy against the pinned selection, including races
        // where the source manifest changed after the initial snapshot check.
        check_snapshot(&target, member)?;
    }
    write_new(
        staging.path(),
        &staging.path().join("learning-set.json"),
        &set,
    )?;
    // Reserve atomically; a failed member never leaves a visible partial bundle.
    fs::create_dir(&destination)?;
    if let Err(error) = fs::rename(staging.path(), &destination) {
        let _ = fs::remove_dir(&destination);
        return Err(error.into());
    }
    Ok(destination)
}

fn check_snapshot(directory: &Path, member: &Member) -> Result<()> {
    let bytes = fs::read(regular_file(directory, "manifest.json")?)?;
    let manifest: Manifest = serde_json::from_slice(&bytes)?;
    if manifest.id != member.experience_id
        || hex::encode(Sha256::digest(&bytes)) != member.manifest_sha256
        || serde_json::to_value(&manifest)? != serde_json::to_value(&member.manifest)?
    {
        return Err(invalid(format!(
            "Recording {} no longer matches the saved learning set. Restore the selected recording or curate a new set.",
            member.experience_id
        )));
    }
    Ok(())
}
