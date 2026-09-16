# Humaan

**Building blocks for Physical AI.**

Humaan Studio is a desktop workspace for assembling machine intelligence.
Humaan CLI provides terminal tools for working with robot experiences. Both share
a local Rust engine and work without cloud signup.

Humaan is in early alpha. Interfaces and data formats may change.

## Available today

- **Studio:** import and inspect experiences, review outcomes and assistance,
  curate learning sets, and prepare supported LeRobot training datasets.
- **CLI:** initialize local projects and import, list, inspect, verify, and export
  Humaan recordings and LeRobot v3.0 datasets.
- **Shared engine:** recording validation, local storage, review history, learning
  sets, and dataset preparation. Studio can launch a separately installed Rerun
  viewer for replay.

Live capture and robot control, model training, policy evaluation, deployment,
and cloud transfer remain planned. The `humaan-kernel` crate is a placeholder,
not an implemented robot operating system.

## Get started

To build the CLI from source, install Rust and run from the repository root:

```sh
cargo install --path apps/cli --locked
humaan --help
```

To develop Studio, install Rust and Bun, then run:

```sh
cd apps/desktop
bun install --frozen-lockfile
bun run tauri dev
```

Native desktop dependencies vary by platform. The current macOS packaging
workflow targets Apple Silicon and requires full Xcode for the native icon.
See the [Studio guide](apps/desktop/README.md) and [CLI guide](apps/cli/README.md)
for setup and supported workflows.

## Repository layout

| Path | Purpose |
| --- | --- |
| `apps/desktop` | Humaan Studio: Tauri, SvelteKit, and TypeScript |
| `apps/cli` | Humaan CLI |
| `crates/engine` | Shared local Rust engine |
| `crates/humaan-kernel` | Reserved robot OS placeholder |
| `specs` | Recording and learning-set format contracts |
| `tools/dataset-preparation` | Python tools embedded by the engine |
| `tests/support` | Shared synthetic test fixtures |

## Releases and validation

Studio and CLI have independent versions: `studio-v<version>` and
`cli-v<version>`. The current GitHub workflows produce signed and notarized
Apple Silicon test artifacts; automated public release publishing and additional
platform packages are not implemented yet.

- [Studio signing workflow](.github/workflows/macos-signing.yml)
- [CLI signing workflow](.github/workflows/cli-macos-signing.yml)
- [Recording format](specs/recording-format/README.md)
- [Learning-set format](specs/learning-set-format/README.md)

Keep Cargo commands at the repository root and Bun commands in `apps/desktop`.
Each application guide lists its validation commands.

## License

Humaan's original code and documentation in this repository are licensed under
[Apache License 2.0](LICENSE), unless otherwise noted. Third-party dependencies
retain their respective licenses. The license does not grant rights to use
Humaan trademarks except as provided in its terms.
