//! Exercise the same setup/status implementation used by the desktop.
fn main() -> humaan_engine::Result<()> {
    let mut args = std::env::args().skip(1);
    let action = args
        .next()
        .ok_or("usage: dataset_tools <status|install> <app-data-directory>")?;
    let directory = std::path::PathBuf::from(args.next().ok_or("missing app-data-directory")?);
    let status = match action.as_str() {
        "install" => humaan_engine::dataset_environment::install(&directory)?,
        "status" => humaan_engine::dataset_environment::status(&directory)?,
        _ => return Err("expected status or install".into()),
    };
    println!("{}", serde_json::to_string_pretty(&status)?);
    Ok(())
}
