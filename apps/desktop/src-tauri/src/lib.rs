mod builder;
mod deployments;
mod policies;
mod cloud;
mod experiences;
mod project_browser;
mod simulation;
mod training;

use tauri::{
    menu::{Menu, MenuItem, MenuItemKind, Submenu},
    Emitter,
};

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
        .menu(|app| {
            let menu = Menu::default(app)?;
            let open = MenuItem::with_id(app, "open-project", "Open…", true, Some("CmdOrCtrl+O"))?;
            let new_project = MenuItem::with_id(
                app,
                "new-project",
                "New Project…",
                true,
                Some("CmdOrCtrl+Shift+N"),
            )?;
            // Keep native Edit/Window/application menus and add project actions to File.
            for item in menu.items()? {
                if let MenuItemKind::Submenu(submenu) = item {
                    if submenu.text()? == "File" {
                        submenu.prepend(&open)?;
                        submenu.prepend(&new_project)?;
                        return Ok(menu);
                    }
                }
            }
            // The default Linux menu does not include File.
            menu.prepend(&Submenu::with_items(
                app,
                "File",
                true,
                &[&new_project, &open],
            )?)?;
            Ok(menu)
        })
        .on_menu_event(|app, event| {
            let name = match event.id().as_ref() {
                "open-project" => "studio:open-project",
                "new-project" => "studio:new-project",
                _ => return,
            };
            if let Err(error) = app.emit_to("main", name, ()) {
                eprintln!("Could not request project action: {error}");
            }
        })
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
