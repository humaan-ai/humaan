//! Optional LeRobot preparation through an isolated Python environment.
use crate::{Result, invalid, learning_sets};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparationConfig {
    pub version: u32,
    pub approve_whole_trajectories: bool,
    pub train: Vec<String>,
    pub validation: Vec<String>,
}

/// The helper is embedded so packaged desktops do not depend on a source checkout.
/// Uses app-managed tools by default, with an optional custom interpreter override.
pub fn prepare(
    start: &Path,
    id: &str,
    parent: &Path,
    config: PreparationConfig,
    environment: &Path,
) -> Result<PathBuf> {
    crate::validate_id(id)?;
    if config.version != 1
        || !config.approve_whole_trajectories
        || config.train.is_empty()
        || config.validation.is_empty()
    {
        return Err(invalid(
            "Assign at least one episode to each split and confirm whole-trajectory suitability.",
        ));
    }
    let python = match std::env::var_os("HUMAAN_DATASET_PYTHON") {
        Some(path) => PathBuf::from(path),
        None => crate::dataset_environment::managed_python(environment)
            .map_err(|_| invalid("Set up dataset tools in the preparation panel first."))?,
    };
    crate::dataset_environment::probe(&python, &environment.join("hf-cache"))?;
    let destination = parent.join(format!("prepared-{id}"));
    if fs::symlink_metadata(&destination).is_ok() {
        return Err(invalid("Prepared dataset destination already exists."));
    }
    let temporary = tempfile::Builder::new()
        .prefix(".humaan-preparation-")
        .tempdir_in(parent)?;
    // Export uses the pinned manifests and verifies all recordings before Python
    // receives the portable bundle. No source paths are interpolated into code.
    let bundle = learning_sets::export_files(start, id, temporary.path())?;
    let script = temporary.path().join("prepare.py");
    fs::write(
        &script,
        include_str!("../../../tools/dataset-preparation/prepare.py"),
    )?;
    let configuration = temporary.path().join("configuration.json");
    fs::write(&configuration, serde_json::to_vec_pretty(&config)?)?;
    let result = Command::new(python)
        .arg(script)
        .arg("--bundle")
        .arg(bundle)
        .arg("--config")
        .arg(configuration)
        .arg("--output")
        .arg(&destination)
        .env("HF_HUB_OFFLINE", "1")
        .env("HF_DATASETS_OFFLINE", "1")
        .env("HF_HOME", temporary.path().join("hf-cache"))
        .output()
        .map_err(|e| {
            invalid(format!(
                "Could not start the dataset preparation environment: {e}"
            ))
        })?;
    if !result.status.success() {
        return Err(invalid(format!(
            "Dataset preparation failed: {}",
            String::from_utf8_lossy(&result.stderr)
        )));
    }
    if !destination.join("preparation-report.json").is_file() {
        return Err(invalid("Preparation did not produce a validation report."));
    }
    Ok(destination)
}
