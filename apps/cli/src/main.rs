use clap::{Parser, Subcommand};
use humaan_engine as engine;
use std::path::PathBuf;
mod progress;

#[derive(Parser)]
#[command(
    name = "humaan",
    version,
    about = "Import, inspect, and export robot experiences offline"
)]
struct Cli {
    /// Override the project's recording store
    #[arg(long, global = true)]
    store: Option<PathBuf>,
    /// Emit machine-readable JSON
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Manage a local project
    Project {
        #[command(subcommand)]
        command: ProjectCommand,
    },
    /// Import a Humaan recording or auto-detect a LeRobot v3.0 dataset
    // TODO: Add an explicit `--format` override if more adapters make detection ambiguous.
    Import { source: PathBuf },
    /// List local experiences and verify their artifacts
    List,
    /// Inspect and verify a local experience
    Inspect {
        /// Experience ID printed by `humaan import` or `humaan list` (not the project ID)
        id: String,
    },
    /// Check all local experiences and report every damaged entry
    Verify,
    /// Export original files to a new directory
    Export { id: String, destination: PathBuf },
    /// Link a local project to a cloud workspace and project (coming soon)
    Link,
    /// Push local experiences to the cloud (coming soon)
    Push,
    /// Pull experiences from the cloud (coming soon)
    Pull,
}

#[derive(Subcommand)]
enum ProjectCommand {
    /// Initialize a named project in the current directory
    Init { name: String },
    /// Show the current project's identity and location
    Show,
}

fn run(cli: Cli) -> engine::Result<std::process::ExitCode> {
    let current = std::env::current_dir()?;
    if let Command::Project { command } = &cli.command {
        let (root, project) = match command {
            ProjectCommand::Init { name } => {
                (current.clone(), engine::project::init(&current, name)?)
            }
            ProjectCommand::Show => engine::project::discover(&current)?
                .ok_or("No local project. Run `humaan project init <name>`.")?,
        };
        if cli.json {
            println!(
                "{}",
                serde_json::to_string_pretty(
                    &serde_json::json!({ "project": project, "root": root })
                )?
            );
        } else {
            println!("{}  id={}\n{}", project.name, project.id, root.display());
        }
        return Ok(std::process::ExitCode::SUCCESS);
    }
    let store = || -> engine::Result<PathBuf> {
        cli.store
            .clone()
            .map(Ok)
            .unwrap_or_else(|| engine::project::store(&current))
    };
    if matches!(&cli.command, Command::Verify) {
        let progress = progress::Progress::start("Verifying recordings", cli.json);
        let results = engine::verify(&store()?)?;
        drop(progress);
        let valid = results.iter().all(|result| result.valid);
        if cli.json {
            println!("{}", serde_json::to_string_pretty(&results)?);
        } else {
            for result in &results {
                match &result.error {
                    Some(error) => println!("FAIL {}: {}", result.id, error),
                    None => println!("OK   {}", result.id),
                }
            }
            println!(
                "Checked {} experiences; {} failed.",
                results.len(),
                results.iter().filter(|result| !result.valid).count()
            );
        }
        return Ok(if valid {
            std::process::ExitCode::SUCCESS
        } else {
            std::process::ExitCode::FAILURE
        });
    }
    let detailed = matches!(&cli.command, Command::Inspect { .. });
    let progress = match &cli.command {
        Command::Import { .. } => Some(progress::Progress::start(
            "Importing and verifying recordings",
            cli.json,
        )),
        Command::List => Some(progress::Progress::start(
            "Listing and verifying recordings",
            cli.json,
        )),
        Command::Inspect { .. } => Some(progress::Progress::start(
            "Inspecting and verifying recording",
            cli.json,
        )),
        Command::Export { .. } => Some(progress::Progress::start(
            "Exporting and verifying recording",
            cli.json,
        )),
        _ => None,
    };
    let (verb, experiences) = match cli.command {
        Command::Project { .. } | Command::Verify => unreachable!(),
        Command::Import { source } => ("Imported", engine::import_recordings(&store()?, &source)?),
        Command::List => ("", engine::list(&store()?)?),
        Command::Inspect { id } => (
            "Verified",
            vec![engine::inspect_experience(&store()?, &id)?],
        ),
        Command::Export { id, destination } => (
            "Exported",
            vec![engine::export(&store()?, &id, &destination)?],
        ),
        Command::Link | Command::Push | Command::Pull => {
            let message = "Cloud linking, push, and pull are coming soon.";
            if cli.json {
                println!(
                    "{}",
                    serde_json::json!({ "status": "coming_soon", "message": message })
                );
            } else {
                eprintln!("{message}");
            }
            return Ok(std::process::ExitCode::FAILURE);
        }
    };
    drop(progress);
    if cli.json {
        let mut output = serde_json::to_value(&experiences)?;
        if detailed {
            for (entry, experience) in output.as_array_mut().unwrap().iter_mut().zip(&experiences) {
                if let Some(details) = engine::lerobot_details(&store()?, experience)? {
                    entry["source"] = details;
                }
            }
        }
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else if experiences.is_empty() {
        println!("No local experiences. Import one with `humaan import <source>`.");
    } else {
        for experience in &experiences {
            println!(
                "{}{}  robot={}  task={}  artifacts={}",
                if verb.is_empty() {
                    String::new()
                } else {
                    format!("{verb} ")
                },
                experience.id,
                experience.robot,
                experience.task,
                experience.artifacts.len()
            );
            if detailed {
                if let Some(details) = engine::lerobot_details(&store()?, experience)? {
                    println!(
                        "  source=lerobot {}  episode={}  frames={}  fps={}  duration_s={}",
                        details["version"].as_str().unwrap_or("unknown"),
                        details["episode_index"],
                        details["frames"],
                        details["fps"],
                        details["duration_s"]
                    );
                    println!("  outcome=unknown  policy=unknown");
                    if let Some(videos) = details["videos"].as_object() {
                        for (camera, video) in videos {
                            println!(
                                "  camera={camera}  from={}s  to={}s",
                                video["from_timestamp"], video["to_timestamp"]
                            );
                        }
                    }
                }
                for artifact in &experience.artifacts {
                    println!(
                        "  {}  format={}  bytes={}\n    sha256={}",
                        artifact.path, artifact.format, artifact.bytes, artifact.sha256
                    );
                }
            }
        }
        if verb == "Imported" {
            println!(
                "Successfully imported {} experience{} into {}.",
                experiences.len(),
                if experiences.len() == 1 { "" } else { "s" },
                store()?.display()
            );
        }
    }
    Ok(std::process::ExitCode::SUCCESS)
}

fn main() -> std::process::ExitCode {
    match run(Cli::parse()) {
        Ok(status) => status,
        Err(error) => {
            eprintln!("humaan: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}
