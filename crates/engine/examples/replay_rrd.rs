//! Prepare a replay without launching a GUI, useful for debugging integrations.
fn main() -> humaan_engine::Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 3 {
        return Err("usage: replay_rrd <recordings-store> <experience-id> <new-output.rrd>".into());
    }
    humaan_engine::replay::save(
        std::path::Path::new(&args[0]),
        &args[1],
        std::path::Path::new(&args[2]),
    )?;
    println!("Prepared {}", args[2]);
    Ok(())
}
