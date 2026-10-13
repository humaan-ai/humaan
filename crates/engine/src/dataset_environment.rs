//! App-owned Python setup. No shell profiles or system Python are modified.
use crate::{Result, invalid};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

const PROJECT: &str = include_str!("../../../tools/dataset-preparation/pyproject.toml");
const LOCK: &str = include_str!("../../../tools/dataset-preparation/uv.lock");
const UV_VERSION: &str = "0.9.11";
const PROBE: &str = "import importlib.metadata as m; import sys; assert (3,11) <= sys.version_info[:2] < (3,14); assert m.version('lerobot') == '0.4.4'; assert m.version('torch') == '2.10.0'; from lerobot.datasets.lerobot_dataset import LeRobotDataset; from torch.utils.data import DataLoader; import av, pyarrow; print('ready')";

#[derive(Debug, Serialize)]
pub struct Status {
    pub ready: bool,
    pub custom: bool,
    pub installing: bool,
    pub phase: String,
    pub error: Option<String>,
}
#[derive(Serialize, Deserialize)]
struct Installed {
    directory: String,
    fingerprint: String,
}
fn fingerprint() -> String {
    hex::encode(Sha256::digest(format!(
        "{PROJECT}\n{LOCK}\n{UV_VERSION}\n{}-{}",
        std::env::consts::OS,
        std::env::consts::ARCH
    )))
}
fn python_in(directory: &Path) -> PathBuf {
    directory.join(if cfg!(windows) {
        ".venv/Scripts/python.exe"
    } else {
        ".venv/bin/python"
    })
}
pub fn managed_python(root: &Path) -> Result<PathBuf> {
    let installed: Installed = serde_json::from_slice(&fs::read(root.join("current.json"))?)?;
    crate::validate_id(&installed.directory)?;
    if installed.fingerprint != fingerprint() {
        return Err(invalid("Dataset tools need an update. Run setup again."));
    }
    let python = python_in(&root.join("environments").join(installed.directory));
    if !python.is_file() {
        return Err(invalid("Dataset tools are missing. Run setup again."));
    }
    Ok(python)
}
fn set_phase(root: &Path, phase: &str) -> Result<()> {
    fs::write(root.join("phase.txt"), phase)?;
    Ok(())
}
fn acquire(root: &Path) -> Result<fs::File> {
    fs::create_dir_all(root)?;
    let file = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(root.join("setup.lock"))?;
    file.try_lock()
        .map_err(|_| invalid("Dataset tools setup is already running."))?;
    Ok(file)
}
/// Readiness means a successful probe for the current pinned environment. A
/// preparation attempt probes again, catching damaged dependencies before copying.
pub fn status(root: &Path) -> Result<Status> {
    let installing = if root.join("setup.lock").exists() {
        let file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(root.join("setup.lock"))?;
        file.try_lock().is_err()
    } else {
        false
    };
    let ready = managed_python(root).is_ok();
    Ok(Status {
        ready,
        custom: false,
        installing,
        phase: if installing {
            fs::read_to_string(root.join("phase.txt"))
                .unwrap_or_else(|_| "Setting up dataset tools".into())
        } else if ready {
            "Dataset tools are ready".into()
        } else {
            "Dataset tools are not installed".into()
        },
        error: if installing {
            None
        } else {
            fs::read_to_string(root.join("error.txt")).ok()
        },
    })
}
fn checked(command: &mut Command, log: &Path) -> Result<()> {
    let output = fs::OpenOptions::new().create(true).append(true).open(log)?;
    let exit = command
        .stdin(Stdio::null())
        .stdout(output.try_clone()?)
        .stderr(output)
        .status()?;
    if !exit.success() {
        let text = fs::read_to_string(log).unwrap_or_default();
        let tail: String = text
            .chars()
            .rev()
            .take(5000)
            .collect::<String>()
            .chars()
            .rev()
            .collect();
        return Err(invalid(format!("Setup command failed ({exit}). {tail}")));
    }
    Ok(())
}
pub fn probe(python: &Path, cache: &Path) -> Result<()> {
    let output = Command::new(python)
        .args(["-I", "-c", PROBE])
        .env("HF_HUB_OFFLINE", "1")
        .env("HF_DATASETS_OFFLINE", "1")
        .env("HF_HOME", cache)
        .output()?;
    if !output.status.success() {
        return Err(invalid(format!(
            "Dataset tools need repair: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    Ok(())
}

/// Checksums are release-pinned, not downloaded alongside an untrusted executable.
fn platform() -> Result<(&'static str, &'static str)> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => Ok((
            "aarch64-apple-darwin",
            "594d9f4cfbd21d5a2f34b0352bf423066a9dab1733c90b5d40e3e227506deb03",
        )),
        ("linux", "x86_64") => Ok((
            "x86_64-unknown-linux-gnu",
            "817c0722b437b4b45b9a7e0231616a09db76bab1b8d178ba7a9680c690db19f0",
        )),
        ("windows", "x86_64") => Ok((
            "x86_64-pc-windows-msvc",
            "45a3ff2a68c246ed9fd2d9df032496c1beebe480357f356ac25d2cb144884c30",
        )),
        _ => Err(invalid(
            "Managed tools support Apple Silicon macOS, x86-64 Linux, and x86-64 Windows. Use a compatible custom Python environment on other platforms.",
        )),
    }
}
pub(crate) fn bootstrap(root: &Path, log: &Path) -> Result<PathBuf> {
    let (target, expected) = platform()?;
    let tools = root.join(format!("uv-{UV_VERSION}"));
    let executable = tools.join(if cfg!(windows) { "uv.exe" } else { "uv" });
    if executable.is_file()
        && Command::new(&executable)
            .arg("--version")
            .output()
            .is_ok_and(|o| {
                o.status.success()
                    && String::from_utf8_lossy(&o.stdout).starts_with(&format!("uv {UV_VERSION} "))
            })
    {
        return Ok(executable);
    }
    let temporary = tempfile::Builder::new()
        .prefix(".uv-download-")
        .tempdir_in(root)?;
    let extension = if cfg!(windows) { "zip" } else { "tar.gz" };
    let archive = temporary.path().join(format!("uv.{extension}"));
    let url = format!(
        "https://github.com/astral-sh/uv/releases/download/{UV_VERSION}/uv-{target}.{extension}"
    );
    checked(
        Command::new("curl")
            .args([
                "--fail",
                "--location",
                "--proto",
                "=https",
                "--proto-redir",
                "=https",
                "--connect-timeout",
                "30",
                "--max-time",
                "600",
                "--output",
            ])
            .arg(&archive)
            .arg(url),
        log,
    )?;
    let mut file = fs::File::open(&archive)?;
    let mut hash = Sha256::new();
    let mut buffer = [0; 65536];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    if hex::encode(hash.finalize()) != expected {
        return Err(invalid("Setup tool download failed checksum verification."));
    }
    // Extract only the expected executable, never arbitrary archive paths.
    let member = if cfg!(windows) {
        "uv.exe".into()
    } else {
        format!("uv-{target}/uv")
    };
    checked(
        Command::new("tar")
            .arg("-xf")
            .arg(&archive)
            .arg("-C")
            .arg(temporary.path())
            .arg(&member),
        log,
    )?;
    fs::create_dir_all(&tools)?;
    let published = tempfile::NamedTempFile::new_in(&tools)?;
    fs::copy(temporary.path().join(member), published.path())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(published.path(), fs::Permissions::from_mode(0o755))?;
    }
    published.as_file().sync_all()?;
    published.persist(&executable)?;
    Ok(executable)
}

pub fn install(root: &Path) -> Result<Status> {
    let guard = acquire(root)?;
    let log = root.join("setup.log");
    fs::write(&log, "")?;
    let _ = fs::remove_file(root.join("error.txt"));
    let mut attempted = None;
    let result = (|| -> Result<()> {
        set_phase(root, "Downloading setup tools")?;
        let uv = bootstrap(root, &log)?;
        let name = uuid::Uuid::new_v4().to_string();
        let environment = root.join("environments").join(&name);
        fs::create_dir_all(&environment)?;
        attempted = Some(environment.clone());
        // Do not relocate a virtualenv: its interpreter paths are absolute.
        fs::write(environment.join("pyproject.toml"), PROJECT)?;
        fs::write(environment.join("uv.lock"), LOCK)?;
        set_phase(
            root,
            "Installing Python and dataset packages (first setup may take several minutes)",
        )?;
        checked(
            Command::new(&uv)
                .args([
                    "sync",
                    "--locked",
                    "--no-dev",
                    "--no-config",
                    "--managed-python",
                    "--python",
                    "3.11",
                    "--project",
                ])
                .arg(&environment)
                .env("UV_PROJECT_ENVIRONMENT", environment.join(".venv"))
                .env("UV_CACHE_DIR", root.join("cache"))
                .env("UV_PYTHON_INSTALL_DIR", root.join("python"))
                .env("UV_PYTHON_INSTALL_BIN", "0")
                .env("UV_PYTHON_DOWNLOADS", "automatic")
                .env_remove("VIRTUAL_ENV"),
            &log,
        )?;
        set_phase(root, "Checking dataset tools")?;
        probe(&python_in(&environment), &root.join("hf-cache"))?;
        let mut ready = tempfile::NamedTempFile::new_in(root)?;
        ready.write_all(&serde_json::to_vec(&Installed {
            directory: name,
            fingerprint: fingerprint(),
        })?)?;
        ready.as_file().sync_all()?;
        ready.persist(root.join("current.json"))?;
        Ok(())
    })();
    if let Err(error) = result {
        if let Some(directory) = attempted {
            let _ = fs::remove_dir_all(directory);
        }
        let _ = fs::write(root.join("error.txt"), error.to_string());
        return Err(error);
    }
    drop(guard);
    status(root)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn setup_lock_is_released_automatically_and_status_tracks_readiness() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("tools");
        assert!(!status(&root).unwrap().ready);
        let guard = acquire(&root).unwrap();
        set_phase(&root, "Installing packages").unwrap();
        assert!(status(&root).unwrap().installing);
        assert!(acquire(&root).is_err());
        drop(guard);
        assert!(!status(&root).unwrap().installing);
        assert!(acquire(&root).is_ok());
    }

    #[test]
    fn changed_lockfile_missing_python_and_traversal_never_report_ready() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let environment = root.join("environments/test");
        let python = python_in(&environment);
        fs::create_dir_all(python.parent().unwrap()).unwrap();
        fs::write(&python, "test marker").unwrap();
        let marker = root.join("current.json");
        let write = |directory: &str, fingerprint: String| {
            fs::write(
                &marker,
                serde_json::to_vec(&Installed {
                    directory: directory.into(),
                    fingerprint,
                })
                .unwrap(),
            )
            .unwrap()
        };
        write("test", fingerprint());
        assert!(status(root).unwrap().ready);
        write("test", "previous dependency lock".into());
        assert!(!status(root).unwrap().ready);
        write("../escape", fingerprint());
        assert!(managed_python(root).is_err());
        write("test", fingerprint());
        fs::remove_file(python).unwrap();
        assert!(!status(root).unwrap().ready);
    }
}
