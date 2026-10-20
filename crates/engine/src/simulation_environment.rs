//! Isolated, app-owned MuJoCo runtime. A verified environment is published only
//! after setup completes, so failed repairs leave the previous runtime intact.
use crate::{invalid, Result};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};
const VERSION: &str = "3.2.7";
const PROBE: &str = "import mujoco; assert mujoco.__version__ == '3.2.7'; m=mujoco.MjModel.from_xml_string('<mujoco/>'); d=mujoco.MjData(m); mujoco.mj_step(m,d); print('ready')";
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub ready: bool,
    pub installing: bool,
    pub phase: String,
    pub version: String,
    pub python: Option<String>,
    pub error: Option<String>,
}
#[derive(Serialize, Deserialize)]
struct Installed {
    directory: String,
    version: String,
}
fn python_in(environment: &Path) -> PathBuf {
    environment.join(if cfg!(windows) {
        ".venv/Scripts/python.exe"
    } else {
        ".venv/bin/python"
    })
}
pub fn managed_python(root: &Path) -> Result<PathBuf> {
    let record: Installed = serde_json::from_slice(
        &fs::read(root.join("current.json"))
            .map_err(|_| invalid("Install MuJoCo in Settings → Tools first."))?,
    )?;
    crate::validate_id(&record.directory)?;
    if record.version != VERSION {
        return Err(invalid("Update MuJoCo in Settings → Tools."));
    }
    let path = python_in(&root.join("environments").join(record.directory));
    if !path.is_file() {
        return Err(invalid("MuJoCo needs repair. Open Settings → Tools."));
    }
    Ok(path)
}
fn lock(root: &Path) -> Result<fs::File> {
    fs::create_dir_all(root)?;
    let file = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(root.join("setup.lock"))?;
    file.try_lock()
        .map_err(|_| invalid("MuJoCo installation is already running."))?;
    Ok(file)
}
fn phase(root: &Path, text: &str) -> Result<()> {
    fs::write(root.join("phase.txt"), text)?;
    Ok(())
}
fn checked(command: &mut Command, log: &Path, timeout: Duration) -> Result<()> {
    let output = fs::OpenOptions::new().create(true).append(true).open(log)?;
    let mut child = command
        .stdin(Stdio::null())
        .stdout(output.try_clone()?)
        .stderr(output)
        .spawn()?;
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => return Ok(()),
            Ok(Some(status)) => {
                let mut file = fs::File::open(log)?;
                let length = file.metadata()?.len();
                file.seek(SeekFrom::Start(length.saturating_sub(5000)))?;
                let mut tail = Vec::new();
                file.read_to_end(&mut tail)?;
                return Err(invalid(format!(
                    "MuJoCo setup failed ({status}). {}",
                    String::from_utf8_lossy(&tail)
                )));
            }
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(error.into());
            }
            _ => {}
        }
        if start.elapsed() > timeout {
            let _ = child.kill();
            let _ = child.wait();
            return Err(invalid(
                "MuJoCo setup timed out. Check your connection and retry.",
            ));
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}
fn probe(python: &Path) -> Result<()> {
    let log = tempfile::NamedTempFile::new()?;
    checked(
        Command::new(python).args(["-I", "-c", PROBE]),
        log.path(),
        Duration::from_secs(20),
    )
}
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
    let python = managed_python(root).ok();
    let probe_error = if !installing {
        python
            .as_ref()
            .and_then(|path| probe(path).err())
            .map(|e| e.to_string())
    } else {
        None
    };
    let ready = python.is_some() && probe_error.is_none();
    Ok(Status {
        ready,
        installing,
        phase: if installing {
            fs::read_to_string(root.join("phase.txt"))
                .unwrap_or_else(|_| "Installing MuJoCo…".into())
        } else if ready {
            format!("MuJoCo {VERSION} is ready")
        } else if python.is_some() {
            "MuJoCo needs repair".into()
        } else {
            "MuJoCo is not installed".into()
        },
        version: VERSION.into(),
        python: python.map(|p| p.to_string_lossy().into_owned()),
        error: if installing {
            None
        } else {
            probe_error.or_else(|| fs::read_to_string(root.join("error.txt")).ok())
        },
    })
}
pub fn install(root: &Path) -> Result<Status> {
    let guard = lock(root)?;
    let log = root.join("setup.log");
    fs::write(&log, "")?;
    let _ = fs::remove_file(root.join("error.txt"));
    let mut attempted = None;
    let result = (|| -> Result<()> {
        phase(root, "Downloading verified setup tools")?;
        let uv = crate::dataset_environment::bootstrap(root, &log)?;
        let name = uuid::Uuid::new_v4().to_string();
        let environment = root.join("environments").join(&name);
        fs::create_dir_all(&environment)?;
        attempted = Some(environment.clone());
        phase(root, "Installing isolated Python 3.11")?;
        checked(
            Command::new(&uv)
                .args([
                    "venv",
                    "--no-config",
                    "--managed-python",
                    "--python",
                    "3.11",
                ])
                .arg(environment.join(".venv"))
                .env("UV_CACHE_DIR", root.join("cache"))
                .env("UV_PYTHON_INSTALL_DIR", root.join("python"))
                .env("UV_PYTHON_INSTALL_BIN", "0")
                .env("UV_PYTHON_DOWNLOADS", "automatic")
                .env_remove("VIRTUAL_ENV"),
            &log,
            Duration::from_secs(600),
        )?;
        phase(root, "Downloading and installing MuJoCo 3.2.7")?;
        checked(
            Command::new(&uv)
                .args([
                    "pip",
                    "install",
                    "--no-config",
                    "--only-binary",
                    ":all:",
                    "--python",
                ])
                .arg(python_in(&environment))
                .arg(format!("mujoco=={VERSION}"))
                .env("UV_CACHE_DIR", root.join("cache"))
                .env_remove("VIRTUAL_ENV"),
            &log,
            Duration::from_secs(600),
        )?;
        phase(root, "Verifying MuJoCo with a physics step")?;
        probe(&python_in(&environment))?;
        let mut marker = tempfile::NamedTempFile::new_in(root)?;
        marker.write_all(&serde_json::to_vec(&Installed {
            directory: name,
            version: VERSION.into(),
        })?)?;
        marker.as_file().sync_all()?;
        marker.persist(root.join("current.json"))?;
        Ok(())
    })();
    if let Err(error) = result {
        if let Some(path) = attempted {
            let _ = fs::remove_dir_all(path);
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
    fn status_lock_and_invalid_markers() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        assert!(!status(root).unwrap().ready);
        let guard = lock(root).unwrap();
        phase(root, "Downloading").unwrap();
        assert!(status(root).unwrap().installing);
        assert!(lock(root).is_err());
        drop(guard);
        assert!(!status(root).unwrap().installing);
        for (directory, version) in [("../escape", VERSION), ("old", "0"), ("missing", VERSION)] {
            fs::write(
                root.join("current.json"),
                serde_json::to_vec(&Installed {
                    directory: directory.into(),
                    version: version.into(),
                })
                .unwrap(),
            )
            .unwrap();
            assert!(managed_python(root).is_err());
        }
    }
    #[test]
    #[ignore = "downloads an isolated Python and MuJoCo runtime"]
    fn install_and_probe_real_runtime() {
        let temp = tempfile::tempdir().unwrap();
        let status = install(temp.path()).unwrap();
        assert!(status.ready);
        assert!(!status.installing);
        probe(&managed_python(temp.path()).unwrap()).unwrap();
    }
}
