//! Small native recording generated independently of the import/export code.
use std::{
    fs,
    path::{Path, PathBuf},
};

/// Create an isolated source inside the caller's temporary directory.
pub fn recording(parent: &Path) -> PathBuf {
    let source = parent.join("fixture");
    fs::create_dir(&source).unwrap();
    let payload = b"{}\n";
    fs::write(source.join("state.jsonl"), payload).unwrap();
    let manifest = serde_json::json!({
        "version": 1,
        "id": "example-001",
        "robot": "test-robot",
        "task": "test-task",
        "artifacts": [{
            "path": "state.jsonl",
            "format": "application/x-ndjson",
            "bytes": payload.len(),
            // SHA-256 of the fixed payload above, independent of engine hashing.
            "sha256": "ca3d163bab055381827226140568f3bef7eaac187cebd76878e0b63e9e442356"
        }]
    });
    fs::write(
        source.join("manifest.json"),
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();
    source
}
