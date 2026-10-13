# Humaan Studio

Tauri, SvelteKit, and TypeScript desktop application. The default **Experiences**
view uses `humaan-engine` directly, sharing project identity, recording formats,
and local storage with the CLI. No account or local backend server is required.

The frontend uses Svelte 5, SvelteKit 2, Tailwind CSS 4, and Vite 8.
Use Node.js 20.19+ or 22.12+ when running the Vite tooling through Node.
Type checks use TypeScript 7 via `svelte-check --tsgo`; the `typescript` 6
package remains installed for Svelte tooling compatibility, alongside the
`@typescript/native` alias for TypeScript 7. Both are intentional.

From this directory:

```sh
bun install
bun run tauri dev
```

`bun run dev` serves the frontend alone; project and recording operations require
Tauri's native process (`bun run tauri dev`).

## Studio navigation

After opening a project, the left activity bar provides **Project**, **Builder**,
**Simulation**, **Experiences**, and **Deployments**, with **Cloud resources** at the bottom.
Experiences and Loops use vertical navigation inside the Experiences panel.
Project, Simulation, Deployments, and Cloud resources currently have empty workspaces. The Settings icon opens a modal dialog with General, Tools, and About sections. The welcome screen stays free of project navigation.
Each view has a contextual sidebar, initially 260px wide. Drag its divider, or
focus the divider and use Left/Right arrows (Home/End for the limits), to resize it.
The activity bar is 48px wide and the footer retains the current project name.
Use the native **File → Open…** menu (**Cmd+O** on macOS, **Ctrl+O** elsewhere)
to open another project. On macOS, File is in the system menu bar. The welcome
screen also retains its Open project button. Menu requests respect busy operations
and unsaved review/curation drafts, with an explanation when switching is blocked.

Experiences opens on **Overview**, with catalog counts, review progress, final
outcomes, and recent reviews from existing metadata; it does not verify files.
The record list and filters stay visible beside Overview, Records, and the detail tab.
**Records** contains Import recordings / Refresh / Verify all, saved verification statuses,
and a TanStack Table with its own search, review, and verification filters. Sidebar
filters and table filters are independent in both directions. TanStack manages sorting state,
header toggles, and sorted rows using its alphanumeric and numeric comparators. Names show a short ID in small monospace text beneath them, with the full ID on hover. The default queue is
All experiences. Text sorts naturally (record 2 before record 10), durations sort
numerically. Headers cycle Default → Descending → Ascending → Default.
Reset sorting restores the original catalog order. Ascending status order is
Unreviewed then Reviewed, Yes then No, and Success, Failure, Timeout, then Aborted. Unknown/missing values stay last in both directions. Columns show
outcome, review state, human assistance, duration, and latest review date; Robot appears
only for mixed-robot projects. Rows open the single detail tab with click or Enter/Space.
Catalog loading batches duration reads from small source metadata files, without
reading media or verifying hashes. Missing or invalid duration displays an em dash;
source errors remain available on hover and in experience details.
Tabs use subtle background and text changes, without a blue active border.
Selecting a record opens one replaceable detail tab, for a maximum of three tabs.
Selecting the same record reuses it; a failed load keeps the previous record.
Closing the detail tab returns to Records. Unsaved reviews must be saved or discarded
before replacing or closing it; switching fixed tabs preserves the draft.
The selected tab persists while switching Studio sections and resets on project open.
Arrow keys and Home/End navigate the tab bar. Loop opens on **Overview**. Selecting
saved learning sets opens reusable, closable tabs; Create learning set opens one
**New learning set** draft tab. Closing an unsaved draft asks for confirmation.
Saving replaces its tab with the saved set. Closing a tab restores the previous
available tab, falling back to Overview.

Settings → General shows the project name, folder, and ID. Tools configures the
Rerun executable and provides dataset environment status/setup/repair. About shows
the installed version and a disabled Check for updates button because no updater
is configured. Closing Settings or pressing Escape preserves the current workspace,
settings draft, and running dataset setup.

Switching views preserves selected experiences, unsaved reviews, curation selections,
and active preparation assignments. Dots on the activity bar indicate unsaved review
or curation drafts. Save or discard those drafts before switching projects. Save or
discard a review before editing a learning set; a curation draft can remain open
while reviewing additional attempts. Active operations temporarily disable activity
navigation to preserve operation ordering.

## Local experience workflow

1. The welcome screen shows **Humaan Studio** with **Open Project** and **Create New**.
   Open a folder initialized by `humaan project init`, or choose **Create New**,
   enter a project name or full project path, optionally use the inline folder icon to choose a parent location,
   and click **Create new project**. The default location is `~/Humaan`. A new folder
   named after the project is created inside the selected location, with internal
   data under `<location>/<project name>/.humaan`. The dialog previews this destination.
   Existing folders are never reused during creation; open them or choose another name.
   **File → New Project…** (**Cmd+Shift+N** / **Ctrl+Shift+N**) opens the same dialog.
2. Choose **Import recordings** and select a Humaan recording or a complete LeRobot
   v3.0 dataset. Imports use the same validation and duplicate protection as the CLI.
3. Search experiences by task, robot, or ID. Select one to load its saved review and
   inspect source episode, duration, cameras, schema, and declared checksums without
   reading video or data payloads.
4. Use **Verify all** to save per-record integrity results. Use **Show failed** on
   the completion notification to filter the table; info icons open failure details.
5. Choose **Export**, then a parent directory. Humaan creates a new subdirectory
   named after the experience; existing exports are never overwritten.

Opening a project and refreshing the catalog read manifests, small source metadata,
and saved verification history, and compare file sizes/modification times without
reading video or data payloads. Selecting an experience reads its manifest, adapter
provenance, and annotations only. **Verify all** performs full checksum verification.
Integrity shows **Not checked**, **Verified**, **Failed**, or **Needs recheck** with
the saved check time. Results persist across sessions; they are not a guarantee that
files have not changed since the check.
Damaged payloads remain browsable. Import, replay, curation, export, and preparation
retain their integrity checks when they consume recording data.

Recording operations run on blocking workers so the UI remains responsive.
Longer operations show progress in the footer; click the progress text for details
and elapsed time. Project switching and recording actions are
unavailable until the current operation finishes. Import/export results and errors
are recorded in Notifications. Use Refresh after changing recordings through the CLI.
Opening or creating a project does not show a progress banner or success message;
errors appear as persistent notifications. The footer shows the active project name, with its local
folder available on hover.

## Status and notifications

Action buttons immediately show a spinner and an operation label, and disable repeat clicks.
**Opened in Rerun** appears in the footer and as a completion toast.
Verification notifications summarize passed/failed counts and provide **Show failed**
(or **Show changed**) to filter Records. Results are shown in the table, not a banner. Ordinary
review saves retain the inline confirmation beside the review controls. Import,
export, verification, dataset preparation, and tool-setup results use compact
bottom-right notifications without moving the workspace. Export/preparation
notifications provide **Copy path**; operation failures provide expandable details
and **Copy details**. Validation guidance remains next to the relevant form.

Success toasts expire after four seconds and footer messages after five seconds of active viewing;
the timer pauses while Studio is unfocused, hidden, hovered, or the message contains
keyboard focus. Warnings/errors remain until dismissed or viewed in history.
The footer bell opens the latest 50 events for this project session, including
dismissed notifications and output paths. Clicking progress opens running-operation
details in the same panel. History clears on project open/switch or explicit Clear
history, and is not persisted to disk. Dismissing a popup does not delete its history.
There are no fabricated progress percentages or unsupported cancellation controls.

## Outcome review

The experience list opens on the **All experiences** queue. Filter by review state,
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
training, and cloud operations are not implemented here.
Policy identity remains unknown.
Studio focuses on importing, reviewing, curating, and preparing robot experiences.
The legacy file editor, generic 3D viewer, assembly/simulation navigation, and
placeholder account/configuration/debug controls have been removed. Dataset tool
setup and repair are available in Settings → Tools and within dataset preparation.

For LeRobot import/export limitations, see [the CLI guide](../cli/README.md).
In particular, shared chunks can contain multiple episodes; exports preserve whole
original files and episode ranges, not reconstructed training datasets.

## Curate a learning set

Choose **Loop** in the activity bar, then **Create learning set**. Name the set, state what the robot
should learn, and enter the curator name. Filter reviewed attempts by outcome or
human assistance, select relevant experiences, and explain why each one belongs.
Selections remain when filters change. **Remove** adjusts the draft; **Discard
selection** clears it. Switching to Experiences preserves the draft so more attempts
can be reviewed. Save or discard the draft before changing projects.

**Save learning set** verifies selected recordings and saves a project-local,
immutable selection under `.humaan/learning-sets/`. Every member pins the original
manifest fingerprint, artifact descriptors, and selected human review revision.
Unreviewed entries, stale review revisions, duplicate IDs, missing rationale, and
corrupt recordings are rejected before publication. A reviewed unknown outcome
can be included with an explicit reason. No outcome is automatically declared
useful or training-ready.

Saved sets show their learning goal, members, rationale, and pinned review labels.
Later review edits do not modify existing sets. Create another set for a revised
selection. **Export experience files** creates `learning-set-<id>/` in the chosen
parent folder, containing `learning-set.json` and `recordings/<experience-id>/`
with every selected recording's original manifest, artifacts, and current review
history. The learning-set manifest retains the pinned reviews used for selection.
Export rejects missing/corrupt recordings and manifests changed since selection.
All copies are verified before the complete folder is published; failures clean up
staged copies and existing destinations are never overwritten.

**Export selection manifest only** remains available for metadata-only export.
Neither export extracts correction segments or creates a trainer-ready dataset.
LeRobot shared chunks stay whole, and may include other episodes. Consumers must
apply the episode ranges in each recording's LeRobot provenance.
VLM evaluation and automated candidate ranking are not implemented.

## Prepare training data

Saved learning sets offer **Prepare training dataset**. Explicitly assign reviewed,
unassisted successes to train and validation, confirm whole-trajectory suitability,
and choose an output folder. The optional LeRobot environment extracts selected
RGB frames and float32 state/action vectors, writes independent LeRobot v3 image
datasets, and validates every sample with the official reader and PyTorch DataLoader.

Choose **Set up dataset tools** in the preparation panel for a one-time download.
Humaan manages Python and packages in its app-data folder; no manual Python setup
is required. Setup progress and **Repair dataset tools** are available there.
See [dataset preparation setup and constraints](../../tools/dataset-preparation/README.md)
for supported platforms/inputs, custom environments, and the standalone bundle command.
Output includes a preparation report with exclusions and source mapping. No model
training is launched. Assisted trajectories and correction segments are unsupported.

## Builder

Open a project and select **Builder**. Add box, cylinder, or sphere modules, or use
**Import URDF** to select a description and its mesh files together. **Import robot
folder** supports directory layouts and lets you choose a URDF when several exist.
Supported meshes are STL and OBJ; package and relative mesh references are resolved
against the selected files. Missing or ambiguous meshes fail the import without
replacing the current assembly. Imports are limited to 64 MB of selected robot files.

Select a link in the tree or 3D viewport to configure mass, center of mass, inertia,
visual/collision geometry, mounting frames, sensors, and its incoming joint. Positions
use metres and rotations use radians. Attach a library module by choosing the parent
link and the mounting point on each side; mounting-frame rotations are included in
the resulting joint transform. Mounts define placement at attachment time; edit the
joint origin to reposition an existing connection. Save any complete assembly as a
reusable module, then attach independent copies elsewhere.

**Save version** writes a new immutable snapshot, including embedded meshes, under
`.humaan/builder/versions`. **Save assembly as module** writes to
`.humaan/builder/modules`; it does not mark the working robot as saved. Snapshots
are limited to 100 MB and are written atomically without replacing older versions.
The latest version opens with the project; choose another from **Saved versions**.
Switching workspaces preserves the draft. Replacing the robot requires confirmation
when changes are unsaved, and opening another project is blocked until they are saved
or discarded through **New** or loading a saved version.

The viewport supports orbit, zoom, pan, picking, fit-to-view, and collision geometry.
**Test** checks assembly structure and previews joint motion; it is not a physics
simulation. Fixed, revolute, continuous, and prismatic joints are supported. Xacro
must be expanded first; mimic, planar, and floating joints are not yet supported.
OBJ materials/textures are not imported. The original URDF is retained in imported
snapshots, but Gazebo/transmission/ros2_control extensions are not mapped into runtime
behavior. Configure sensors and hardware mappings explicitly. AI Chat is available;
AI-generated Builder edits still require provider integration.

Builder checks:

```sh
bun run test:builder
bunx playwright install chromium
bun run test:e2e
```

Browser tests exercise the real UI with an isolated Tauri IPC adapter. Native snapshot
storage is checked separately with `cargo test -p desktop builder::tests --lib` from
the repository root.

## Open in Rerun

Install the standalone **Rerun 0.37.2** viewer, matching the pinned Rust SDK:

```sh
cargo install rerun-cli --version 0.37.2 --locked
```

Alternatively use a matching prebuilt viewer from the
[official releases](https://github.com/rerun-io/rerun/releases/tag/0.37.2).
Humaan first uses the executable saved in Settings → Tools, then
`HUMAAN_RERUN_PATH`, then `~/.cargo/bin/rerun`, then `rerun` on PATH.
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

## Saved verification status

Studio's Verify all saves the latest per-record result and timestamp atomically to
`.humaan/recordings/.verification.json`, outside the recording artifacts. CLI
verification remains read-only. The table restores these results on project open;
info icons show the time and explanation on hover/focus and open a details dialog.
The verification filter and Show failed action apply only to the Records table;
they preserve the sidebar list, search, and review queue.

A metadata fingerprint tracks the manifest contents plus artifact sizes and modification
times. Open/Refresh marks a changed record **Needs recheck** without hashing media.
Changes detected during verification also require a recheck. **Verified** means the
record passed at the displayed time, not a live guarantee: modifications that preserve
file size and timestamps require Verify all to detect. Reviews do not invalidate file
integrity status. Damaged manifests remain visible as unreadable-recording rows.

## Releases

Studio releases use `studio-v<version>` independently of the CLI. The
[macOS workflow](../../.github/workflows/macos-signing.yml) builds and notarizes
Apple Silicon Studio artifacts.

## Native macOS icon

`src-tauri/icons/Humaan.icon` is the editable Icon Composer project used for the
native macOS icon. Keep its `icon.json` and referenced SVG assets together.
`icons/legacy/` holds the previous artwork as PNG/ICNS fallbacks for development
and older systems; these are separate from the layered icon.

Packaging requires full Xcode with Icon Composer support. Command Line Tools
alone do not include the required asset compiler. After installing Xcode and
opening it to finish setup, select it with:

```sh
sudo xcode-select --switch /Applications/Xcode.app/Contents/Developer
xcrun actool --version
```

Then run `bun run tauri build --bundles app,dmg -- --locked` from this directory.
The packaging preflight rejects a missing asset compiler because Tauri otherwise
logs an error and can produce a fallback-only bundle. Confirm the resulting app
contains `Contents/Resources/Assets.car` and inspect its icon in Finder before
signing and publishing. The native icon was compiled successfully with Xcode 27; the asset catalog contains light, dark, and tinted layered variants. Finder appearance still needs a visual check of the rebuilt package.


## Local simulation (Milestone 2)

Save a robot version in Builder, open **Test**, resolve simulation errors, then choose **Open in Simulation**. Simulation also provides a saved-version selector and refresh action. Component-specific Builder findings open the affected link in Configure.

The first local adapter uses the official [MuJoCo Python bindings](https://mujoco.readthedocs.io/en/stable/python.html). Open **Settings → Tools → MuJoCo → Install MuJoCo**, or use the **Simulation settings** icon beside Refresh. One-time setup downloads an isolated Python 3.11 environment and MuJoCo 3.2.7 into Humaan's local app data folder, verifies a physics step, and selects the managed runtime automatically. It does not modify system Python. Setup requires internet access; simulation works offline afterward.

The Tools card shows installation phases, readiness, errors, **Refresh MuJoCo status**, and **Repair MuJoCo**. A failed repair keeps the previously published runtime intact. Managed installation supports Apple Silicon macOS, x86-64 Linux, and x86-64 Windows, using the same checksum-verified setup-tool bootstrap as dataset tools. Installation tests currently cover Apple Silicon macOS.

For an existing environment, expand **Advanced runtime** under **Settings → Tools → MuJoCo** and enter its Python executable's absolute path. Leave this field blank to use managed MuJoCo. **Use installed MuJoCo** in Settings switches back from a custom environment. Advanced manual setup remains available:

```sh
python3 -m venv .venv
.venv/bin/python -m pip install mujoco==3.2.7
```

On Windows, the environment executable is `.venv\Scripts\python.exe`.

Select duration, gravity, and joint position targets, then run. Playback is available after computation finishes. Completed and failed runs remain in the sidebar. Each run stores its source version ID, a complete robot snapshot, configuration, and result under `.humaan/simulation/runs`. Completed runs additionally retain engine version, generated MJCF, frames, joint positions, contact counts, and adapter assumptions. Reloading results does not require the original Builder snapshot to remain present.

Current physics scope:

- Fixed root at the saved robot placement, floor at 0 m, gravity along negative Z, and contact friction 0.8.
- Fixed, revolute, continuous, and prismatic joints; finite joint limits and effort-limited position servos (kp=20, kv=2). Hardware velocity limits are not enforced.
- Explicit mass, center of mass, and full inertia for non-root links; missing physical properties must be supplied rather than inferred.
- Primitive collisions and STL/OBJ mesh convex hulls. Links without collision geometry are explicitly reported; sensors are not simulated.
- 2 ms physics step, 20 Hz recorded playback, 0.1–10 simulated seconds, at most 256 links, and a 60-second worker timeout.
- A completed run indicates physics execution, not task success or hardware deployment readiness.

Validation and tests:

```sh
bun test src/lib/builder/model.test.ts src/lib/simulation/model.test.ts
bunx playwright test tests/builder.spec.ts
```

From the repository root, using the environment's Python path:

```sh
.venv/bin/python apps/desktop/src-tauri/src/simulation_worker_test.py
cargo test -p desktop --lib
HUMAAN_TEST_PYTHON=/absolute/path/to/.venv/bin/python cargo test -p desktop simulation::tests::real_mujoco_worker_round_trip --lib -- --ignored
```

Browser tests use an isolated IPC adapter. Python tests exercise real physics; the opt-in Rust integration test exercises the embedded worker and persistent result storage.

The managed-installer integration test downloads into a temporary directory and requires internet access:

```sh
cargo test -p humaan-engine simulation_environment::tests::install_and_probe_real_runtime --lib -- --ignored
```

Builder’s **Place on ground** now updates the robot’s saved placement. Save a new version and select it in Simulation to apply that placement; **Restore origin** resets it. Existing saved runs keep their original results. Explode movements remain view-only.
