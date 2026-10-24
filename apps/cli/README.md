# Humaan CLI

A local, offline CLI for managing robot experiences. No account or local
server is required. Install from the repository root with a Rust toolchain:

```sh
cargo install --path apps/cli --locked
humaan --help
```

The CLI version comes from `cli` in the repository-root `version.json`.
After editing it, run `bun run version:sync` at the root and commit the updated
Cargo manifests and lockfile with `version.json`. Use `bun run build:cli` for a
release build with automatic synchronization. Direct Cargo builds and installs
reject stale package versions; run the sync command first. CI requires committed
versions to match and checks release tags against `cli-v<version>`.

In an interactive terminal, import, list, inspect, verify, and export show an
animated activity indicator with elapsed time while processing. It clears before
results or errors appear. This indicates ongoing work, not a completion percentage.
Animation is disabled with `--json` or when stderr is redirected.

With an existing Humaan recording directory, try the following from a disposable
project directory. Replace the source path and experience ID with your own:

```sh
humaan project init my-robot
humaan project show
humaan import /path/to/recording
humaan list
humaan inspect EXPERIENCE_ID --json
humaan verify
humaan export EXPERIENCE_ID ./exported-recording
```

`humaan project init <name>` initializes the current directory as a local project.
It saves a stable UUID, display name, and metadata version in `.humaan/project.json`.
It requires no workspace or account, preserves existing recordings, and refuses
to overwrite an existing project. To create a project in a new directory, create
that directory and change into it before running init.

`humaan project show` displays the nearest project's name, ID, and root directory.
With `--json`, project commands emit an object containing `project` and `root`.
Recording commands search the current directory and its parents for project
metadata and use that project's `.humaan/recordings` store. Without a project,
they retain the current-directory `.humaan/recordings` default. Invalid project
metadata produces an error. Use `--store /path/to/recordings` on recording commands
to override project discovery. Project commands ignore it.

`--json` emits an array of manifests for import, list, inspect, and export. LeRobot
inspection also includes a `source` object with episode provenance and feature metadata. Errors go to stderr with
exit code 1. Invalid command arguments use exit code 2. Reimporting an existing
ID or exporting to an existing destination fails without overwriting it.

Prepare recordings using the [v1 recording format](../../specs/recording-format/README.md).
Every read verifies artifact sizes and SHA-256 checksums, including `list`.
`inspect <id>` also shows each artifact's path, format, byte size, and checksum.
If a damaged experience prevents listing, run `humaan verify` to check all entries
and report failures without changing any files. It exits with code 1 if any entry
fails, or 0 if all entries pass (including an empty store). `verify --json` emits
an array of `{ "id", "valid", "error" }` results, with `error` omitted for valid
entries. An unreadable store fails immediately; hidden staging entries are ignored.

## Importing LeRobot recordings

Record with LeRobot, then import a complete local **LeRobot v3.0** dataset:

```sh
humaan import /path/to/lerobot-dataset
humaan list
humaan inspect <imported-experience-id>
humaan inspect <imported-experience-id> --json
```

Import detects `manifest.json` for Humaan v1 or `meta/info.json` for LeRobot.
No `--format` option is needed or currently implemented. Other LeRobot versions
are rejected with a supported-version message. LeRobot must finish recording or
downloading before import; partial downloads and episode subsets are not supported.

The adapter reads `meta/tasks.parquet`, `meta/episodes/**/*.parquet`, and referenced
data/video chunks. It validates episode/frame totals, indices, increasing timestamps,
task references, declared data columns, and video range metadata. Missing files
fail the import before experiences are published. Video decoding, sensor alignment,
physical units, and model-training compatibility are not validated.

Each episode becomes an experience. Inspection shows source episode, frames, FPS,
duration (frames/FPS), and camera time ranges. JSON inspection includes the original
feature schema and episode metadata. Outcomes and policy identity remain unknown;
the importer does not infer success or learning improvement.

Shared chunks are copied once into staging and hard-linked between experiences in
the local store, never linked to the source dataset. Treat imported artifacts as
immutable: editing a shared chunk affects every imported experience using it.
Checksums detect such changes. The source can be moved after import. IDs combine a
dataset content digest and episode index; importing the same complete dataset again
fails without overwriting anything. Changing dataset contents creates a different
identity. Validation and staging finish before publication; publication is not
crash-atomic across an entire batch, so do not interrupt it or modify the store concurrently.

Export preserves the original referenced chunk files plus Humaan's manifest and
episode provenance. A shared chunk may contain other episodes; export does not trim
videos or produce a standalone LeRobot training dataset. Dataset preparation is available in Studio for supported learning sets; the CLI
does not yet expose a preparation command. Large imports require space for the copied files, and verification
reads shared files again for each experience.

The reusable `humaan-engine` crate owns these operations. Live hardware capture,
and cloud storage are not implemented yet. Studio already uses the shared engine
for local project, experience, review, curation, and preparation workflows.

Validation:

```sh
cargo check -p humaan-cli -p humaan-engine
cargo test -p humaan-cli -p humaan-engine
```

## Releases

CLI releases use `cli-v<version>` independently of Studio. The
[macOS workflow](../../.github/workflows/cli-macos.yml) builds and notarizes
Apple Silicon CLI artifacts.

The [Linux workflow](../../.github/workflows/cli-linux.yml) tests and builds an
x86-64 CLI on Ubuntu 22.04, then packages `humaan` and `LICENSE` in
`humaan-cli_<version>_linux_x86_64.tar.gz` with a SHA-256 checksum file.
Run it manually with a blank `release_tag` to download build artifacts, or push a
matching `cli-v<version>` tag to attach the downloads to an existing GitHub release.
Manual runs can also specify an existing release tag.

To build locally on Linux, install Rust and a C/C++ build toolchain, then run from
the repository root:

```sh
cargo build -p humaan-cli --release --locked
./target/release/humaan --help
```

## Windows distribution

The [Windows workflow](../../.github/workflows/cli-windows.yml) tests and builds
with the MSVC toolchain on `windows-2022`. It packages `humaan.exe` and `LICENSE`
in `humaan-cli_<version>_windows_x86_64.zip`, checks the extracted executable with
`--version` and `--help`, and generates `humaan-cli-windows-x86_64-SHA256SUMS`.

Run it manually with a blank `release_tag` for Actions artifacts only. For release
assets, create a GitHub release (draft is supported) for a matching `cli-v<version>`
tag that includes this workflow, then run it with that tag. Tag pushes also trigger
builds; the release must exist before upload. Reruns replace assets with matching
names. CLI and Studio versions remain independent.

Extract the ZIP, then run `.\humaan.exe --help` in PowerShell. Add the containing
folder to your user `PATH` to invoke `humaan` from other directories. The executable
is unsigned; Windows code signing is not configured.

For a local build, install Rust's MSVC toolchain and Visual Studio C++ Build Tools
with the Windows SDK, then run from the repository root:

```powershell
cargo build -p humaan-cli --release --locked --target x86_64-pc-windows-msvc
.\target\x86_64-pc-windows-msvc\release\humaan.exe --help
```
