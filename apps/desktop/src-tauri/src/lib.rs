mod builder;
mod builder_project;
mod deployments;
mod policies;
mod cloud;
mod experiences;
mod project_browser;
mod simulation;
mod training;
mod updates;
mod menus;

use tauri::{
    menu::MenuItemKind,
};
#[cfg(target_os = "linux")]
use tauri::Manager;

#[tauri::command]
fn uses_custom_titlebar() -> bool {
    cfg!(target_os = "linux")
}

#[tauri::command]
fn show_titlebar_icon() -> bool {
    !cfg!(target_os = "macos")
}

#[tauri::command]
fn app_menu_labels(window: tauri::WebviewWindow) -> Result<Vec<String>, String> {
    let menu = window.menu().ok_or("Window menu is unavailable")?;
    menu.items()
        .map_err(|error| error.to_string())?
        .into_iter()
        .filter_map(|item| match item {
            MenuItemKind::Submenu(submenu) => Some(submenu.text()),
            _ => None,
        })
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn show_app_menu(window: tauri::WebviewWindow, label: String, x: f64) -> Result<(), String> {
    let menu = window.menu().ok_or("Window menu is unavailable")?;
    for item in menu.items().map_err(|error| error.to_string())? {
        if let MenuItemKind::Submenu(submenu) = item {
            if submenu.text().map_err(|error| error.to_string())? == label {
                return window
                    .popup_menu_at(&submenu, tauri::LogicalPosition::new(x, 40.0))
                    .map_err(|error| error.to_string());
            }
        }
    }
    Err(format!("Menu {label} is unavailable"))
}

#[tauri::command]
async fn select_directory(title: Option<String>) -> Option<String> {
    let result = rfd::AsyncFileDialog::new()
        .set_title(title.as_deref().unwrap_or("Select Project Directory"))
        .pick_folder()
        .await;

    result.map(|file| file.path().to_string_lossy().into_owned())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|_app| {
            #[cfg(target_os = "linux")]
            if let Some(window) = _app.get_webview_window("main") {
                window.set_decorations(false)?;
                // Keep the attached menu and its accelerators, but open it from
                // the custom title bar instead of showing a second menu row.
                window.hide_menu()?;
            }
            Ok(())
        })
        .menu(menus::build)
        .on_menu_event(|app, event| menus::handle(app, event.id().as_ref()))
        .invoke_handler(tauri::generate_handler![
            uses_custom_titlebar,
            show_titlebar_icon,
            updates::studio_update_target,
            updates::open_studio_download,
            menus::set_project_menu_enabled,
            app_menu_labels,
            show_app_menu,
            experiences::experience_project,
            experiences::experience_list,
            experiences::experience_import,
            experiences::experience_inspect,
            experiences::experience_verify,
            experiences::experience_export,
            experiences::experience_replay,
            experiences::experience_reviews,
            experiences::experience_annotate,
            experiences::learning_set_list,
            experiences::learning_set_create,
            experiences::learning_set_export,
            experiences::learning_set_export_files,
            experiences::learning_set_prepare,
            experiences::dataset_tools_status,
            experiences::dataset_tools_install,
            simulation::simulation_tools_status,
            simulation::simulation_tools_install,
            cloud::cloud_connect,
            cloud::cloud_jobs,
            cloud::cloud_quote,
            cloud::cloud_launch,
            cloud::cloud_job_action,
            training::training_catalog,
            training::training_create,
            training::training_load,
            training::training_run,
            simulation::simulation_list,
            simulation::simulation_load,
            simulation::simulation_delete,
            simulation::simulation_run,
            policies::policy_list,
            policies::policy_load,
            policies::policy_promote,
            policies::policy_check,
            policies::policy_assignment,
            policies::policy_assign,
            deployments::deployment_list,
            deployments::deployment_load,
            deployments::deployment_save,
            deployments::deployment_sample,
            builder::builder_list,
            builder_project::builder_project,
            builder_project::builder_add_robot,
            builder_project::builder_import_robot,
            builder_project::builder_draft_load,
            builder_project::builder_draft_save,
            builder_project::builder_activate_robot,
            builder_project::builder_robot_files,
            builder_project::builder_read_robot_file,
            builder::builder_load,
            builder::builder_save,
            builder::builder_delete_version,
            project_browser::project_entries,
            project_browser::project_create_entry,
            project_browser::project_read_file,
            project_browser::project_save_file,
            select_directory
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
