#!/usr/bin/env python3
"""Prepare offline LeRobot v3 image datasets from a Humaan learning-set bundle.

Use the pinned environment in pyproject.toml. No network calls or policy training.
"""
from __future__ import annotations

import argparse
import hashlib
import importlib.metadata
import json
import math
import os
from pathlib import Path, PurePosixPath
import re
import tempfile
from contextlib import ExitStack

# Keep SDK fallback lookups local, even if source metadata is incomplete.
os.environ["HF_HUB_OFFLINE"] = "1"
os.environ["HF_DATASETS_OFFLINE"] = "1"

DEFAULT_KEYS = {"index", "episode_index", "frame_index", "timestamp", "task_index"}
PINNED_LEROBOT = "0.4.4"


def require(condition, message):
    if not condition:
        raise ValueError(message)


def regular(root: Path, relative: str) -> Path:
    require(isinstance(relative, str) and relative and "\\" not in relative and ":" not in relative,
            "Invalid relative recording path")
    parts = relative.split("/")
    require(not PurePosixPath(relative).is_absolute() and all(p not in ("", ".", "..") for p in parts),
            "Recording paths cannot escape their directory")
    path = root
    for part in parts:
        path /= part
        require(not path.is_symlink(), f"Symbolic links are not supported: {relative}")
    require(path.is_file(), f"Missing recording file: {relative}")
    return path


def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def read_json(path):
    return json.loads(path.read_text())


def valid_id(value):
    require(isinstance(value, str) and re.fullmatch(r"[A-Za-z0-9_-]{1,128}", value), "Invalid experience ID")


def eligible(member):
    review = member["review"]["review"]
    return review["review_state"] == "reviewed" and review["outcome"] == "success" and review["assistance"] == "no"


def load_plan(bundle: Path, config: dict):
    set_path = regular(bundle, "learning-set.json")
    learning_set = read_json(set_path)
    require(learning_set.get("version") == 1, "Unsupported learning set version")
    valid_id(learning_set["id"])
    require(config.get("version") == 1 and config.get("approve_whole_trajectories") is True,
            "Confirm whole-trajectory suitability with approve_whole_trajectories: true")
    require(set(config) == {"version", "approve_whole_trajectories", "train", "validation"},
            "Unknown or missing preparation configuration fields")
    members = {}
    for member in learning_set["members"]:
        valid_id(member["experience_id"])
        require(member["experience_id"] not in members, "Duplicate learning set member")
        members[member["experience_id"]] = member
    chosen = []
    for split in ("train", "validation"):
        ids = config[split]
        require(isinstance(ids, list) and ids, f"Select at least one {split} episode")
        for id_ in ids:
            valid_id(id_)
            require(id_ in members, f"Experience is not in this learning set: {id_}")
            require(id_ not in chosen, "An episode cannot be repeated or cross train/validation splits")
            require(eligible(members[id_]), f"{id_}: v1 preparation requires reviewed success with no human assistance")
            chosen.append(id_)
    return learning_set, members


def verify_recording(bundle, member):
    id_ = member["experience_id"]
    directory = bundle / "recordings" / id_
    path = regular(bundle, f"recordings/{id_}/manifest.json")
    require(digest(path) == member["manifest_sha256"], f"{id_}: manifest differs from the curated snapshot")
    manifest = read_json(path)
    require(manifest == member["manifest"] and manifest["id"] == id_ and manifest["version"] == 1,
            f"{id_}: invalid manifest snapshot")
    artifacts = {}
    for artifact in manifest["artifacts"]:
        name = artifact["path"]
        require(name.lower() not in {p.lower() for p in artifacts}, "Duplicate artifact path")
        require(name.lower() != "manifest.json", "Manifest cannot be an artifact")
        path = regular(directory, name)
        require(path.stat().st_size == artifact["bytes"] and digest(path) == artifact["sha256"],
                f"{id_}: integrity check failed for {name}")
        artifacts[name] = artifact
    provenance = [a for a in artifacts.values() if a["format"] == "application/vnd.humaan.lerobot-experience+json"]
    require(len(provenance) == 1, f"{id_}: preparation supports imported LeRobot v3.0 episodes only")
    metadata = read_json(regular(directory, provenance[0]["path"]))
    require(metadata.get("format") == "lerobot" and metadata.get("version") == "v3.0", "Unsupported source format")
    require(re.fullmatch(r"[0-9a-f]{64}", metadata.get("dataset_sha256", "")), "Missing source dataset identity")
    return directory, manifest, artifacts, metadata


def schema(metadata):
    fps = metadata["fps"]
    require(isinstance(fps, (int, float)) and math.isfinite(fps) and fps > 0 and int(fps) == fps,
            "LeRobot export requires a positive integer FPS")
    result = {}
    for key, feature in metadata["features"].items():
        if key in DEFAULT_KEYS:
            continue
        shape = feature.get("shape")
        require(isinstance(shape, list) and shape and all(type(n) is int and n > 0 for n in shape),
                f"Missing or invalid shape for {key}")
        dtype = feature["dtype"]
        if dtype == "video":
            require(len(shape) == 3 and shape[2] == 3, f"{key}: only H×W×3 RGB video is supported")
            dtype = "image"
        else:
            require(dtype == "float32" and len(shape) == 1, f"{key}: only float32 vectors are supported")
        result[key] = {"dtype": dtype, "shape": tuple(shape), "names": feature.get("names")}
    for key in ("observation.state", "action"):
        require(key in result and result[key]["dtype"] == "float32", f"Required float32 vector missing: {key}")
    require(any(f["dtype"] == "image" for f in result.values()), "At least one RGB camera is required")
    require(set(metadata["videos"]) == {k for k, f in result.items() if f["dtype"] == "image"}, "Camera metadata/schema mismatch")
    return int(fps), result


def source_rows(directory, artifacts, metadata):
    import pyarrow.parquet as pq
    path = metadata["data"]["path"]
    require(path in artifacts and "meta/tasks.parquet" in artifacts, "Source files must be declared artifacts")
    rows = pq.read_table(regular(directory, path)).to_pylist()
    selected = [r for r in rows if r["episode_index"] == metadata["episode_index"]]
    start, end = metadata["data"]["dataset_from_index"], metadata["data"]["dataset_to_index"]
    require(len(selected) == metadata["frames"] == end - start and len(selected) > 0, "Inconsistent episode range")
    tasks = pq.read_table(regular(directory, "meta/tasks.parquet")).to_pandas()
    if "__index_level_0__" in tasks.columns:
        tasks = tasks.set_index("__index_level_0__")
    labels = {int(row.task_index): label for label, row in tasks.iterrows()}
    for index, row in enumerate(selected):
        require(row["frame_index"] == index and row["index"] == start + index, "Non-contiguous episode frames")
        timestamp = row["timestamp"]
        require(isinstance(timestamp, (int, float)) and math.isfinite(timestamp) and abs(timestamp - index / metadata["fps"]) <= 1e-4,
                "Source timestamps do not match the episode FPS")
        require(row["task_index"] in labels and isinstance(labels[row["task_index"]], str), "Missing task label")
        row["task"] = labels[row["task_index"]]
    return selected


def camera_frames(path, times, start, end, fps):
    """Stream nearest RGB frames within an episode's bounds; never reuse a frame."""
    import av
    require(math.isfinite(start) and math.isfinite(end) and 0 <= start < end, "Invalid camera time range")
    with av.open(str(path)) as container:
        frames = iter(container.decode(video=0))
        previous = None
        upcoming = next(frames, None)
        last_used = -math.inf
        for timestamp in times:
            target = start + timestamp
            require(start <= target < end, "Requested camera timestamp lies outside the selected episode")
            while upcoming is not None and upcoming.time is not None and upcoming.time < target:
                previous, upcoming = upcoming, next(frames, None)
            candidates = [f for f in (previous, upcoming) if f is not None and f.time is not None and start - 1e-6 <= f.time < end]
            require(candidates, "Video has no frame at the selected episode timestamp")
            frame = min(candidates, key=lambda f: abs(f.time - target))
            require(abs(frame.time - target) <= 0.5 / fps + 1e-6 and frame.time > last_used,
                    "Missing, repeated, or misaligned video frame")
            last_used = frame.time
            yield frame.to_ndarray(format="rgb24")


def write_episode(dataset, entry, features, fps):
    import numpy as np
    directory, manifest, artifacts, metadata = entry
    rows = source_rows(directory, artifacts, metadata)
    with ExitStack() as stack:
        cameras = {}
        for key, video in metadata["videos"].items():
            require(video["path"] in artifacts, "Camera video must be a verified artifact")
            generator = camera_frames(regular(directory, video["path"]), [r["timestamp"] for r in rows],
                                      video["from_timestamp"], video["to_timestamp"], fps)
            stack.callback(generator.close)
            cameras[key] = generator
        for row in rows:
            frame = {"task": row["task"]}
            for key, feature in features.items():
                value = next(cameras[key]) if key in cameras else np.asarray(row[key], dtype=np.float32)
                require(tuple(value.shape) == feature["shape"], f"{manifest['id']}: wrong shape for {key}")
                require(np.isfinite(value).all(), f"{manifest['id']}: non-finite values in {key}")
                frame[key] = value
            dataset.add_frame(frame)
        dataset.save_episode()
    return len(rows)


def validate_loader(root, repo_id, features, expected_frames, expected_episodes):
    from lerobot.datasets.lerobot_dataset import LeRobotDataset
    from torch.utils.data import DataLoader
    import torch
    dataset = LeRobotDataset(repo_id, root=root, video_backend="pyav")
    require(len(dataset) == expected_frames and dataset.num_episodes == expected_episodes, "Prepared dataset count mismatch")
    batches = 0
    tensor_shapes = {}
    for batch in DataLoader(dataset, batch_size=min(8, len(dataset)), num_workers=0, shuffle=False):
        for key, feature in features.items():
            tensor = batch[key]
            shape = feature["shape"]
            expected = (shape[2], shape[0], shape[1]) if feature["dtype"] == "image" else (() if shape == (1,) else shape)
            tensor_shapes[key] = list(tensor.shape[1:])
            require(tuple(tensor.shape[1:]) == expected and tensor.dtype == torch.float32 and bool(torch.isfinite(tensor).all()),
                    f"DataLoader produced invalid {key} tensors")
        batches += 1
    return {"passed": True, "frames": len(dataset), "episodes": dataset.num_episodes, "batches": batches, "sample_tensor_shapes": tensor_shapes}


def prepare(bundle: Path, config: dict, output: Path):
    require(importlib.metadata.version("lerobot") == PINNED_LEROBOT, f"Install lerobot=={PINNED_LEROBOT}")
    from lerobot.datasets.lerobot_dataset import LeRobotDataset
    set_bytes = regular(bundle, "learning-set.json").read_bytes()
    learning_set, members = load_plan(bundle, config)
    require(json.loads(set_bytes) == learning_set, "Learning set changed while reading; retry with stable files")
    require(not output.exists() and not output.is_symlink(), "Output already exists; choose a new directory")
    require(output.parent.is_dir(), "Output parent directory must exist")
    entries, identities = {}, set()
    common = None
    for id_ in config["train"] + config["validation"]:
        print(f"Verifying {id_}", flush=True)
        entry = verify_recording(bundle, members[id_])
        metadata = entry[3]
        identity = (metadata["dataset_sha256"], metadata["episode_index"])
        require(identity not in identities, "Duplicate source episode: train/validation leakage or repeated data")
        identities.add(identity)
        current = (*schema(metadata), entry[1]["robot"])
        require(common is None or current == common, "Selected episodes must have matching FPS, robot, and feature schemas")
        common = current
        entries[id_] = entry
    fps, features, robot = common
    report = {"version": 1, "target": "lerobot-v3-image", "learning_set_id": learning_set["id"],
              "learning_set_sha256": hashlib.sha256(set_bytes).hexdigest(), "configuration": config,
              "dependencies": {p: importlib.metadata.version(p) for p in ("lerobot", "torch", "pyarrow", "av")},
              "excluded": [{"experience_id": id_, "reason": "Not assigned to a split" if eligible(m) else "Not a reviewed unassisted success"}
                           for id_, m in members.items() if id_ not in entries], "splits": {}}
    with tempfile.TemporaryDirectory(prefix=".humaan-prepare-", dir=output.parent) as temporary:
        staging = Path(temporary) / "dataset"
        staging.mkdir()
        for split in ("train", "validation"):
            repo_id = f"humaan/{learning_set['id']}-{split}"
            root = staging / split
            dataset = LeRobotDataset.create(repo_id, fps, features, root=root, robot_type=robot,
                                             use_videos=False, video_backend="pyav")
            mapping = []
            try:
                for index, id_ in enumerate(config[split]):
                    print(f"Preparing {split}: {id_}", flush=True)
                    count = write_episode(dataset, entries[id_], features, fps)
                    mapping.append({"episode_index": index, "experience_id": id_, "frames": count,
                                    "source_dataset_sha256": entries[id_][3]["dataset_sha256"],
                                    "source_episode_index": entries[id_][3]["episode_index"],
                                    "review_revision": members[id_]["review"]["revision"],
                                    "manifest_sha256": members[id_]["manifest_sha256"]})
            finally:
                dataset.finalize()
            del dataset
            # Image pixels are embedded into Parquet; remove SDK's temporary PNGs
            # so moving the directory also proves it has no absolute image dependency.
            import shutil
            if (root / "images").exists():
                shutil.rmtree(root / "images")
            check = validate_loader(root, repo_id, features, sum(m["frames"] for m in mapping), len(mapping))
            report["splits"][split] = {"repo_id": repo_id, "episodes": mapping, "dataloader_validation": check}
        (staging / "preparation-report.json").write_text(json.dumps(report, indent=2) + "\n")
        (staging / "learning-set.json").write_bytes(set_bytes)
        # Reserve the destination before rename, preserving any pre-existing output.
        output.mkdir()
        try:
            staging.rename(output)
        except BaseException:
            output.rmdir()
            raise
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--bundle", type=Path, required=True)
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    try:
        report = prepare(args.bundle, read_json(args.config), args.output)
        print(json.dumps({"output": str(args.output), "validation": {k: v["dataloader_validation"] for k, v in report["splits"].items()}}))
    except Exception as error:
        parser.exit(1, f"Dataset preparation failed: {error}\n")


if __name__ == "__main__":
    main()
