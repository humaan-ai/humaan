# Recording format v1

An experience is a recorded robot attempt at a task. Its recording is a directory containing `manifest.json` and its declared artifacts.
See [the sample recording](../../examples/recording/manifest.json) for a complete,
importable example with synthetic joint state and action data.

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

Import and export copy only declared artifacts and the original manifest, without
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
claim interoperability with an existing robotics recording standard.

The sample `state.jsonl` has one synthetic record. `timestamp_ns` is elapsed time
from the experience start; `position_rad` and `action_rad` are joint position and
command arrays in radians. Other artifact formats remain opaque to the engine.
