use tauri::{
    menu::{Menu, MenuItem, MenuItemKind, PredefinedMenuItem, Submenu},
    AppHandle, Emitter, Manager, Wry,
};

const VIEWS: &[(&str, &str)] = &[
    ("project", "Project"),
    ("builder", "Builder"),
    ("simulation", "Simulation"),
    ("experiences", "Experiences"),
    ("deployments", "Deployments"),
    ("cloud", "Compute"),
    ("catalog", "Catalog"),
];

fn item(
    app: &AppHandle,
    id: &str,
    label: &str,
    enabled: bool,
    shortcut: Option<&str>,
) -> tauri::Result<MenuItem<Wry>> {
    MenuItem::with_id(app, id, label, enabled, shortcut)
}

pub fn build(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    let menu = Menu::default(app)?;
    let new_project = item(
        app,
        "new-project",
        "New Project…",
        true,
        Some("CmdOrCtrl+Shift+N"),
    )?;
    let open = item(
        app,
        "open-project",
        "Open Project…",
        true,
        Some("CmdOrCtrl+O"),
    )?;
    let settings = item(app, "settings", "Settings…", true, Some("CmdOrCtrl+,"))?;
    let recent = item(app, "open-recent", "Open Recent…", true, None)?;
    let updates = item(app, "check-updates", "Check for update...", true, None)?;
    let mut has_file = false;
    let mut view = None;
    for entry in menu.items()? {
        let MenuItemKind::Submenu(submenu) = entry else {
            continue;
        };
        let label = submenu.text()?;
        if label == "File" {
            submenu.prepend(&recent)?;
            submenu.prepend(&open)?;
            submenu.prepend(&new_project)?;
            has_file = true;
        }
        #[cfg(target_os = "macos")]
        if label == app.package_info().name {
            submenu.insert(&updates, 1)?;
            submenu.insert(&settings, 2)?;
        }
        #[cfg(not(target_os = "macos"))]
        if label == "Edit" {
            // Tauri's predefined Undo/Redo are unsupported on Windows and Linux.
            submenu.remove_at(0)?;
            submenu.remove_at(0)?;
            submenu.insert(&item(app, "edit-undo", "Undo", true, Some("Ctrl+Z"))?, 0)?;
            submenu.insert(
                &item(app, "edit-redo", "Redo", true, Some("Ctrl+Shift+Z"))?,
                1,
            )?;
            submenu.append(&PredefinedMenuItem::separator(app)?)?;
            submenu.append(&settings)?;
        }
        if label == "View" {
            view = Some(submenu.clone());
        }
        #[cfg(target_os = "linux")]
        if label == "Window" {
            while !submenu.items()?.is_empty() {
                submenu.remove_at(0)?;
            }
            submenu.append(&item(app, "window-minimize", "Minimize", true, None)?)?;
            submenu.append(&item(
                app,
                "window-maximize",
                "Maximize / Restore",
                true,
                None,
            )?)?;
            submenu.append(&item(app, "window-close", "Close Window", true, None)?)?;
        }
        if label == "Help" {
            for (position, (id, title)) in [
                ("help-docs", "Documentation"),
                ("help-issues", "Report an Issue"),
                ("help-repository", "GitHub Repository"),
            ]
            .into_iter()
            .enumerate()
            {
                submenu.insert(&item(app, id, title, true, None)?, position)?;
            }
            #[cfg(not(target_os = "macos"))]
            {
                submenu.insert(&PredefinedMenuItem::separator(app)?, 3)?;
                submenu.insert(&updates, 4)?;
            }
        }
    }
    if !has_file {
        let file = Submenu::with_items(app, "File", true, &[&new_project, &open, &recent])?;
        file.append(&PredefinedMenuItem::separator(app)?)?;
        file.append(&item(
            app,
            "window-close",
            "Close Window",
            true,
            Some("Ctrl+W"),
        )?)?;
        file.append(&item(app, "quit", "Quit", true, Some("Ctrl+Q"))?)?;
        menu.prepend(&file)?;
    }
    let view = if let Some(view) = view {
        view
    } else {
        let view = Submenu::new(app, "View", true)?;
        let position = menu.items()?.iter().position(|entry| {
            matches!(entry, MenuItemKind::Submenu(submenu) if submenu.text().ok().as_deref() == Some("Window"))
        }).unwrap_or(menu.items()?.len());
        menu.insert(&view, position)?;
        view
    };
    for (position, (id, title)) in VIEWS.iter().enumerate() {
        view.insert(
            &item(app, &format!("view-{id}"), title, false, None)?,
            position,
        )?;
    }
    view.insert(&PredefinedMenuItem::separator(app)?, VIEWS.len())?;
    view.insert(
        &item(app, "toggle-chat", "Toggle Chat", true, None)?,
        VIEWS.len() + 1,
    )?;
    view.insert(
        &item(app, "toggle-terminal", "Toggle Bottom Panel", true, None)?,
        VIEWS.len() + 2,
    )?;
    view.insert(&PredefinedMenuItem::separator(app)?, VIEWS.len() + 3)?;
    #[cfg(not(target_os = "macos"))]
    view.append(&item(
        app,
        "window-fullscreen",
        "Full Screen",
        true,
        Some("F11"),
    )?)?;
    Ok(menu)
}

#[tauri::command]
pub fn set_project_menu_enabled(window: tauri::WebviewWindow, enabled: bool) -> Result<(), String> {
    let menu = window.menu().ok_or("Window menu is unavailable")?;
    for entry in menu.items().map_err(|e| e.to_string())? {
        if let MenuItemKind::Submenu(submenu) = entry {
            for (id, _) in VIEWS {
                if let Some(MenuItemKind::MenuItem(item)) = submenu.get(&format!("view-{id}")) {
                    item.set_enabled(enabled).map_err(|e| e.to_string())?;
                }
            }
        }
    }
    Ok(())
}

fn help_url(id: &str) -> Option<&'static str> {
    match id {
        "help-docs" => Some("https://humaan.ai/docs"),
        "help-issues" => Some("https://github.com/humaan-ai/humaan/issues/new/choose"),
        "help-repository" => Some("https://github.com/humaan-ai/humaan"),
        _ => None,
    }
}

pub fn handle(app: &AppHandle, id: &str) {
    if let Some(url) = help_url(id) {
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(error) = crate::updates::open_browser(url.into()).await {
                let _ = app.emit_to("main", "studio:menu-error", error);
            }
        });
        return;
    }
    if id.starts_with("window-") {
        if let Some(window) = app.get_webview_window("main") {
            let result = match id {
                "window-close" => window.close(),
                "window-minimize" => window.minimize(),
                "window-maximize" => window.is_maximized().and_then(|maximized| {
                    if maximized {
                        window.unmaximize()
                    } else {
                        window.maximize()
                    }
                }),
                "window-fullscreen" => window
                    .is_fullscreen()
                    .and_then(|fullscreen| window.set_fullscreen(!fullscreen)),
                _ => return,
            };
            if let Err(error) = result {
                let _ = app.emit_to("main", "studio:menu-error", error.to_string());
            }
        }
        return;
    }
    if id == "quit" {
        app.exit(0);
        return;
    }
    let event = match id {
        "open-project" => "studio:open-project",
        "open-recent" => "studio:open-recent",
        "new-project" => "studio:new-project",
        "check-updates" => "studio:check-updates",
        "settings" => "studio:settings",
        "toggle-chat" | "toggle-terminal" | "edit-undo" | "edit-redo" => "studio:menu-action",
        _ if VIEWS.iter().any(|(view, _)| id == format!("view-{view}")) => "studio:menu-action",
        _ => return,
    };
    if let Err(error) = app.emit_to("main", event, id) {
        eprintln!("Could not request menu action: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn help_links_are_fixed_and_unknown_actions_are_rejected() {
        assert_eq!(help_url("help-docs"), Some("https://humaan.ai/docs"));
        assert_eq!(
            help_url("help-issues"),
            Some("https://github.com/humaan-ai/humaan/issues/new/choose")
        );
        assert_eq!(
            help_url("help-repository"),
            Some("https://github.com/humaan-ai/humaan")
        );
        assert_eq!(help_url("https://evil.example"), None);
    }
}
