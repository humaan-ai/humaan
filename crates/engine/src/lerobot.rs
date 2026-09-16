//! LeRobot v3.0 import. Original chunk files remain intact; episode metadata
//! records the ranges needed to interpret files containing multiple attempts.
use crate::{Artifact, Manifest, Result, invalid, regular_file};
use parquet::file::reader::{FileReader, SerializedFileReader};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Read,
    path::Path,
};

fn rows(root: &Path, relative: &str) -> Result<Vec<Value>> {
    let path = regular_file(root, relative)
        .map_err(|e| invalid(format!("LeRobot file {relative}: {e}")))?;
    let reader = SerializedFileReader::new(fs::File::open(path)?)
        .map_err(|e| invalid(format!("LeRobot Parquet {relative}: {e}")))?;
    reader
        .get_row_iter(None)?
        .map(|row| Ok(row?.to_json_value()))
        .collect()
}

fn number(value: &Value, key: &str) -> Result<u64> {
    value[key]
        .as_u64()
        .ok_or_else(|| invalid(format!("LeRobot: missing or invalid {key}")))
}

fn files(root: &Path, relative: &str, output: &mut Vec<String>) -> Result<()> {
    let path = root.join(relative);
    if fs::symlink_metadata(&path)?.file_type().is_symlink() {
        return Err(invalid("LeRobot metadata cannot contain symbolic links"));
    }
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| invalid("non-UTF-8 metadata path"))?;
        let child = format!("{relative}/{name}");
        if entry.file_type()?.is_dir() {
            files(root, &child, output)?;
        } else if child.ends_with(".parquet") {
            regular_file(root, &child)?;
            output.push(child);
        }
    }
    output.sort();
    Ok(())
}

fn chunk_path(template: &str, chunk: u64, file: u64, camera: &str) -> Result<String> {
    let path = template
        .replace("{chunk_index:03d}", &format!("{chunk:03}"))
        .replace("{file_index:03d}", &format!("{file:03}"))
        .replace("{video_key}", camera);
    if path.contains(['{', '}']) {
        return Err(invalid(format!(
            "unsupported LeRobot path template: {template}"
        )));
    }
    Ok(path)
}

fn checksum(path: &Path) -> Result<(u64, String)> {
    let mut file = fs::File::open(path)?;
    let mut hash = Sha256::new();
    let mut size = 0;
    let mut buffer = [0; 65536];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
        size += n as u64;
    }
    Ok((size, hex::encode(hash.finalize())))
}

struct Episode {
    index: u64,
    task: String,
    metadata: Value,
    paths: BTreeSet<String>,
}

pub fn import(store: &Path, source: &Path) -> Result<Vec<Manifest>> {
    let info: Value = serde_json::from_slice(&fs::read(regular_file(source, "meta/info.json")?)?)?;
    if info["codebase_version"] != "v3.0" {
        return Err(invalid(format!(
            "unsupported LeRobot dataset version {}; supported: v3.0",
            info["codebase_version"]
        )));
    }
    let fps = info["fps"]
        .as_f64()
        .filter(|v| v.is_finite() && *v > 0.0)
        .ok_or_else(|| invalid("LeRobot fps must be positive"))?;
    let robot = info["robot_type"]
        .as_str()
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| invalid("LeRobot robot_type is missing"))?;
    let features = info["features"]
        .as_object()
        .ok_or_else(|| invalid("LeRobot features are missing"))?;
    for key in [
        "timestamp",
        "episode_index",
        "frame_index",
        "index",
        "task_index",
    ] {
        if !features.contains_key(key) {
            return Err(invalid(format!("LeRobot feature {key} is missing")));
        }
    }
    let data_template = info["data_path"]
        .as_str()
        .ok_or_else(|| invalid("LeRobot data_path is missing"))?;
    let mut metadata_files = vec![];
    files(source, "meta/episodes", &mut metadata_files)?;
    let mut all_paths = BTreeSet::from([
        "meta/info.json".to_string(),
        "meta/tasks.parquet".to_string(),
    ]);
    all_paths.extend(metadata_files.iter().cloned());
    if source.join("meta/stats.json").exists() {
        all_paths.insert("meta/stats.json".into());
    }
    let tasks = rows(source, "meta/tasks.parquet")?;
    let task_indices: BTreeSet<u64> = tasks
        .iter()
        .map(|row| number(row, "task_index"))
        .collect::<Result<_>>()?;
    if tasks.len() as u64 != number(&info, "total_tasks")? || task_indices.len() != tasks.len() {
        return Err(invalid("LeRobot task count or indices are inconsistent"));
    }
    let mut episodes = BTreeMap::new();
    let mut data_cache = BTreeMap::new();
    let mut total_frames = 0u64;
    for metadata_file in &metadata_files {
        for row in rows(source, metadata_file)? {
            let index = number(&row, "episode_index")?;
            let length = number(&row, "length")?;
            let start = number(&row, "dataset_from_index")?;
            let end = number(&row, "dataset_to_index")?;
            if length == 0 || end.checked_sub(start) != Some(length) {
                return Err(invalid(format!(
                    "LeRobot episode {index}: invalid frame range"
                )));
            }
            let data = chunk_path(
                data_template,
                number(&row, "data/chunk_index")?,
                number(&row, "data/file_index")?,
                "",
            )?;
            if !data_cache.contains_key(&data) {
                data_cache.insert(data.clone(), rows(source, &data)?);
            }
            let selected: Vec<_> = data_cache[&data]
                .iter()
                .filter(|r| r["episode_index"].as_u64() == Some(index))
                .collect();
            if selected.len() as u64 != length {
                return Err(invalid(format!(
                    "LeRobot episode {index}: expected {length} frames in {data}, found {}",
                    selected.len()
                )));
            }
            let mut last_time = None;
            for (frame, record) in selected.iter().enumerate() {
                let time = record["timestamp"]
                    .as_f64()
                    .filter(|t| t.is_finite() && *t >= 0.0)
                    .ok_or_else(|| {
                        invalid(format!("LeRobot episode {index}: invalid timestamp"))
                    })?;
                if number(record, "frame_index")? != frame as u64
                    || number(record, "index")? != start + frame as u64
                    || last_time.is_some_and(|previous| time <= previous)
                    || !task_indices.contains(&number(record, "task_index")?)
                {
                    return Err(invalid(format!(
                        "LeRobot episode {index}: inconsistent frame indices, timestamps, or task"
                    )));
                }
                for key in features.keys().filter(|k| features[*k]["dtype"] != "video") {
                    if record.get(key).is_none() {
                        return Err(invalid(format!(
                            "LeRobot episode {index}: missing data feature {key}"
                        )));
                    }
                }
                last_time = Some(time);
            }
            let mut paths = BTreeSet::from([
                data.clone(),
                metadata_file.clone(),
                "meta/info.json".into(),
                "meta/tasks.parquet".into(),
            ]);
            if all_paths.contains("meta/stats.json") {
                paths.insert("meta/stats.json".into());
            }
            let mut videos = BTreeMap::new();
            for (camera, feature) in features {
                if feature["dtype"] != "video" {
                    continue;
                }
                let prefix = format!("videos/{camera}");
                let template = info["video_path"]
                    .as_str()
                    .ok_or_else(|| invalid("LeRobot video_path is missing"))?;
                let path = chunk_path(
                    template,
                    number(&row, &format!("{prefix}/chunk_index"))?,
                    number(&row, &format!("{prefix}/file_index"))?,
                    camera,
                )?;
                regular_file(source, &path).map_err(|e| {
                    invalid(format!(
                        "LeRobot episode {index}: missing or invalid video {path}: {e}"
                    ))
                })?;
                let from = row[format!("{prefix}/from_timestamp")]
                    .as_f64()
                    .unwrap_or(f64::NAN);
                let to = row[format!("{prefix}/to_timestamp")]
                    .as_f64()
                    .unwrap_or(f64::NAN);
                if !from.is_finite() || !to.is_finite() || from < 0.0 || to <= from {
                    return Err(invalid(format!(
                        "LeRobot episode {index}: invalid video time range for {camera}"
                    )));
                }
                videos.insert(
                    camera.clone(),
                    json!({"path": path, "from_timestamp": from, "to_timestamp": to}),
                );
                paths.insert(path);
            }
            let labels = row["tasks"]
                .as_array()
                .ok_or_else(|| invalid("LeRobot episode tasks are missing"))?;
            let labels: Vec<&str> = labels
                .iter()
                .map(|v| {
                    v.as_str()
                        .filter(|s| !s.trim().is_empty())
                        .ok_or_else(|| invalid("invalid LeRobot task label"))
                })
                .collect::<Result<_>>()?;
            if labels.is_empty() {
                return Err(invalid("LeRobot episode has no task labels"));
            }
            let metadata = json!({"format": "lerobot", "version": "v3.0", "episode_index": index,
                "frames": length, "fps": fps, "duration_s": length as f64 / fps,
                "data": {"path": data, "dataset_from_index": start, "dataset_to_index": end},
                "videos": videos, "features": features, "episode_metadata": row,
                "outcome": null, "policy": null});
            all_paths.extend(paths.iter().cloned());
            total_frames = total_frames
                .checked_add(length)
                .ok_or_else(|| invalid("LeRobot frame count overflow"))?;
            if episodes
                .insert(
                    index,
                    Episode {
                        index,
                        task: labels.join("; "),
                        metadata,
                        paths,
                    },
                )
                .is_some()
            {
                return Err(invalid("duplicate LeRobot episode_index"));
            }
        }
    }
    if episodes.is_empty()
        || episodes.len() as u64 != number(&info, "total_episodes")?
        || total_frames != number(&info, "total_frames")?
        || data_cache
            .values()
            .map(|rows| rows.len() as u64)
            .sum::<u64>()
            != total_frames
    {
        return Err(invalid(
            "LeRobot dataset is incomplete: episode/frame totals do not match meta/info.json",
        ));
    }
    let mut expected_start = 0;
    for (expected_index, episode) in episodes.values().enumerate() {
        if episode.index != expected_index as u64
            || episode.metadata["data"]["dataset_from_index"].as_u64() != Some(expected_start)
        {
            return Err(invalid(
                "LeRobot dataset must contain contiguous episodes and frame ranges",
            ));
        }
        expected_start = episode.metadata["data"]["dataset_to_index"]
            .as_u64()
            .unwrap();
    }
    // Content identity is independent of the download's directory name.
    let mut identity = Sha256::new();
    let mut artifacts = BTreeMap::new();
    for path in &all_paths {
        let (bytes, sha256) = checksum(&regular_file(source, path)?)?;
        identity.update(path.as_bytes());
        identity.update([0]);
        identity.update(sha256.as_bytes());
        artifacts.insert(
            path.clone(),
            Artifact {
                path: path.clone(),
                format: match Path::new(path).extension().and_then(|s| s.to_str()) {
                    Some("mp4") => "video/mp4",
                    Some("parquet") => "application/vnd.apache.parquet",
                    _ => "application/json",
                }
                .into(),
                bytes,
                sha256,
            },
        );
    }
    let dataset_id = hex::encode(identity.finalize());
    let mut manifests = vec![];
    for episode in episodes.values() {
        let id = format!("lr-{dataset_id}-{:06}", episode.index);
        if store.join(&id).try_exists()? {
            return Err(invalid(format!(
                "experience {id} already exists; no experiences imported"
            )));
        }
        manifests.push(Manifest {
            version: 1,
            id,
            robot: robot.into(),
            task: episode.task.clone(),
            artifacts: vec![],
        });
    }
    // Stage and verify all files before publishing. Shared chunks are copied once
    // then hard-linked between imported experiences, never to the user's source.
    fs::create_dir_all(store)?;
    let staging = tempfile::Builder::new()
        .prefix(".humaan-lerobot-")
        .tempdir_in(store)?;
    let shared = staging.path().join("shared");
    for (path, artifact) in &artifacts {
        let target = shared.join(path);
        fs::create_dir_all(target.parent().unwrap())?;
        fs::copy(regular_file(source, path)?, &target)?;
        if checksum(&target)? != (artifact.bytes, artifact.sha256.clone()) {
            return Err(invalid(format!(
                "LeRobot source changed during import: {path}"
            )));
        }
    }
    for (episode, manifest) in episodes.values().zip(&mut manifests) {
        let root = staging.path().join(&manifest.id);
        fs::create_dir(&root)?;
        for path in &episode.paths {
            let target = root.join(path);
            fs::create_dir_all(target.parent().unwrap())?;
            fs::hard_link(shared.join(path), &target)?;
            let a = &artifacts[path];
            manifest.artifacts.push(Artifact {
                path: a.path.clone(),
                format: a.format.clone(),
                bytes: a.bytes,
                sha256: a.sha256.clone(),
            });
        }
        let mut metadata = episode.metadata.clone();
        metadata["dataset_sha256"] = dataset_id.clone().into();
        let path = "lerobot-experience.json";
        fs::write(root.join(path), serde_json::to_vec_pretty(&metadata)?)?;
        let (bytes, sha256) = checksum(&root.join(path))?;
        manifest.artifacts.push(Artifact {
            path: path.into(),
            format: "application/vnd.humaan.lerobot-experience+json".into(),
            bytes,
            sha256,
        });
        fs::write(
            root.join("manifest.json"),
            serde_json::to_vec_pretty(&manifest)?,
        )?;
        crate::inspect(&root)?;
    }
    let mut published = vec![];
    for manifest in &manifests {
        let destination = store.join(&manifest.id);
        let result = (|| -> Result<()> {
            fs::create_dir(&destination)?;
            if let Err(e) = fs::rename(staging.path().join(&manifest.id), &destination) {
                let _ = fs::remove_dir(&destination);
                return Err(e.into());
            }
            Ok(())
        })();
        if let Err(e) = result {
            for path in published {
                fs::remove_dir_all(path)?;
            }
            return Err(e);
        }
        published.push(destination);
    }
    Ok(manifests)
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use parquet::{
        data_type::{ByteArray, ByteArrayType, DoubleType, Int64Type},
        file::writer::SerializedFileWriter,
        schema::parser::parse_message_type,
    };
    use std::sync::Arc;

    enum Column {
        Int(Vec<i64>),
        Float(Vec<f64>),
        Text(Vec<&'static str>),
        List(Vec<&'static str>),
    }
    fn parquet(root: &Path, path: &str, schema: &str, columns: Vec<Column>) {
        let path = root.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut writer = SerializedFileWriter::new(
            fs::File::create(path).unwrap(),
            Arc::new(parse_message_type(schema).unwrap()),
            Default::default(),
        )
        .unwrap();
        let mut group = writer.next_row_group().unwrap();
        for values in columns {
            let mut column = group.next_column().unwrap().unwrap();
            match values {
                Column::Int(v) => {
                    column
                        .typed::<Int64Type>()
                        .write_batch(&v, None, None)
                        .unwrap();
                }
                Column::Float(v) => {
                    column
                        .typed::<DoubleType>()
                        .write_batch(&v, None, None)
                        .unwrap();
                }
                Column::Text(v) => {
                    column
                        .typed::<ByteArrayType>()
                        .write_batch(
                            &v.into_iter().map(ByteArray::from).collect::<Vec<_>>(),
                            None,
                            None,
                        )
                        .unwrap();
                }
                Column::List(v) => {
                    let n = v.len();
                    column
                        .typed::<ByteArrayType>()
                        .write_batch(
                            &v.into_iter().map(ByteArray::from).collect::<Vec<_>>(),
                            Some(&vec![1; n]),
                            Some(&vec![0; n]),
                        )
                        .unwrap();
                }
            }
            column.close().unwrap();
        }
        group.close().unwrap();
        writer.close().unwrap();
    }

    pub(crate) fn fixture(root: &Path) {
        fs::create_dir_all(root.join("meta")).unwrap();
        fs::write(root.join("meta/info.json"), serde_json::to_vec(&json!({
            "codebase_version": "v3.0", "robot_type": "test dual arm", "fps": 2,
            "total_episodes": 2, "total_frames": 4, "total_tasks": 1,
            "data_path": "data/chunk-{chunk_index:03d}/file-{file_index:03d}.parquet",
            "video_path": "videos/{video_key}/chunk-{chunk_index:03d}/file-{file_index:03d}.mp4",
            "features": {"timestamp": {"dtype": "float64"}, "episode_index": {"dtype": "int64"},
                "frame_index": {"dtype": "int64"}, "index": {"dtype": "int64"}, "task_index": {"dtype": "int64"},
                "observation.images.front": {"dtype": "video", "shape": [2, 2, 3]}}
        })).unwrap()).unwrap();
        parquet(
            root,
            "meta/tasks.parquet",
            "message tasks { REQUIRED INT64 task_index; REQUIRED BINARY __index_level_0__ (UTF8); }",
            vec![Column::Int(vec![0]), Column::Text(vec!["pick object"])],
        );
        parquet(
            root,
            "meta/episodes/chunk-000/file-000.parquet",
            "message episodes {
            REQUIRED INT64 episode_index; REQUIRED INT64 length;
            REQUIRED INT64 dataset_from_index; REQUIRED INT64 dataset_to_index;
            REQUIRED INT64 data/chunk_index; REQUIRED INT64 data/file_index;
            REQUIRED GROUP tasks (LIST) { REPEATED GROUP list { REQUIRED BINARY element (UTF8); } }
            REQUIRED INT64 videos/observation.images.front/chunk_index;
            REQUIRED INT64 videos/observation.images.front/file_index;
            REQUIRED DOUBLE videos/observation.images.front/from_timestamp;
            REQUIRED DOUBLE videos/observation.images.front/to_timestamp;
        }",
            vec![
                Column::Int(vec![0, 1]),
                Column::Int(vec![2, 2]),
                Column::Int(vec![0, 2]),
                Column::Int(vec![2, 4]),
                Column::Int(vec![0, 0]),
                Column::Int(vec![0, 0]),
                Column::List(vec!["pick object", "pick object"]),
                Column::Int(vec![0, 0]),
                Column::Int(vec![0, 0]),
                Column::Float(vec![0.0, 1.0]),
                Column::Float(vec![1.0, 2.0]),
            ],
        );
        write_data(root, vec![0.0, 0.5, 0.0, 0.5]);
        let video = root.join("videos/observation.images.front/chunk-000/file-000.mp4");
        fs::create_dir_all(video.parent().unwrap()).unwrap();
        // Opaque bytes: the importer preserves videos but does not decode them.
        fs::write(video, b"synthetic shared video bytes").unwrap();
    }

    fn write_data(root: &Path, timestamps: Vec<f64>) {
        parquet(
            root,
            "data/chunk-000/file-000.parquet",
            "message frames {
            REQUIRED INT64 episode_index; REQUIRED INT64 frame_index; REQUIRED INT64 index;
            REQUIRED INT64 task_index; REQUIRED DOUBLE timestamp;
        }",
            vec![
                Column::Int(vec![0, 0, 1, 1]),
                Column::Int(vec![0, 1, 0, 1]),
                Column::Int(vec![0, 1, 2, 3]),
                Column::Int(vec![0, 0, 0, 0]),
                Column::Float(timestamps),
            ],
        );
    }

    #[test]
    fn maps_shared_chunks_and_preserves_export_and_identity() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        fixture(&source);
        let store = temp.path().join("store");
        let imported = crate::import_recordings(&store, &source).unwrap();
        assert_eq!(imported.len(), 2);
        assert_ne!(imported[0].id, imported[1].id);
        let details = crate::lerobot_details(&store, &imported[1])
            .unwrap()
            .unwrap();
        assert_eq!(details["episode_index"], 1);
        assert_eq!(details["data"]["dataset_from_index"], 2);
        assert_eq!(
            details["videos"]["observation.images.front"]["from_timestamp"],
            1.0
        );
        assert!(details["outcome"].is_null());
        assert!(
            crate::import_recordings(&store, &source)
                .unwrap_err()
                .to_string()
                .contains("already exists")
        );
        fs::rename(&source, temp.path().join("moved-source")).unwrap();
        assert!(crate::verify(&store).unwrap().iter().all(|v| v.valid));
        let exported = temp.path().join("exported");
        crate::export(&store, &imported[1].id, &exported).unwrap();
        let video = "videos/observation.images.front/chunk-000/file-000.mp4";
        assert_eq!(
            fs::read(exported.join(video)).unwrap(),
            b"synthetic shared video bytes"
        );
        assert_eq!(crate::list(&store).unwrap().len(), 2);
        // Edits to the external original never change the imported files.
        fs::write(temp.path().join("moved-source").join(video), "changed").unwrap();
        assert!(crate::verify(&store).unwrap().iter().all(|v| v.valid));
        fs::write(store.join(&imported[1].id).join(video), "corrupt").unwrap();
        assert!(crate::verify(&store).unwrap().iter().all(|v| !v.valid));
    }

    #[test]
    fn incomplete_or_invalid_datasets_publish_nothing() {
        for case in 0..5 {
            let temp = tempfile::tempdir().unwrap();
            let source = temp.path().join("source");
            fixture(&source);
            let store = temp.path().join("store");
            let info_path = source.join("meta/info.json");
            let mut info: Value = serde_json::from_slice(&fs::read(&info_path).unwrap()).unwrap();
            match case {
                0 => {
                    fs::remove_file(source.join("data/chunk-000/file-000.parquet")).unwrap();
                }
                1 => {
                    fs::remove_file(
                        source.join("videos/observation.images.front/chunk-000/file-000.mp4"),
                    )
                    .unwrap();
                }
                2 => {
                    info["codebase_version"] = "v2.1".into();
                }
                3 => {
                    info["data_path"] = "../outside.parquet".into();
                }
                _ => write_data(&source, vec![0.5, 0.0, 0.0, 0.5]),
            }
            fs::write(info_path, serde_json::to_vec(&info).unwrap()).unwrap();
            let error = crate::import_recordings(&store, &source).unwrap_err();
            assert!(!error.to_string().is_empty());
            assert!(!store.exists());
        }
    }
}
