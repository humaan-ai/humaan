# Humaan desktop

Tauri, SvelteKit, and TypeScript desktop application. The default **Experiences**
view uses `humaan-engine` directly, sharing project identity, recording formats,
and local storage with the CLI. No account or local backend server is required.

From this directory:

```sh
bun install
bun run tauri dev
```

`bun run dev` serves the frontend alone; project and recording operations require
Tauri's native process (`bun run tauri dev`).

## Local experience workflow

1. Choose **Open project** and select a folder initialized by `humaan project init`.
   Alternatively, enter a new project name and choose a folder to initialize.
2. Choose **Import recordings** and select a Humaan recording or a complete LeRobot
   v3.0 dataset. Imports use the same validation and duplicate protection as the CLI.
3. Search experiences by task, robot, or ID. Select one to verify its artifacts and
   inspect source episode, duration, cameras, schema, and checksums when available.
4. Use **Verify all** to report damaged entries individually. Healthy entries in the
   verification report remain inspectable even if corruption prevents catalog listing.
5. Choose **Export**, then a parent directory. Humaan creates a new subdirectory
   named after the experience; existing exports are never overwritten.

Opening a project and refreshing the catalog read manifests only, without scanning
video or data files. Catalog entries do not imply verified integrity; selecting an
experience or choosing Verify all performs full checksum verification.

Recording operations run on blocking workers so the UI remains responsive and
shows activity and elapsed time. Project switching and recording actions are
unavailable until the current operation finishes. Import/export success and errors
are displayed explicitly. Use Refresh after changing recordings through the CLI.

## Outcome review

The experience list opens on the **Unreviewed** queue. Filter by review state,
outcome, or human assistance, and search by task, robot, or ID. Select an experience,
optionally inspect it in Rerun, then enter its outcome, assistance, reviewer name,
and notes. **Save and next** advances through the current queue without opening Rerun
for every attempt. Save or discard edits before changing selection or project.

Success, failure, timeout, aborted, and unknown describe the final attempt outcome.
Human assistance is independent. A reviewed experience can still have an unknown
outcome. Labels are human judgments; integrity verification does not establish success.
Reviewer names are self-reported local attribution, not authenticated identities.
Review history is retained locally and included in native import/export.

Automatic evaluators, previews, bulk labeling, timestamped event annotations,
learning-set selection, training, and cloud operations are not implemented here.
Policy identity remains unknown.
The existing file-editor, assembly, and simulation shell remains accessible through
the activity bar; it does not represent implemented learning-loop capabilities.

For LeRobot import/export limitations, see [the CLI guide](../cli/README.md).
In particular, shared chunks can contain multiple episodes; exports preserve whole
original files and episode ranges, not reconstructed training datasets.

## Open in Rerun

Install the standalone **Rerun 0.37.2** viewer, matching the pinned Rust SDK:

```sh
cargo install rerun-cli --version 0.37.2 --locked
```

Alternatively use a matching prebuilt viewer from the
[official releases](https://github.com/rerun-io/rerun/releases/tag/0.37.2).
Humaan looks at `HUMAAN_RERUN_PATH`, then `~/.cargo/bin/rerun`, then `rerun` on PATH.
If needed, set `HUMAAN_RERUN_PATH` to the executable before starting the desktop.
No Python environment is needed. Missing or mismatched viewers produce a setup error.

Select an imported LeRobot experience, then choose **Open in Rerun**. Humaan verifies
the artifacts, prepares a temporary RRD using the Rust SDK, and launches a separate
viewer process. Preparation shows activity and elapsed time. The temporary file is
removed after the viewer exits normally; an abrupt desktop exit may leave it in the
OS temporary directory.

The `experience_time` timeline contains only the selected episode's video-frame
references and state/action samples. Camera presentation timestamps are offset by
the source episode's video start; data timestamps remain episode-relative. Original
video chunks are embedded intact, so the RRD is not a privacy-redacted episode clip.
State/action vectors are plotted with source channel labels; units, coordinate
frames, and robot geometry are not inferred. Rerun handles playback and layout.

Video decoding depends on the viewer/platform. Native H.264/H.265/VP8/VP9 playback
may require FFmpeg 5.1+ on PATH; see [Rerun video support](https://rerun.io/docs/concepts/logging-and-ingestion/video).
The first adapter supports LeRobot v3.0 video assets and one-dimensional numeric
state/action vectors. Other shapes produce an explicit error. Native Humaan v1
recordings without LeRobot provenance do not yet have a replay adapter.

For a headless replay export, from the repository root:

```sh
cargo run -p humaan-engine --features rerun --example replay_rrd -- \
  /path/to/.humaan/recordings <experience-id> /tmp/new-experience.rrd
```

## Validation

```sh
bun run check
bun run build
```

From the repository root:

```sh
cargo test -p desktop --lib --locked
cargo check --workspace --locked
cargo fmt --all -- --check
```
