//! Export selected LeRobot episodes to Rerun without requiring Python.
use crate::{Manifest, Result, inspect_experience, invalid, lerobot_details, regular_file};
use parquet::file::reader::{FileReader, SerializedFileReader};
use rerun::{
    AssetVideo, RecordingStreamBuilder, Scalars, TextDocument, VideoFrameReference,
    components::VideoTimestamp,
};
use serde::Deserialize;
use serde_json::Value;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

#[derive(Deserialize)]
struct DataRange {
    path: String,
    dataset_from_index: u64,
    dataset_to_index: u64,
}
#[derive(Deserialize)]
struct VideoRange {
    path: String,
    from_timestamp: f64,
    to_timestamp: f64,
}
#[derive(Deserialize)]
struct Source {
    format: String,
    version: String,
    episode_index: u64,
    frames: u64,
    duration_s: f64,
    data: DataRange,
    videos: BTreeMap<String, VideoRange>,
    features: BTreeMap<String, Value>,
}
struct Sample {
    time: f64,
    values: BTreeMap<String, f64>,
}

fn video_frames(timestamps: Vec<i64>, range: &VideoRange, duration: f64) -> Vec<i64> {
    let from = (range.from_timestamp * 1e9).round();
    let to = (range.to_timestamp * 1e9).round();
    timestamps
        .into_iter()
        .filter(|ns| {
            (*ns as f64) >= from && (*ns as f64) < to && (*ns as f64) - from < duration * 1e9
        })
        .collect()
}

fn artifact(root: &Path, manifest: &Manifest, relative: &str) -> Result<PathBuf> {
    if !manifest.artifacts.iter().any(|a| a.path == relative) {
        return Err(invalid(format!(
            "replay references an undeclared artifact: {relative}"
        )));
    }
    regular_file(root, relative)
}

fn samples(root: &Path, manifest: &Manifest, source: &Source) -> Result<Vec<Sample>> {
    if source.format != "lerobot"
        || source.version != "v3.0"
        || source.frames == 0
        || !source.duration_s.is_finite()
        || source.duration_s <= 0.0
        || source
            .data
            .dataset_to_index
            .checked_sub(source.data.dataset_from_index)
            != Some(source.frames)
    {
        return Err(invalid("invalid LeRobot replay metadata"));
    }
    let reader = SerializedFileReader::new(std::fs::File::open(artifact(
        root,
        manifest,
        &source.data.path,
    )?)?)?;
    let mut result: Vec<Sample> = vec![];
    for row in reader.get_row_iter(None)? {
        let row = row?.to_json_value();
        if row["episode_index"].as_u64() != Some(source.episode_index) {
            continue;
        }
        let time = row["timestamp"]
            .as_f64()
            .filter(|t| t.is_finite() && *t >= 0.0 && *t < source.duration_s)
            .ok_or_else(|| invalid("invalid episode timestamp for replay"))?;
        if row["index"].as_u64() != Some(source.data.dataset_from_index + result.len() as u64)
            || row["frame_index"].as_u64() != Some(result.len() as u64)
            || result.last().is_some_and(|previous| time <= previous.time)
        {
            return Err(invalid("episode frame range or timestamps changed"));
        }
        let mut values = BTreeMap::new();
        for key in ["observation.state", "action"] {
            if let Some(feature) = source.features.get(key) {
                let vector = row[key].as_array().ok_or_else(|| {
                    invalid(format!("replay requires a numeric vector for {key}"))
                })?;
                if feature["shape"].as_array().is_none_or(|shape| {
                    shape.len() != 1 || shape[0].as_u64() != Some(vector.len() as u64)
                }) {
                    return Err(invalid(format!("unsupported replay shape for {key}")));
                }
                for (index, value) in vector.iter().enumerate() {
                    let value = value
                        .as_f64()
                        .filter(|n| n.is_finite())
                        .ok_or_else(|| invalid(format!("non-numeric {key} value")))?;
                    let name = feature["names"][index].as_str().unwrap_or("value");
                    // Index keeps identically named channels distinct. Escape source names
                    // so slash/bracket characters do not change the entity hierarchy.
                    let safe_name: String = name
                        .chars()
                        .map(|c| {
                            if c.is_alphanumeric() || c == '_' || c == '-' {
                                c
                            } else {
                                '_'
                            }
                        })
                        .collect();
                    values.insert(format!("{key}/{index:03}_{safe_name}"), value);
                }
            }
        }
        result.push(Sample { time, values });
    }
    if result.len() as u64 != source.frames {
        return Err(invalid("missing episode frames for replay"));
    }
    Ok(result)
}

/// Convert only the selected episode's timeline to an RRD at a new destination.
/// Whole video assets remain intact; frame references exclude adjacent episodes.
/// No viewer is started, making this operation usable in headless tests.
pub fn save(store: &Path, id: &str, destination: &Path) -> Result<()> {
    let manifest = inspect_experience(store, id)?;
    let source: Source =
        serde_json::from_value(lerobot_details(store, &manifest)?.ok_or_else(|| {
            invalid("Rerun replay currently supports imported LeRobot v3.0 experiences only")
        })?)?;
    let root = store.join(id);
    let samples = samples(&root, &manifest, &source)?;
    let mut videos = vec![];
    for (index, (name, range)) in source.videos.iter().enumerate() {
        if !range.from_timestamp.is_finite()
            || !range.to_timestamp.is_finite()
            || range.from_timestamp < 0.0
            || range.to_timestamp <= range.from_timestamp
        {
            return Err(invalid(format!("invalid camera range: {name}")));
        }
        let video = AssetVideo::from_file_path(artifact(&root, &manifest, &range.path)?)?;
        let timestamps = video.read_frame_timestamps_nanos()?;
        let selected = video_frames(timestamps, range, source.duration_s);
        if selected.is_empty() {
            return Err(invalid(format!(
                "no video frames within episode range: {name}"
            )));
        }
        videos.push((
            format!("cameras/{index:03}"),
            name,
            video,
            selected,
            range.from_timestamp,
        ));
    }
    // Reserve the output to avoid overwriting files chosen by the caller.
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)?;
    let result = (|| -> Result<()> {
        let recording = RecordingStreamBuilder::new("humaan")
            .enabled(true)
            .with_blueprint(rerun::blueprint::Blueprint::auto().with_time_panel(
                rerun::blueprint::TimePanel::new().with_timeline("experience_time"),
            ))
            .recording_name(format!("{} · {}", manifest.task, id))
            .save(destination)?;
        recording.log_static("experience", &TextDocument::new(format!("Task: {}\nRobot: {}\nExperience: {}\nLeRobot episode: {}\nOutcome: unknown\nPolicy: unknown\nSignals retain source names and units; no robot geometry is inferred.", manifest.task, manifest.robot, id, source.episode_index)))?;
        for (entity, name, video, timestamps, offset) in videos {
            recording.log_static(entity.clone(), &video)?;
            recording.log_static(format!("{entity}/label"), &TextDocument::new(name.as_str()))?;
            for timestamp in timestamps {
                recording.set_duration_secs(
                    "experience_time",
                    (timestamp as f64 / 1e9 - offset).max(0.0),
                );
                recording.log(
                    entity.clone(),
                    &VideoFrameReference::new(VideoTimestamp::from_nanos(timestamp)),
                )?;
            }
        }
        for sample in samples {
            recording.set_duration_secs("experience_time", sample.time);
            for (entity, value) in sample.values {
                recording.log(entity, &Scalars::new([value]))?;
            }
        }
        recording.flush_blocking()?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(destination);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn video_selection_excludes_neighboring_episodes() {
        let range = VideoRange {
            path: "shared.mp4".into(),
            from_timestamp: 1.0,
            to_timestamp: 2.0,
        };
        assert_eq!(
            video_frames(
                vec![0, 500_000_000, 1_000_000_000, 1_500_000_000, 2_000_000_000],
                &range,
                1.0
            ),
            vec![1_000_000_000, 1_500_000_000]
        );
    }

    #[test]
    fn selected_episode_replays_and_rejects_missing_or_undeclared_data() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        crate::lerobot::tests::fixture(&source);
        std::fs::write(
            source.join("videos/observation.images.front/chunk-000/file-000.mp4"),
            include_bytes!("../tests/fixtures/shared-video.mp4"),
        )
        .unwrap();
        let store = temp.path().join("store");
        let imported = crate::import_recordings(&store, &source).unwrap();
        let manifest = &imported[1];
        let mut metadata: Source =
            serde_json::from_value(crate::lerobot_details(&store, manifest).unwrap().unwrap())
                .unwrap();
        let data = samples(&store.join(&manifest.id), manifest, &metadata).unwrap();
        assert_eq!(data.len(), 2);
        assert_eq!(
            data.iter().map(|s| s.time).collect::<Vec<_>>(),
            vec![0.0, 0.5]
        );
        let output = temp.path().join("selected.rrd");
        save(&store, &manifest.id, &output).unwrap();
        let original = std::fs::read(&output).unwrap();
        assert!(original.len() > 100);
        assert!(save(&store, &manifest.id, &output).is_err());
        assert_eq!(original, std::fs::read(output).unwrap());
        metadata.data.path = "../outside.parquet".into();
        assert!(samples(&store.join(&manifest.id), manifest, &metadata).is_err());
        std::fs::remove_file(
            store
                .join(&manifest.id)
                .join("data/chunk-000/file-000.parquet"),
        )
        .unwrap();
        let rejected = temp.path().join("rejected.rrd");
        assert!(save(&store, &manifest.id, &rejected).is_err());
        assert!(!rejected.exists());
    }
}
