# Recording format v1

An experience is a recorded robot attempt at a task. Its recording is a directory containing `manifest.json` and its declared artifacts.
The [test fixture helper](../../tests/support/recording.rs) generates a minimal
synthetic recording used by automated tests.

The manifest has these required fields:

| Field | Meaning |
| --- | --- |
| `version` | Integer `1` |
| `id` | Unique local ID: 1–128 ASCII letters, digits, `_`, or `-` |
| `robot` | Nonempty robot identifier |
| `task` | Nonempty task description |
| `artifacts` | Nonempty array of artifact descriptors |

Each artifact requires `path` (portable relative path using `/`), `format`
(nonempty format identifier), `bytes` (unsigned byte count), and `sha256`
(lowercase hex digest of the original file). Paths must be unique ignoring case,
without empty, `.` or `..` segments, backslashes, colons, or symbolic links.
`manifest.json` is reserved. Unknown manifest fields and unsupported versions
are rejected so metadata cannot silently acquire an unintended meaning.

Import and export copy declared artifacts, the original manifest, and optional review metadata described below, without
transcoding, and verify both source and copied data. Existing destinations are
never overwritten. Failed validation does not publish an experience. Keep source
recordings stable during import. The store is intended for local, trusted use,
not concurrent modification by other processes.

The initial catalog consists of experience directories named by ID. Listing verifies
every experience, and reports corruption as an error. `humaan verify` collects
per-entry validation results so damaged experiences do not hide healthy ones.
Hidden staging directories are
ignored. SQLite indexing, live capture adapters, and richer time, clock, unit,
coordinate-frame, and sensor metadata remain future work. This version does not
claim a universal robotics recording standard. The LeRobot adapter described below
preserves source-specific metadata within declared artifacts.

The manifest records artifact formats without defining their internal schemas.
Artifact payloads remain opaque to the core recording importer; adapters define
any format-specific interpretation.

## LeRobot import provenance

The LeRobot v3.0 adapter creates one v1 manifest per source episode. It preserves
the referenced original Parquet/video chunks and source metadata without transcoding.
An additional `lerobot-experience.json` artifact, with format
`application/vnd.humaan.lerobot-experience+json`, records:

- Source format/version, dataset content SHA-256, and episode index.
- Frame count, FPS, duration in seconds (frames/FPS), and feature schema.
- Data path and half-open global frame range (`dataset_from_index`, `dataset_to_index`).
- Each camera's video path and source `from_timestamp`/`to_timestamp` in seconds.
- Original episode metadata; normalized outcome and policy remain `null`.

The sidecar is checksummed like every other artifact. Files may span multiple
episodes: consumers must apply its frame/time ranges. The v1 manifest's shape stays
unchanged. Exports preserve these ranges and whole referenced files; they are Humaan
recordings, not reconstructed LeRobot training datasets. This is an import contract,
not a guarantee of compatibility with any training model.

## Human outcome annotations v1

Optional `annotations.json` stores mutable review metadata beside `manifest.json`;
it is not a checksummed recording artifact. Original artifacts and manifest bytes
are unchanged by reviews. New producers should reserve this filename for reviews.
Legacy recordings that already declare this path as an artifact remain copyable,
but cannot be annotated in place.

The document contains `version: 1`, `experience_id` matching the manifest, and
`revisions` in ascending contiguous order starting at 1. Each revision contains:

- `revision`, `saved_at_unix_ms` (UTC Unix milliseconds), and `source: "human"`.
- `review.outcome`: `success`, `failure`, `timeout`, `aborted`, or `unknown`.
- `review.assistance`: `yes`, `no`, or `unknown`.
- `review.review_state`: `unreviewed` or `reviewed`.
- `review.reviewer`: self-reported name, nonblank, at most 200 UTF-8 bytes.
- `review.notes`: at most 10,000 UTF-8 bytes.

The last revision is current. Missing annotations mean unknown outcome/assistance
and unreviewed state. Reviewed unknown is valid. Review state and assistance must
not be inferred from outcome. Saving requires the previously read revision number;
stale edits fail rather than overwrite a newer review. Writes use a temporary file
and atomic replacement, with a `.annotation-lock` file to serialize writers. After
an interrupted writer, remove a leftover lock only once no writer is running.
This local history is editable on disk and is not a tamper-proof audit log.

Import/export validate and preserve the entire review history. Malformed review
metadata fails copying rather than silently losing labels. Review metadata is not
covered by artifact integrity verification. Automatic outcomes and timed events
are not part of this annotation version.
