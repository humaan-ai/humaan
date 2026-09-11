use std::fs;
use std::path::Path;
mod experiences;

#[derive(serde::Serialize, Clone)]
struct FileEntry {
    name: String,
    path: String,
    #[serde(rename = "isDir")]
    is_dir: bool,
    children: Option<Vec<FileEntry>>,
}

#[tauri::command]
async fn select_directory(title: Option<String>) -> Option<String> {
    let result = rfd::AsyncFileDialog::new()
        .set_title(title.as_deref().unwrap_or("Select Project Directory"))
        .pick_folder()
        .await;

    result.map(|file| file.path().to_string_lossy().into_owned())
}

#[tauri::command]
fn read_directory(path: String) -> Option<Vec<FileEntry>> {
    let path = Path::new(&path);
    read_dir_recursive(path, 0)
}

fn read_dir_recursive(path: &Path, depth: usize) -> Option<Vec<FileEntry>> {
    if depth > 4 {
        return None;
    }

    let entries = fs::read_dir(path).ok()?;
    let mut file_entries = Vec::new();

    for entry in entries {
        if let Ok(entry) = entry {
            let path_buf = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();

            // Skip common build/metadata directories to keep UI fast and prevent infinite loops
            if name == ".git"
                || name == "node_modules"
                || name == "target"
                || name == ".svelte-kit"
                || name == "dist"
                || name == "build"
                || name == ".DS_Store"
            {
                continue;
            }

            let is_dir = path_buf.is_dir();
            let children = if is_dir {
                read_dir_recursive(&path_buf, depth + 1)
            } else {
                None
            };

            file_entries.push(FileEntry {
                name,
                path: path_buf.to_string_lossy().into_owned(),
                is_dir,
                children,
            });
        }
    }

    // Sort: directories first, then files alphabetically
    file_entries.sort_by(|a, b| {
        if a.is_dir != b.is_dir {
            b.is_dir.cmp(&a.is_dir)
        } else {
            a.name.to_lowercase().cmp(&b.name.to_lowercase())
        }
    });

    Some(file_entries)
}

#[tauri::command]
fn read_file(path: String) -> Result<String, String> {
    fs::read_to_string(path).map_err(|e| e.to_string())
}

#[tauri::command]
fn write_file(path: String, content: String) -> Result<(), String> {
    fs::write(path, content).map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            experiences::experience_project,
            experiences::experience_list,
            experiences::experience_import,
            experiences::experience_inspect,
            experiences::experience_verify,
            experiences::experience_export,
            experiences::experience_replay,
            experiences::experience_reviews,
            experiences::experience_annotate,
            select_directory,
            read_directory,
            read_file,
            write_file
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
