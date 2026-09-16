import json
from pathlib import Path
import shutil
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import prepare


def fixture(root):
    import av
    import numpy as np
    import pyarrow as pa
    import pyarrow.parquet as pq
    source = root / "source"
    source.mkdir()
    video = source / "camera.mp4"
    with av.open(str(video), "w") as container:
        stream = container.add_stream("libx264", rate=2)
        stream.width, stream.height, stream.pix_fmt = 16, 16, "yuv420p"
        for index in range(6):
            frame = av.VideoFrame.from_ndarray(np.full((16, 16, 3), index * 30, dtype=np.uint8), format="rgb24")
            for packet in stream.encode(frame):
                container.mux(packet)
        for packet in stream.encode():
            container.mux(packet)
    rows = [{"episode_index": i // 2, "frame_index": i % 2, "index": i,
             "timestamp": (i % 2) / 2, "task_index": 0,
             "observation.state": [float(i), 1.0], "action": [float(i) + 0.25]} for i in range(6)]
    pq.write_table(pa.Table.from_pylist(rows), source / "data.parquet")
    pq.write_table(pa.Table.from_pylist([{"task_index": 0, "__index_level_0__": "pick object"}]), source / "tasks.parquet")
    bundle = root / "bundle"
    members = []
    features = {"observation.state": {"dtype": "float32", "shape": [2], "names": ["a", "b"]},
                "action": {"dtype": "float32", "shape": [1], "names": ["a"]},
                "observation.images.front": {"dtype": "video", "shape": [16, 16, 3], "names": ["height", "width", "channels"]}}
    for episode in range(3):
        id_ = f"episode-{episode}"
        directory = bundle / "recordings" / id_
        (directory / "meta").mkdir(parents=True)
        shutil.copy(source / "data.parquet", directory / "data.parquet")
        shutil.copy(video, directory / "camera.mp4")
        shutil.copy(source / "tasks.parquet", directory / "meta/tasks.parquet")
        metadata = {"format": "lerobot", "version": "v3.0", "fps": 2, "frames": 2,
                    "dataset_sha256": "a" * 64, "episode_index": episode, "features": features,
                    "data": {"path": "data.parquet", "dataset_from_index": episode * 2, "dataset_to_index": episode * 2 + 2},
                    "videos": {"observation.images.front": {"path": "camera.mp4", "from_timestamp": episode, "to_timestamp": episode + 1}}}
        (directory / "lerobot-experience.json").write_text(json.dumps(metadata))
        manifest = {"version": 1, "id": id_, "robot": "test-arm", "task": "pick object", "artifacts": []}
        for path in ("data.parquet", "camera.mp4", "meta/tasks.parquet", "lerobot-experience.json"):
            manifest["artifacts"].append({"path": path, "bytes": (directory / path).stat().st_size,
                "sha256": prepare.digest(directory / path),
                "format": "application/vnd.humaan.lerobot-experience+json" if path == "lerobot-experience.json" else "opaque"})
        (directory / "manifest.json").write_text(json.dumps(manifest))
        members.append({"experience_id": id_, "manifest": manifest, "manifest_sha256": prepare.digest(directory / "manifest.json"),
                        "reason": "Useful example", "review": {"revision": 1, "review": {"review_state": "reviewed", "outcome": "success", "assistance": "no"}}})
    (bundle / "learning-set.json").write_text(json.dumps({"version": 1, "id": "test-set", "members": members}))
    return bundle


def config():
    return {"version": 1, "approve_whole_trajectories": True, "train": ["episode-0"], "validation": ["episode-2"]}


class PreparationTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.bundle = fixture(self.root)

    def test_real_loader_selected_frames_and_portability(self):
        from lerobot.datasets.lerobot_dataset import LeRobotDataset
        from torch.utils.data import DataLoader
        output = self.root / "prepared"
        report = prepare.prepare(self.bundle, config(), output)
        self.assertEqual(report["excluded"][0]["experience_id"], "episode-1")
        self.assertEqual(report["splits"]["train"]["dataloader_validation"]["frames"], 2)
        moved = self.root / "moved"
        output.rename(moved)
        train = LeRobotDataset("humaan/test-set-train", root=moved / "train", video_backend="pyav")
        val = LeRobotDataset("humaan/test-set-validation", root=moved / "validation", video_backend="pyav")
        self.assertEqual(next(iter(DataLoader(train, batch_size=2)))["action"].flatten().tolist(), [0.25, 1.25])
        batch = next(iter(DataLoader(val, batch_size=2)))
        self.assertEqual(batch["observation.state"][:, 0].tolist(), [4, 5])
        self.assertEqual(tuple(batch["observation.images.front"].shape), (2, 3, 16, 16))
        self.assertGreater(float(batch["observation.images.front"][0].mean()), 0.4)
        self.assertFalse(list(moved.rglob("*.mp4")))
        with self.assertRaisesRegex(ValueError, "already exists"):
            prepare.prepare(self.bundle, config(), moved)

    def test_requires_explicit_disjoint_splits_and_eligible_labels(self):
        for change in ({"validation": ["episode-0"]}, {"train": []}, {"approve_whole_trajectories": False}):
            with self.assertRaises(ValueError):
                prepare.load_plan(self.bundle, {**config(), **change})
        path = self.bundle / "learning-set.json"
        value = prepare.read_json(path)
        value["members"][0]["review"]["review"]["assistance"] = "yes"
        path.write_text(json.dumps(value))
        with self.assertRaisesRegex(ValueError, "no human assistance"):
            prepare.load_plan(self.bundle, config())

    def test_corrupt_source_publishes_nothing(self):
        (self.bundle / "recordings/episode-2/camera.mp4").write_bytes(b"broken")
        with self.assertRaisesRegex(ValueError, "integrity"):
            prepare.prepare(self.bundle, config(), self.root / "output")
        self.assertFalse((self.root / "output").exists())
        self.assertFalse(list(self.root.glob(".humaan-prepare-*")))

    def resign(self, id_):
        directory = self.bundle / "recordings" / id_
        manifest = prepare.read_json(directory / "manifest.json")
        for artifact in manifest["artifacts"]:
            artifact["bytes"] = (directory / artifact["path"]).stat().st_size
            artifact["sha256"] = prepare.digest(directory / artifact["path"])
        (directory / "manifest.json").write_text(json.dumps(manifest))
        path = self.bundle / "learning-set.json"
        learning_set = prepare.read_json(path)
        member = next(m for m in learning_set["members"] if m["experience_id"] == id_)
        member["manifest"] = manifest
        member["manifest_sha256"] = prepare.digest(directory / "manifest.json")
        path.write_text(json.dumps(learning_set))

    def test_failed_camera_decode_cleans_staging(self):
        path = self.bundle / "recordings/episode-2/lerobot-experience.json"
        metadata = prepare.read_json(path)
        metadata["videos"]["observation.images.front"].update(from_timestamp=2.9, to_timestamp=3.9)
        path.write_text(json.dumps(metadata))
        self.resign("episode-2")
        with self.assertRaisesRegex(ValueError, "frame"):
            prepare.prepare(self.bundle, config(), self.root / "output")
        self.assertFalse((self.root / "output").exists())
        self.assertFalse(list(self.root.glob(".humaan-prepare-*")))

    def test_aliased_source_episode_cannot_cross_splits(self):
        path = self.bundle / "recordings/episode-2/lerobot-experience.json"
        metadata = prepare.read_json(path)
        metadata["episode_index"] = 0
        path.write_text(json.dumps(metadata))
        self.resign("episode-2")
        with self.assertRaisesRegex(ValueError, "Duplicate source episode"):
            prepare.prepare(self.bundle, config(), self.root / "output")
        self.assertFalse((self.root / "output").exists())

    def test_nonfinite_actions_are_rejected(self):
        import pyarrow as pa
        import pyarrow.parquet as pq
        path = self.bundle / "recordings/episode-2/data.parquet"
        rows = pq.read_table(path).to_pylist()
        rows[4]["action"] = [float("nan")]
        pq.write_table(pa.Table.from_pylist(rows), path)
        self.resign("episode-2")
        with self.assertRaisesRegex(ValueError, "non-finite"):
            prepare.prepare(self.bundle, config(), self.root / "output")
        self.assertFalse((self.root / "output").exists())

    def test_paths_cannot_escape_bundle(self):
        with self.assertRaises(ValueError):
            prepare.regular(self.bundle, "../source/data.parquet")
        (self.bundle / "linked").symlink_to(self.root / "source", target_is_directory=True)
        with self.assertRaises(ValueError):
            prepare.regular(self.bundle, "linked/data.parquet")


if __name__ == "__main__":
    unittest.main()
