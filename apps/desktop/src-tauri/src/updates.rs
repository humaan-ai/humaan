use std::process::Command;

#[tauri::command]
pub fn studio_update_target() -> String {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    if os == "linux" {
        let package = if std::env::var_os("APPIMAGE").is_some() {
            "appimage"
        } else {
            "deb"
        };
        format!("{os}-{arch}-{package}")
    } else {
        format!("{os}-{arch}")
    }
}

fn valid_download_url(url: &str) -> bool {
    let Some(path) =
        url.strip_prefix("https://github.com/humaan-ai/humaan/releases/download/studio-v")
    else {
        return false;
    };
    let Some((version, filename)) = path.split_once('/') else {
        return false;
    };
    let safe = |value: &str| {
        !value.is_empty()
            && value
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c))
    };
    safe(version)
        && safe(filename)
        && filename.starts_with(&format!("Humaan_{version}_"))
        && [".dmg", ".deb", ".AppImage", "-setup.exe"]
            .iter()
            .any(|suffix| filename.ends_with(suffix))
}

#[tauri::command]
pub async fn open_studio_download(url: String) -> Result<(), String> {
    if !valid_download_url(&url) {
        return Err("Invalid Studio download URL".into());
    }
    open_browser(url).await
}

pub(crate) async fn open_browser(url: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        #[cfg(target_os = "macos")]
        let result = Command::new("open").arg(&url).status();
        #[cfg(target_os = "windows")]
        let result = Command::new("rundll32.exe")
            .arg("url.dll,FileProtocolHandler")
            .arg(&url)
            .status();
        #[cfg(target_os = "linux")]
        let result = Command::new("xdg-open").arg(&url).status();
        #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
        return Err("Opening links is unsupported on this platform".into());
        #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
        match result {
            Ok(status) if status.success() => Ok(()),
            Ok(_) => Err("Could not open the link in your browser".into()),
            Err(error) => Err(format!("Could not open the link: {error}")),
        }
    })
    .await
    .map_err(|error| error.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_opens_studio_release_downloads() {
        let base = "https://github.com/humaan-ai/humaan/releases/download/studio-v0.0.1-alpha.3/";
        assert!(valid_download_url(&format!(
            "{base}Humaan_0.0.1-alpha.3_x64-setup.exe"
        )));
        for tail in [
            "../../evil.exe",
            "Humaan_0.0.1-alpha.2_x64-setup.exe",
            "Humaan_0.0.1-alpha.3_x64-setup.exe?next=evil",
            "Humaan_0.0.1-alpha.3_script.sh",
        ] {
            assert!(!valid_download_url(&format!("{base}{tail}")));
        }
        assert!(!valid_download_url("https://evil.example/update.exe"));
        assert!(!valid_download_url("file:///tmp/update.exe"));
    }
}
