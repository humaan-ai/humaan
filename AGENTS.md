# humaan

## Scope and architecture

- This repository owns the open-source desktop application and planned CLI, shared local engine, capture adapters, and recording specifications.
- The current Cargo workspace contains `crates/humaan-kernel` and `apps/desktop/src-tauri`. The desktop frontend uses Svelte, Vite, and Tauri in `apps/desktop`.
- The agreed direction is offline capture, inspection, export, and disk storage without an account or local backend server. Humaan-operated cloud storage is optional.
- Reserve Kernel for the future robot OS; use engine/capture terminology for new local application libraries. Existing `humaan-kernel` code predates this direction: inspect its responsibilities before changing it, and do not rename it incidentally.
- Website, hosted console, CMS, and cloud service work belongs in the separate `humaan.ai` repository.
- Read `README.md` and relevant source before editing. If available, `../PLAN.md` describes the target architecture; planned directories are not necessarily implemented.

## Commands and validation

- Run Rust commands from this repository root. For Rust changes, use `cargo fmt --all -- --check` and targeted `cargo check` / `cargo test` for affected packages; broaden to the workspace when shared behavior changes.
- Use Bun for the desktop frontend, preserving `apps/desktop/bun.lock`. Run frontend commands from `apps/desktop`: `bun run dev`, `bun run check`, and `bun run build`.
- Validate changed behavior with appropriate existing checks and tests. Documentation-only changes need review for accuracy and paths, not application builds.
- Preserve unrelated changes. Do not edit generated build output in `target/` or dependency directories.
- Report what changed, checks run, and any checks blocked by missing dependencies or platform requirements.
