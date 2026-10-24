# Local learning set format v1

A learning set is an immutable, explicitly curated selection of whole experiences
for a stated learning goal. It is not a claim of training compatibility or an
instruction to imitate every recorded action. Failures and reviewed unknown
outcomes may be useful evidence when the curator records why they belong.

Each project stores `.humaan/learning-sets/<id>.json` with these fields:

| Field | Meaning |
| --- | --- |
| `version` | `1` |
| `id` | Locally generated UUID |
| `project_id` | Owning local project identity |
| `name` | Nonblank set name, up to 200 UTF-8 bytes |
| `learning_goal` | Nonblank learning objective, up to 4,000 UTF-8 bytes |
| `curator` | Self-reported name, up to 200 UTF-8 bytes |
| `created_at_unix_ms` | Creation time, UTC Unix milliseconds |
| `members` | Nonempty selection, unique by experience ID |

Each member contains `experience_id`, nonblank `reason` (up to 4,000 UTF-8 bytes),
`manifest_sha256` of the original manifest bytes, the complete `manifest` snapshot
(including artifact paths and checksums), and the complete selected `review`
revision from [annotations v1](../recording-format/README.md#human-outcome-annotations-v1).
Artifact paths remain relative to the source experience recording directory.

Creation requires the selected revision to be the latest local review and its
state to be `reviewed`. It verifies all selected artifacts and validates every
selection before atomically publishing a new set file without overwriting any
existing set. Keep recordings stable during creation, as with recording import.
No review or source artifact is edited. Human assistance is not interpreted as a
known correction segment, and unknown policy identity is not filled in.

Later reviews do not update a saved set. Creating a new set captures revised
selections; sets do not yet have a mutable draft or revision lifecycle on disk.
Desktop drafts are in memory and are discarded on application exit.

Manifest-only export creates `learning-set-<id>.json`, preserving the selection and snapshots
without copying recording artifacts. Consumers must resolve the source recordings,
check fingerprints and artifact integrity, and define training preparation and
evaluation splits separately. Local files are trusted metadata, not a tamper-proof
audit record. Cloud transfer, VLM evaluation, automatic ranking, temporal selection,
trainer integration, and learning-set import are outside this version.


## Experience file export

Full export creates a new directory:

```text
learning-set-<id>/
├── learning-set.json
└── recordings/
    └── <experience-id>/
        ├── manifest.json
        ├── annotations.json  # if present, current review history
        └── ...declared original artifacts
```

Only selected experience directories are exported. Every source manifest and copied
manifest must match the saved member fingerprint and manifest snapshot. The normal
recording export verifies source and copied artifact checksums. Missing, modified,
or corrupt sources fail the entire export. Copies are staged beside the destination
and published together only after verification; existing destinations are preserved.
A failed export cleans up staging files. Keep source recordings stable during export.

The set's pinned `members[].review` remains the curation evidence. Individual
recordings carry current annotation history, which may include later reviews;
consumers must not substitute the latest review for the pinned selection label.
Original LeRobot video/Parquet chunks remain intact, potentially including frames
from unselected episodes. Apply the provenance episode ranges when preparing data.
Shared chunks may be copied once per selected experience. This export is a portable
recording bundle, not a privacy-redacted clip or a trainer-ready dataset.

## Dataset preparation

For supported imitation data, the optional [LeRobot preparation adapter](../../tools/dataset-preparation/README.md)
consumes this bundle and produces independent train/validation datasets, source
mappings, and a DataLoader validation report. The saved learning-set contract itself
remains unchanged. Preparation requires explicit episode assignments and does not
turn every curated member into a training example.
