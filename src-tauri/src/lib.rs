mod platform;

use bfm_core::external::{find_conflicts, scan, ScanCache, ScanSource, ORIGIN_ADOBE, ORIGIN_SYSTEM};
use bfm_core::import::{import_paths as core_import, refresh_detection, restore_trashed, trash_fonts};
use bfm_core::{store, FontRecord, ImportReport, Library, DETECTION_VERSION};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, State, WindowEvent};

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
    fn settings_file(&self) -> PathBuf {
        self.root.join("settings.json")
    }
    fn external_cache(&self) -> PathBuf {
        self.root.join("external-cache.json")
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Settings {
    /// Folder where Adobe Fonts keeps synced fonts; None = the standard place.
    #[serde(default)]
    adobe_dir: Option<String>,
}

struct AppData {
    paths: Paths,
    lib: Mutex<Library>,
    settings: Mutex<Settings>,
    /// System and Adobe fonts; None while the first scan is running.
    external: Mutex<Option<Vec<FontRecord>>>,
    scan_lock: Mutex<()>,
}

/// The standard Adobe Fonts folder.
fn default_adobe_dir() -> Option<PathBuf> {
    if cfg!(windows) {
        std::env::var_os("APPDATA").map(|a| PathBuf::from(a).join("Adobe").join("CoreSync").join("plugins").join("livetype"))
    } else {
        dirs::home_dir().map(|h| h.join("Library/Application Support/Adobe/CoreSync/plugins/livetype"))
    }
}

fn adobe_dir(settings: &Settings) -> Option<PathBuf> {
    settings.adobe_dir.as_ref().map(PathBuf::from).or_else(default_adobe_dir)
}

/// Where the fonts the app doesn't own live on this computer.
fn scan_sources(settings: &Settings) -> Vec<ScanSource> {
    let mut v = Vec::new();
    let sys = |dir: PathBuf, recursive: bool| ScanSource { dir, origin: ORIGIN_SYSTEM.into(), recursive, any_extension: false };
    if cfg!(windows) {
        if let Some(w) = std::env::var_os("WINDIR") {
            v.push(sys(PathBuf::from(w).join("Fonts"), false));
        }
        if let Some(l) = std::env::var_os("LOCALAPPDATA") {
            v.push(sys(PathBuf::from(l).join("Microsoft").join("Windows").join("Fonts"), false));
        }
    } else {
        v.push(sys(PathBuf::from("/usr/share/fonts"), true));
        if let Some(h) = dirs::home_dir() {
            v.push(sys(h.join(".local/share/fonts"), true));
        }
    }
    if let Some(a) = adobe_dir(settings) {
        v.push(ScanSource { dir: a, origin: ORIGIN_ADOBE.into(), recursive: true, any_extension: true });
    }
    v
}

/// Reads system and Adobe fonts (fast after the first time, thanks to the cache),
/// lets the interface load their files, and tells it the list changed.
fn rescan(app: &AppHandle) -> Vec<FontRecord> {
    let data = app.state::<AppData>();
    let _guard = data.scan_lock.lock().unwrap();
    let settings = data.settings.lock().unwrap().clone();
    let sources = scan_sources(&settings);
    for s in &sources {
        if s.dir.exists() {
            let _ = app.asset_protocol_scope().allow_directory(&s.dir, true);
        }
    }
    let cache_file = data.paths.external_cache();
    let mut cache: ScanCache = std::fs::read(&cache_file).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
    let list = scan(&sources, &mut cache, DETECTION_VERSION);
    if let Ok(bytes) = serde_json::to_vec(&cache) {
        let _ = std::fs::write(&cache_file, bytes);
    }
    *data.external.lock().unwrap() = Some(list.clone());
    let _ = app.emit("external-fonts", list.len());
    list
}

#[tauri::command]
fn get_external(data: State<AppData>) -> Option<Vec<FontRecord>> {
    data.external.lock().unwrap().clone()
}

#[tauri::command]
async fn rescan_external(app: AppHandle) -> Result<Vec<FontRecord>, String> {
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || rescan(&handle)).await.map_err(|e| e.to_string())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SettingsDto {
    adobe_dir: String,
    adobe_dir_is_default: bool,
    adobe_dir_exists: bool,
}

fn settings_dto(settings: &Settings) -> SettingsDto {
    let dir = adobe_dir(settings);
    SettingsDto {
        adobe_dir: dir.as_ref().map(|d| d.to_string_lossy().to_string()).unwrap_or_default(),
        adobe_dir_is_default: settings.adobe_dir.is_none(),
        adobe_dir_exists: dir.map(|d| d.exists()).unwrap_or(false),
    }
}

#[tauri::command]
fn get_settings(data: State<AppData>) -> SettingsDto {
    settings_dto(&data.settings.lock().unwrap())
}

/// Changes (or with None, resets) the Adobe Fonts folder, then rescans.
#[tauri::command]
async fn set_adobe_dir(app: AppHandle, dir: Option<String>) -> Result<SettingsDto, String> {
    let data = app.state::<AppData>();
    let dto = {
        let mut s = data.settings.lock().unwrap();
        s.adobe_dir = dir.filter(|d| !d.trim().is_empty());
        let bytes = serde_json::to_vec_pretty(&*s).map_err(|e| e.to_string())?;
        std::fs::write(data.paths.settings_file(), bytes).map_err(|e| format!("Could not save settings: {e}"))?;
        settings_dto(&s)
    };
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || rescan(&handle)).await.map_err(|e| e.to_string())?;
    Ok(dto)
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
    let mut report = core_import(&mut lib, &inputs, &data.paths.library_dir(), &origin, now());
    if let Some(ext) = data.external.lock().unwrap().as_ref() {
        let added: Vec<FontRecord> = lib.fonts.iter().filter(|f| report.added.contains(&f.id)).cloned().collect();
        report.conflicts = find_conflicts(&added, ext);
    }
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
    let settings: Settings = std::fs::read(paths.settings_file()).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
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
        .manage(AppData { paths, lib: Mutex::new(lib), settings: Mutex::new(settings), external: Mutex::new(None), scan_lock: Mutex::new(()) })
        .invoke_handler(tauri::generate_handler![
            get_state,
            import_paths,
            save_library,
            set_active,
            delete_fonts,
            restore_fonts,
            get_external,
            rescan_external,
            get_settings,
            set_adobe_dir
        ])
        .setup(move |app| {
            // List system and Adobe fonts in the background; the interface updates when done.
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                rescan(&handle);
            });
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
