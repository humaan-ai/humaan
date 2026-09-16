//! Local verification history, separate from immutable recordings and CLI verification.
use crate::{Result, invalid, regular_file, validate_id};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

const FILE: &str = ".verification.json";
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Report {
    pub id: String,
    pub valid: bool,
    pub error: Option<String>,
    pub checked_at_unix_ms: u64,
    pub status: String,
    #[serde(default)]
    fingerprint: String,
}
#[derive(Serialize, Deserialize)]
struct Cache {
    version: u32,
    reports: BTreeMap<String, Report>,
}

// Fingerprint file metadata, not file contents. This detects ordinary external edits
// cheaply; it is not a substitute for rehashing files with Verify all.
fn fingerprint(store: &Path, id: &str) -> Result<String> {
    validate_id(id)?;
    let directory = store.join(id);
    if fs::symlink_metadata(&directory)?.file_type().is_symlink() {
        return Err(invalid("Recording is a symbolic link"));
    }
    let manifest_path = regular_file(&directory, "manifest.json")?;
    let bytes = fs::read(manifest_path)?;
    let mut hash = Sha256::new();
    hash.update(&bytes);
    let manifest: crate::Manifest = serde_json::from_slice(&bytes)?;
    for artifact in manifest.artifacts {
        hash.update(artifact.path.as_bytes());
        match regular_file(&directory, &artifact.path).and_then(|path| Ok(fs::metadata(path)?)) {
            Ok(metadata) => {
                hash.update(metadata.len().to_le_bytes());
                hash.update(
                    metadata
                        .modified()?
                        .duration_since(UNIX_EPOCH)?
                        .as_nanos()
                        .to_le_bytes(),
                );
            }
            Err(error) => hash.update(error.to_string().as_bytes()),
        }
    }
    Ok(hex::encode(hash.finalize()))
}
fn stamp(store: &Path, id: &str) -> String {
    fingerprint(store, id).unwrap_or_else(|error| format!("unavailable:{error}"))
}

pub fn read(store: &Path) -> Result<BTreeMap<String, Report>> {
    if !store.try_exists()? {
        return Ok(BTreeMap::new());
    }
    match fs::symlink_metadata(store.join(FILE)) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(BTreeMap::new()),
        Err(error) => return Err(error.into()),
        Ok(_) => (),
    }
    let cache: Cache = serde_json::from_slice(&fs::read(regular_file(store, FILE)?)?)?;
    if cache.version != 1 {
        return Err(invalid("Unsupported verification history version"));
    }
    let mut reports = cache.reports;
    for (id, report) in &mut reports {
        if report.id != *id {
            return Err(invalid("Verification history ID mismatch"));
        }
        if report.fingerprint != stamp(store, id) {
            report.status = "needs_recheck".into();
        }
    }
    Ok(reports)
}

pub fn verify_and_save(store: &Path) -> Result<Vec<Report>> {
    let mut before = BTreeMap::new();
    if store.try_exists()? {
        for entry in fs::read_dir(store)? {
            let id = entry?.file_name().to_string_lossy().into_owned();
            if !id.starts_with('.') {
                before.insert(id.clone(), stamp(store, &id));
            }
        }
    }
    let results = crate::verify(store)?;
    let checked_at_unix_ms = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64;
    let reports: BTreeMap<_, _> = results
        .into_iter()
        .map(|result| {
            let fingerprint = stamp(store, &result.id);
            let changed = before.get(&result.id) != Some(&fingerprint);
            let report = Report {
                id: result.id.clone(),
                valid: result.valid,
                error: result.error,
                checked_at_unix_ms,
                status: if changed {
                    "needs_recheck"
                } else if result.valid {
                    "verified"
                } else {
                    "failed"
                }
                .into(),
                fingerprint,
            };
            (result.id, report)
        })
        .collect();
    fs::create_dir_all(store)?;
    if fs::symlink_metadata(store.join(FILE)).is_ok() {
        regular_file(store, FILE)?;
    }
    let mut temporary = tempfile::NamedTempFile::new_in(store)?;
    temporary.write_all(&serde_json::to_vec_pretty(&Cache {
        version: 1,
        reports: reports.clone(),
    })?)?;
    temporary.as_file().sync_all()?;
    temporary.persist(store.join(FILE)).map_err(|error| {
        invalid(format!(
            "Verification finished but history could not be saved: {error}"
        ))
    })?;
    Ok(reports.into_values().collect())
}
