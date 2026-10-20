// Cargo reads package metadata before build scripts run, so fail rather than
// modifying a manifest too late to affect the executable's version.
pub fn check(component: &str, relative_root: &str) {
    let root = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join(relative_root);
    let path = root.join("version.json");
    println!("cargo:rerun-if-changed={}", path.display());
    let versions: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&path).expect("Read version.json from the repository root"),
    ).expect("Parse version.json");
    let expected = versions[component].as_str().expect("Missing component version in version.json");
    let actual = std::env::var("CARGO_PKG_VERSION").unwrap();
    assert_eq!(actual, expected, "Package version differs from version.json. Run `bun run version:sync` from the repository root before building.");
}
