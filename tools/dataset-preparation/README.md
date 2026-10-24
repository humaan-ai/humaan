# Prepare LeRobot datasets

This optional Python tool converts a Humaan learning-set recording bundle into
independent LeRobot v3 **image-based** train and validation datasets. It uses the
LeRobot 0.4.4 writer and checks every output sample through its standard
`LeRobotDataset` and PyTorch `DataLoader` before publishing a directory.
No custom DataLoader, training loop, cloud account, or model download is required.
The core recording, review, and export workflows remain independent of Python.

## Managed desktop setup

In **Prepare training dataset**, choose **Set up dataset tools** once. Humaan
installs its own Python 3.11 and the locked dependencies in the platform's local
app-data directory under `dataset-tools/`. No terminal commands, existing Python,
uv installation, or environment variables are needed for normal desktop use.
Setup requires an internet connection and enough space for Python/PyTorch and its
package cache. Progress appears in the panel. Preparation works offline afterward.

Humaan downloads a pinned uv 0.9.11 archive from Astral's official GitHub release,
checks its embedded SHA-256, and extracts only the expected executable. Bootstrap
uses the OS's `curl` and `tar` commands (included with macOS and modern Windows;
required packages on Linux). uv installs managed Python and uses the embedded
project/lockfile to synchronize packages into an app-owned environment. System
Python and shell profiles are not modified.

Setup supports Apple Silicon macOS, x86-64 Linux, and x86-64 Windows. The automated
installation test runs on Apple Silicon; the other platform paths need release
validation. **Repair dataset tools** builds a fresh environment and switches to it
only after an import/version check passes. Failed attempts can be retried; the
previous environment remains intact. Dependency-lock changes request fresh setup.
Old successful environments and caches are retained for now, so repeated repairs
can use additional disk space. The app-data `setup.log` contains diagnostics.

## Advanced/custom Python and standalone use

Python 3.11 is the tested runtime. For command-line use, install
[uv](https://docs.astral.sh/uv/) if needed and run from this repository root:

```sh
uv sync --project tools/dataset-preparation --python 3.11 --locked
```

The lockfile pins the SDK and dependencies. Installation needs network access;
preparation sets Hugging Face offline flags. Advanced desktop users can override
the managed environment by setting `HUMAAN_DATASET_PYTHON` to the absolute Python
executable path before starting Humaan. Unset it to return to managed tools. The
override is validated before preparation and is never modified by managed setup.

## Desktop

Open a saved learning set and choose **Prepare training dataset**. Assign eligible
experiences to Train, Validation, or Exclude. Both splits must be nonempty. Confirm
that the complete selected trajectories contain suitable imitation behavior, then
choose a parent directory. Output is written to `prepared-<learning-set-id>/`.
Existing output is never overwritten. Failures clean up staging data and leave
original recordings and learning sets unchanged.

## Exported-bundle command

After **Export experience files**, create a configuration using IDs from
`learning-set.json`:

```json
{
  "version": 1,
  "approve_whole_trajectories": true,
  "train": ["TRAIN_EXPERIENCE_ID"],
  "validation": ["VALIDATION_EXPERIENCE_ID"]
}
```

Run with a new output directory and an existing parent:

```sh
uv run --project tools/dataset-preparation --locked python tools/dataset-preparation/prepare.py \
  --bundle /path/to/learning-set-ID \
  --config /path/to/preparation.json \
  --output /path/to/prepared-ID
```

An explicit assignment is required; the tool does not silently train on all set
members or automatically convert failed actions into demonstrations. Labels come
from the set's pinned human review, not newer annotations beside a recording.

## Supported data and validation

The first adapter requires:

- Imported LeRobot v3.0 episodes, reviewed success with assistance explicitly `no`.
- At least one episode per split; no duplicate source dataset/episode identity,
  including aliases, anywhere in the preparation.
- Matching robot, integer positive FPS, feature shapes, dtypes, and dimension names.
- `observation.state` and `action` as one-dimensional float32 vectors, and at least
  one H×W×3 RGB video feature. Additional non-camera features must be float32 vectors.
- Contiguous frame/global indices and timestamps matching `frame_index / fps`
  within 0.0001 seconds. Source task labels must resolve through tasks metadata.
- Camera frames inside the selected video's half-open episode range, within half
  a frame interval of each requested timestamp, with no repeated frame.

All selected source artifacts must match the recording checksums and pinned manifest
fingerprints. Symbolic links and path traversal are rejected. Decoded RGB frames
are embedded in the new Parquet files; shared source video chunks are not copied
into the prepared dataset. Episodes are renumbered independently in each split.
The SDK recomputes statistics separately for each split. Use **training statistics**
for policy normalization; do not fit normalization on validation data.

LeRobot 0.4.4 exposes length-one numeric features as scalar tensors; other vectors
retain their channel dimension. The report records the actual sample tensor shapes
so a policy adapter can account for this SDK behavior.

The exporter does not infer units, action meaning, coordinate frames, corrections,
policy compatibility, or the absence of related scene/session leakage. Curators
must choose independent validation episodes. Assisted successes require future
segment-level preparation and are excluded in this first whole-episode adapter.

## Output and use

```text
prepared-ID/
├── learning-set.json
├── preparation-report.json
├── train/                 # LeRobot v3 metadata and selected image/state/action data
└── validation/            # Separate held-out dataset and statistics
```

The report records configuration, SDK versions, learning-set checksum, selected
source identities, output episode mapping, pinned review revisions, exclusions,
and actual loader-validation counts. A successful report means every prepared
sample loaded with finite tensors and the expected shapes. It does not mean a
policy was trained or its behavior improved.

Example using the existing reader:

```python
import json
from pathlib import Path
from lerobot.datasets.lerobot_dataset import LeRobotDataset
from torch.utils.data import DataLoader

root = Path("/path/to/prepared-ID")
report = json.loads((root / "preparation-report.json").read_text())
dataset = LeRobotDataset(
    report["splits"]["train"]["repo_id"],
    root=root / "train",
    video_backend="pyav",
)
batch = next(iter(DataLoader(dataset, batch_size=8, num_workers=0)))
print(batch["observation.state"].shape, batch["action"].shape)
```

The source observations/actions retain their meanings and dimensions. Configure
the chosen policy's camera mapping, action horizon (`delta_timestamps`),
normalization, and training settings separately. Held-out data validation does not
replace robot-task evaluation.

## Tests

```sh
uv run --project tools/dataset-preparation --locked python -m unittest discover \
  -s tools/dataset-preparation/tests -v
```

The tests construct a real shared MP4/Parquet fixture and exercise the official
LeRobot writer/reader and PyTorch DataLoader, including moving the prepared output
and excluding the middle source episode. They also check corruption, split overlap,
eligibility, path safety, and cleanup on failure.

SDK reference: [LeRobot v3 datasets](https://huggingface.co/docs/lerobot/v0.4.4/lerobot-dataset-v3).
