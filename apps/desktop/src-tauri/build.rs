#[path = "../../../scripts/version-guard.rs"]
mod version_guard;
fn main() {
    version_guard::check("studio", "../../..");
    tauri_build::build()
}
