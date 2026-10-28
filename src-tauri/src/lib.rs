mod platform;

use bfm_core::import::{import_paths as core_import, refresh_detection, restore_trashed, trash_fonts};
use bfm_core::{store, FontRecord, ImportReport, Library};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, State, WindowEvent};

/// Where everything lives: `<home>/Buni Font Manager`.
struct Paths {
    root: PathBuf,
}

impl Paths {
    fn library_dir(&self) -> PathBuf {
        self.root.join("Library")
    }
    fn library_file(&self) -> PathBuf {
        self.root.join("library.json")
    }
    fn trash_dir(&self) -> PathBuf {
        self.root.join(".trash")
    }
}

struct AppData {
    paths: Paths,
    lib: Mutex<Library>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StateDto {
    library: Library,
    library_dir: String,
    version: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ImportResult {
    report: ImportReport,
    library: Library,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DeleteResult {
    removed: Vec<FontRecord>,
    library: Library,
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

fn save(data: &AppData, lib: &Library) -> Result<(), String> {
    store::save(&data.paths.library_file(), lib).map_err(|e| format!("Could not save the library: {e}"))
}

#[tauri::command]
fn get_state(app: AppHandle, data: State<AppData>) -> StateDto {
    StateDto {
        library: data.lib.lock().unwrap().clone(),
        library_dir: data.paths.library_dir().to_string_lossy().to_string(),
        version: app.package_info().version.to_string(),
    }
}

/// Imports dropped or picked files. Adds them to `collection` when one is given.
#[tauri::command]
async fn import_paths(data: State<'_, AppData>, paths: Vec<String>, collection: Option<String>, origin: String) -> Result<ImportResult, String> {
    let inputs: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
    let mut lib = data.lib.lock().unwrap();
    let report = core_import(&mut lib, &inputs, &data.paths.library_dir(), &origin, now());
    if let Some(name) = collection.map(|c| c.trim().to_string()).filter(|c| !c.is_empty()) {
        lib.add_to_collection(&name, &report.font_ids);
    }
    let new_paths: Vec<String> = lib.fonts.iter().filter(|f| report.added.contains(&f.id)).map(|f| f.path.clone()).collect();
    platform::activate(&new_paths);
    save(&data, &lib)?;
    Ok(ImportResult { report, library: lib.clone() })
}

/// Saves edits made in the interface (collections, favourites, tags, sample texts).
/// Font files and activation are not changed here.
#[tauri::command]
fn save_library(data: State<AppData>, library: Library) -> Result<(), String> {
    let mut lib = data.lib.lock().unwrap();
    // Keep file-level facts from our own copy; take everything editable from the interface.
    let mut merged = library;
    for f in merged.fonts.iter_mut() {
        if let Some(ours) = lib.font(&f.id) {
            f.path = ours.path.clone();
            f.active = ours.active;
        }
    }
    merged.fonts.retain(|f| lib.font(&f.id).is_some());
    merged.detection = lib.detection;
    *lib = merged;
    save(&data, &lib)
}

#[tauri::command]
fn set_active(data: State<AppData>, ids: Vec<String>, active: bool) -> Result<Library, String> {
    let mut lib = data.lib.lock().unwrap();
    let mut paths = Vec::new();
    for f in lib.fonts.iter_mut() {
        if ids.contains(&f.id) && f.active != active {
            f.active = active;
            paths.push(f.path.clone());
        }
    }
    if active {
        platform::activate(&paths);
    } else {
        platform::deactivate(&paths);
    }
    save(&data, &lib)?;
    Ok(lib.clone())
}

/// Deletes fonts from the library. Files go to a trash folder so the action can be undone
/// until the app restarts.
#[tauri::command]
fn delete_fonts(data: State<AppData>, ids: Vec<String>) -> Result<DeleteResult, String> {
    let mut lib = data.lib.lock().unwrap();
    let active_paths: Vec<String> = lib.fonts.iter().filter(|f| ids.contains(&f.id) && f.active).map(|f| f.path.clone()).collect();
    platform::deactivate(&active_paths);
    let removed = trash_fonts(&mut lib, &ids, &data.paths.trash_dir()).map_err(|e| format!("Could not delete: {e}"))?;
    save(&data, &lib)?;
    Ok(DeleteResult { removed, library: lib.clone() })
}

/// Undo for `delete_fonts`: puts the files back and restores the library as it was.
#[tauri::command]
fn restore_fonts(data: State<AppData>, removed: Vec<FontRecord>, snapshot: Library) -> Result<Library, String> {
    restore_trashed(&removed, &data.paths.trash_dir()).map_err(|e| format!("Could not restore: {e}"))?;
    let mut lib = data.lib.lock().unwrap();
    *lib = snapshot;
    let paths: Vec<String> = removed.iter().filter(|f| f.active).map(|f| f.path.clone()).collect();
    platform::activate(&paths);
    save(&data, &lib)?;
    Ok(lib.clone())
}

fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

pub fn run() {
    let root = dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")).join("Buni Font Manager");
    let paths = Paths { root };
    // Deleted fonts can only be undone within one run of the app.
    let _ = std::fs::remove_dir_all(paths.trash_dir());
    let mut lib = store::load(&paths.library_file()).unwrap_or_default();
    // Re-read fonts once when the style detection has improved since they were imported.
    if refresh_detection(&mut lib) {
        let _ = store::save(&paths.library_file(), &lib);
    }

    // Activation lasts until Windows restarts, so bring back every active font now.
    let startup_paths: Vec<String> = lib.fonts.iter().filter(|f| f.active).map(|f| f.path.clone()).collect();
    std::thread::spawn(move || {
        platform::activate(&startup_paths);
    });

    let started_hidden = std::env::args().any(|a| a == "--hidden");

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| show_main(app)))
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, Some(vec!["--hidden"])))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(AppData { paths, lib: Mutex::new(lib) })
        .invoke_handler(tauri::generate_handler![get_state, import_paths, save_library, set_active, delete_fonts, restore_fonts])
        .setup(move |app| {
            let open = MenuItem::with_id(app, "open", "Open Buni Font Manager", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open, &quit])?;
            TrayIconBuilder::with_id("tray")
                .icon(app.default_window_icon().cloned().expect("app icon"))
                .tooltip("Buni Font Manager")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "open" => show_main(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                        show_main(tray.app_handle());
                    }
                })
                .build(app)?;
            if !started_hidden {
                show_main(app.handle());
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing the window keeps the app in the tray so fonts stay managed.
            if let WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running Buni Font Manager");
}
