# Humaan CLI

Offline import, inspection, and export of robot experiences. No account or local
server is required. Install from the repository root with a Rust toolchain:

```sh
cargo install --path apps/cli --locked
humaan --help
```

Try the included synthetic recording from the repository root:

```sh
cargo run -p humaan-cli -- project init my-robot
cargo run -p humaan-cli -- project show
cargo run -p humaan-cli -- import examples/recording
cargo run -p humaan-cli -- list
cargo run -p humaan-cli -- inspect example-001 --json
cargo run -p humaan-cli -- verify
cargo run -p humaan-cli -- export example-001 ./exported-recording
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
to override project discovery. Project and cloud placeholder commands ignore it.

`--json` emits an array of manifests for import, list, inspect, and export; errors go to stderr with
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

The reusable `humaan-engine` crate owns these operations. Live hardware capture,
cloud storage, and desktop integration are not implemented yet.

`humaan link`, `humaan push`, and `humaan pull` are placeholders that display
“Cloud linking, push, and pull are coming soon.” They do not access the network
or change local files and exit with code 1 because the operation is unavailable.
With `--json`, they emit an object with `status: "coming_soon"` and `message`.

Validation:

```sh
cargo check -p humaan-cli -p humaan-engine
cargo test -p humaan-cli -p humaan-engine
```
