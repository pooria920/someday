#[cfg(target_os = "windows")]
mod auto_images;
#[cfg(target_os = "windows")]
mod autostart;
mod backup;
mod capture;
mod database;
mod formatted_text;
mod images;
mod settings;
mod shortcuts;
#[cfg(target_os = "windows")]
mod windows_clipboard;
#[cfg(target_os = "windows")]
mod windows_selection;

use serde::Serialize;
use std::path::PathBuf;
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Manager,
};

struct DatabasePath(PathBuf);

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FoundationStatus {
    pending_count: u32,
    restore_report: backup::RestoreReport,
    theme: Option<String>,
    schema_version: u32,
    auto_save_images: bool,
    auto_save_images_available: bool,
    launch_at_login: Option<bool>,
    shortcut_available: bool,
    toggle_shortcut_available: bool,
    shortcuts: settings::ShortcutSettings,
    capture_result: Option<shortcuts::CaptureUpdate>,
}

#[tauri::command]
async fn get_foundation_status(
    app: tauri::AppHandle,
    path: tauri::State<'_, DatabasePath>,
    capture: tauri::State<'_, shortcuts::CaptureState>,
) -> Result<FoundationStatus, &'static str> {
    #[cfg(target_os = "windows")]
    let auto_save_images_available = auto_images::available(&app);
    #[cfg(not(target_os = "windows"))]
    let auto_save_images_available = false;
    let restore_report = app.state::<backup::Backups>().report.clone();
    let path = path.0.clone();
    let shortcut_available = capture
        .shortcut_available
        .load(std::sync::atomic::Ordering::Acquire);
    let toggle_shortcut_available = capture
        .toggle_shortcut_available
        .load(std::sync::atomic::Ordering::Acquire);
    let capture_result = capture
        .latest
        .lock()
        .map_err(|_| "storage_unavailable")?
        .clone();
    tauri::async_runtime::spawn_blocking(move || {
        let parent = path.parent().ok_or("storage_unavailable")?;
        std::fs::create_dir_all(parent).map_err(|_| "storage_unavailable")?;
        let connection = database::open(&path).map_err(|_| "storage_unavailable")?;
        let pending_count =
            database::pending_count(&connection).map_err(|_| "storage_unavailable")?;
        let shortcuts = settings::load(&connection).map_err(|_| "storage_unavailable")?;
        Ok(FoundationStatus {
            #[cfg(target_os = "windows")]
            launch_at_login: autostart::enabled().ok(),
            #[cfg(not(target_os = "windows"))]
            launch_at_login: None,
            pending_count,
            restore_report,
            schema_version: 7,
            theme: connection
                .query_row("SELECT theme FROM shortcut_settings WHERE id=1", [], |r| {
                    r.get(0)
                })
                .map_err(|_| "settings_unavailable")?,
            auto_save_images: connection
                .query_row(
                    "SELECT auto_save_images FROM shortcut_settings WHERE id = 1",
                    [],
                    |row| row.get(0),
                )
                .map_err(|_| "settings_unavailable")?,
            auto_save_images_available,
            shortcuts,
            shortcut_available,
            toggle_shortcut_available,
            capture_result,
        })
    })
    .await
    .map_err(|_| "storage_unavailable")?
}

#[tauri::command]
async fn create_backup(
    window: tauri::WebviewWindow,
    path: tauri::State<'_, DatabasePath>,
) -> Result<bool, &'static str> {
    #[cfg(target_os = "windows")]
    {
        let path = path.0.clone();
        tauri::async_runtime::spawn_blocking(move || {
            let app = window.app_handle();
            let state = app.state::<backup::Backups>();
            let _guard = state.prepared.lock().map_err(|_| "backup_write_failed")?;
            let selected = rfd::FileDialog::new()
                .set_parent(&window)
                .set_title("Create Someday backup")
                .set_file_name(format!(
                    "Someday-{}.someday-backup",
                    chrono::Local::now().format("%Y-%m-%d-%H%M%S")
                ))
                .add_filter("Someday backup", &["someday-backup"])
                .save_file();
            dismiss_panel_later(window.as_ref().window());
            let Some(destination) = selected else {
                return Ok(false);
            };
            backup::create(&path, &destination)?;
            Ok(true)
        })
        .await
        .map_err(|_| "backup_write_failed")?
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (window, path);
        Err("unsupported_platform")
    }
}

#[tauri::command]
async fn prepare_restore(
    window: tauri::WebviewWindow,
    path: tauri::State<'_, DatabasePath>,
) -> Result<Option<backup::PreparedRestore>, &'static str> {
    #[cfg(target_os = "windows")]
    {
        let path = path.0.clone();
        tauri::async_runtime::spawn_blocking(move || {
            let app = window.app_handle();
            let state = app.state::<backup::Backups>();
            let mut prepared = state.prepared.lock().map_err(|_| "backup_read_failed")?;
            let selected = rfd::FileDialog::new()
                .set_parent(&window)
                .set_title("Restore Someday backup")
                .add_filter("Someday backup", &["someday-backup"])
                .pick_file();
            dismiss_panel_later(window.as_ref().window());
            let Some(source) = selected else {
                return Ok(None);
            };
            backup::prepare(&path, &source, &mut prepared).map(Some)
        })
        .await
        .map_err(|_| "backup_read_failed")?
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (window, path);
        Err("unsupported_platform")
    }
}

#[tauri::command]
async fn restore_backup(
    app: tauri::AppHandle,
    path: tauri::State<'_, DatabasePath>,
    token: String,
) -> Result<(), &'static str> {
    let path = path.0.clone();
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let state = handle.state::<backup::Backups>();
        let mut prepared = state.prepared.lock().map_err(|_| "backup_write_failed")?;
        backup::schedule(&path, &token, &mut prepared)
    })
    .await
    .map_err(|_| "backup_write_failed")??;
    app.restart();
}

#[tauri::command]
async fn set_shortcuts(
    app: tauri::AppHandle,
    settings: settings::ShortcutSettings,
) -> Result<settings::ShortcutSettings, &'static str> {
    #[cfg(target_os = "windows")]
    {
        tauri::async_runtime::spawn_blocking(move || shortcuts::update(&app, settings))
            .await
            .map_err(|_| "settings_unavailable")?
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (app, settings);
        Err("unsupported_platform")
    }
}

#[tauri::command]
async fn set_auto_save_images(app: tauri::AppHandle, enabled: bool) -> Result<bool, &'static str> {
    #[cfg(target_os = "windows")]
    {
        tauri::async_runtime::spawn_blocking(move || auto_images::set(&app, enabled))
            .await
            .map_err(|_| "settings_unavailable")?
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (app, enabled);
        Err("unsupported_platform")
    }
}

#[tauri::command]
async fn set_launch_at_login(enabled: bool) -> Result<bool, &'static str> {
    #[cfg(target_os = "windows")]
    {
        tauri::async_runtime::spawn_blocking(move || autostart::set(enabled))
            .await
            .map_err(|_| "autostart_unavailable")?
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = enabled;
        Err("unsupported_platform")
    }
}

async fn storage<T: Send + 'static>(
    path: PathBuf,
    action: impl FnOnce(&mut rusqlite::Connection) -> rusqlite::Result<T> + Send + 'static,
) -> Result<T, &'static str> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut connection = database::open(&path).map_err(|_| "storage_unavailable")?;
        action(&mut connection).map_err(|error| match error {
            rusqlite::Error::QueryReturnedNoRows => "not_found",
            rusqlite::Error::InvalidQuery => "invalid_name",
            rusqlite::Error::SqliteFailure(ref code, _)
                if code.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE =>
            {
                "duplicate_name"
            }
            rusqlite::Error::SqliteFailure(ref code, _)
                if code.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_FOREIGNKEY =>
            {
                "not_found"
            }
            _ => "storage_unavailable",
        })
    })
    .await
    .map_err(|_| "storage_unavailable")?
}

#[tauri::command]
async fn list_items(
    path: tauri::State<'_, DatabasePath>,
    view: database::ItemView,
) -> Result<database::ItemSnapshot, &'static str> {
    storage(path.0.clone(), move |connection| {
        database::list_items(connection, view)
    })
    .await
}

#[tauri::command]
async fn set_theme(path: tauri::State<'_, DatabasePath>, light: bool) -> Result<(), &'static str> {
    storage(path.0.clone(), move |connection| {
        connection
            .execute(
                "UPDATE shortcut_settings SET theme=?1 WHERE id=1",
                [if light { "light" } else { "dark" }],
            )
            .map(|_| ())
    })
    .await
}

#[tauri::command]
async fn update_note(
    path: tauri::State<'_, DatabasePath>,
    id: String,
    content: String,
) -> Result<(), &'static str> {
    let path = path.0.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut connection = database::open(&path).map_err(|_| "storage_unavailable")?;
        database::update_note(&mut connection, &id, &content)
    })
    .await
    .map_err(|_| "storage_unavailable")?
}

#[tauri::command]
async fn export_image(
    window: tauri::WebviewWindow,
    path: tauri::State<'_, DatabasePath>,
    id: String,
) -> Result<bool, &'static str> {
    #[cfg(target_os = "windows")]
    {
        let path = path.0.clone();
        tauri::async_runtime::spawn_blocking(move || {
            let connection = database::open(&path).map_err(|_| "storage_unavailable")?;
            let (bytes, mime): (Vec<u8>, String) = connection
                .query_row(
                    "SELECT image_original,image_mime FROM items WHERE id=?1 AND type='image'",
                    [id],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .map_err(|_| "item_unavailable")?;
            let extension = match mime.as_str() {
                "image/png" => "png",
                "image/jpeg" => "jpg",
                "image/webp" => "webp",
                _ => return Err("invalid_image"),
            };
            let selected = rfd::FileDialog::new()
                .set_parent(&window)
                .set_title("Save image as")
                .set_file_name(format!("Someday.{}", extension))
                .add_filter("Original image", &[extension])
                .save_file();
            dismiss_panel_later(window.as_ref().window());
            let Some(destination) = selected else {
                return Ok(false);
            };
            images::export_original(&destination, &bytes)?;
            Ok(true)
        })
        .await
        .map_err(|_| "export_failed")?
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (window, path, id);
        Err("unsupported_platform")
    }
}

#[tauri::command]
async fn save_note(
    path: tauri::State<'_, DatabasePath>,
    content: String,
) -> Result<(capture::Outcome, u32), capture::Outcome> {
    let path = path.0.clone();
    tauri::async_runtime::spawn_blocking(move || capture::save(&path, &content))
        .await
        .map_err(|_| capture::Outcome::StorageUnavailable)?
}

#[cfg(target_os = "windows")]
fn save_image(
    path: &std::path::Path,
    image: images::PreparedImage,
) -> Result<(capture::Outcome, u32), &'static str> {
    let parent = path.parent().ok_or("storage_unavailable")?;
    std::fs::create_dir_all(parent).map_err(|_| "storage_unavailable")?;
    let mut connection = database::open(path).map_err(|_| "storage_unavailable")?;
    database::save_image(&mut connection, &image).map_err(|_| "storage_unavailable")
}

#[tauri::command]
async fn import_image(
    window: tauri::WebviewWindow,
    path: tauri::State<'_, DatabasePath>,
) -> Result<Option<(capture::Outcome, u32)>, &'static str> {
    #[cfg(target_os = "windows")]
    {
        let path = path.0.clone();
        tauri::async_runtime::spawn_blocking(move || {
            let result = images::import(&window);
            dismiss_panel_later(window.as_ref().window());
            result?.map(|image| save_image(&path, image)).transpose()
        })
        .await
        .map_err(|_| "storage_unavailable")?
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (window, path);
        Err("unsupported_platform")
    }
}

#[tauri::command]
async fn paste_image(
    window: tauri::WebviewWindow,
    path: tauri::State<'_, DatabasePath>,
) -> Result<(capture::Outcome, u32), &'static str> {
    #[cfg(target_os = "windows")]
    {
        let owner = window.hwnd().map_err(|_| "clipboard_unavailable")?.0 as isize;
        let path = path.0.clone();
        tauri::async_runtime::spawn_blocking(move || {
            let image = windows_clipboard::read_image(owner)?;
            save_image(&path, image)
        })
        .await
        .map_err(|_| "storage_unavailable")?
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (window, path);
        Err("unsupported_platform")
    }
}

#[tauri::command]
async fn load_item_image(
    path: tauri::State<'_, DatabasePath>,
    id: String,
    variant: database::ImageVariant,
) -> Result<tauri::ipc::Response, &'static str> {
    let bytes = storage(path.0.clone(), move |connection| {
        database::load_item_image(connection, &id, variant)
    })
    .await?;
    Ok(tauri::ipc::Response::new(bytes))
}

#[tauri::command]
async fn open_item(path: tauri::State<'_, DatabasePath>, id: String) -> Result<(), &'static str> {
    let url = storage(path.0.clone(), move |connection| {
        database::item_link(connection, &id)
    })
    .await?;
    tauri::async_runtime::spawn_blocking(move || {
        #[cfg(target_os = "windows")]
        {
            use windows::core::PCWSTR;
            use windows::Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL};
            let wide: Vec<u16> = url.encode_utf16().chain(Some(0)).collect();
            let result = unsafe {
                ShellExecuteW(
                    None,
                    windows::core::w!("open"),
                    PCWSTR(wide.as_ptr()),
                    None,
                    None,
                    SW_SHOWNORMAL,
                )
            };
            if result.0 as isize > 32 {
                Ok(())
            } else {
                Err("open_failed")
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = url;
            Err("unsupported_platform")
        }
    })
    .await
    .map_err(|_| "open_failed")?
}

#[tauri::command]
async fn copy_item(
    window: tauri::WebviewWindow,
    path: tauri::State<'_, DatabasePath>,
    id: String,
) -> Result<(), &'static str> {
    #[cfg(target_os = "windows")]
    {
        let owner = window.hwnd().map_err(|_| "clipboard_unavailable")?.0 as isize;
        let content = storage(path.0.clone(), move |connection| {
            database::item_for_copy(connection, &id)
        })
        .await?;
        tauri::async_runtime::spawn_blocking(move || match content {
            database::CopyContent::Text(text, html) => {
                windows_clipboard::write_formatted(owner, &text, html.as_deref())
            }
            database::CopyContent::Image(bytes) => windows_clipboard::write_image(owner, &bytes),
        })
        .await
        .map_err(|_| "clipboard_unavailable")?
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (window, path, id);
        Err("unsupported_platform")
    }
}

#[tauri::command]
async fn delete_item(path: tauri::State<'_, DatabasePath>, id: String) -> Result<(), &'static str> {
    storage(path.0.clone(), move |connection| {
        database::delete_item(connection, &id)
    })
    .await
}

#[tauri::command]
async fn set_item_archived(
    path: tauri::State<'_, DatabasePath>,
    id: String,
    archived: bool,
) -> Result<(), &'static str> {
    storage(path.0.clone(), move |connection| {
        database::set_item_archived(connection, &id, archived)
    })
    .await
}

#[tauri::command]
async fn set_item_category(
    path: tauri::State<'_, DatabasePath>,
    id: String,
    category_id: Option<String>,
) -> Result<(), &'static str> {
    storage(path.0.clone(), move |connection| {
        database::set_item_category(connection, &id, category_id.as_deref())
    })
    .await
}

#[tauri::command]
async fn create_category(
    path: tauri::State<'_, DatabasePath>,
    name: String,
) -> Result<database::Category, &'static str> {
    storage(path.0.clone(), move |connection| {
        database::create_category(connection, &name)
    })
    .await
}

#[tauri::command]
async fn rename_category(
    path: tauri::State<'_, DatabasePath>,
    id: String,
    name: String,
) -> Result<(), &'static str> {
    storage(path.0.clone(), move |connection| {
        database::rename_category(connection, &id, &name)
    })
    .await
}

#[tauri::command]
async fn delete_category(
    path: tauri::State<'_, DatabasePath>,
    id: String,
) -> Result<(), &'static str> {
    storage(path.0.clone(), move |connection| {
        database::delete_category(connection, &id)
    })
    .await
}

#[tauri::command]
fn drag_panel(app: tauri::AppHandle) -> Result<(), &'static str> {
    app.get_webview_window("main")
        .ok_or("panel_unavailable")?
        .start_dragging()
        .map_err(|_| "panel_unavailable")
}

#[tauri::command]
fn hide_panel(app: tauri::AppHandle) -> Result<(), &'static str> {
    app.get_webview_window("main")
        .ok_or("panel_unavailable")?
        .hide()
        .map_err(|_| "panel_unavailable")
}

fn panel_geometry(
    x: i32,
    y: i32,
    work_width: u32,
    work_height: u32,
    width: u32,
    height: u32,
    scale_ratio: f64,
) -> (i32, i32, u32, u32) {
    let width = ((width as f64 * scale_ratio).round() as u32).min(work_width);
    let height = ((height as f64 * scale_ratio).round() as u32).min(work_height);
    (
        x + ((work_width - width) / 2) as i32,
        y + ((work_height - height) / 2) as i32,
        width,
        height,
    )
}

fn show_panel(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        #[cfg(target_os = "windows")]
        let point = unsafe {
            use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowRect};
            let mut rect = windows::Win32::Foundation::RECT::default();
            GetWindowRect(GetForegroundWindow(), &mut rect)
                .ok()
                .map(|_| {
                    (
                        (rect.left as f64 + rect.right as f64) / 2.0,
                        (rect.top as f64 + rect.bottom as f64) / 2.0,
                    )
                })
        };
        #[cfg(not(target_os = "windows"))]
        let point: Option<(f64, f64)> = None;
        let monitor = point
            .and_then(|(x, y)| window.monitor_from_point(x, y).ok().flatten())
            .or_else(|| {
                window
                    .cursor_position()
                    .ok()
                    .and_then(|p| window.monitor_from_point(p.x, p.y).ok().flatten())
            });
        if let (Some(monitor), Ok(size), Ok(scale)) =
            (monitor, window.inner_size(), window.scale_factor())
        {
            let area = monitor.work_area();
            let (x, y, width, height) = panel_geometry(
                area.position.x,
                area.position.y,
                area.size.width,
                area.size.height,
                size.width,
                size.height,
                monitor.scale_factor() / scale,
            );
            let _ = window.set_position(tauri::PhysicalPosition::new(x, y));
            let _ = window.set_size(tauri::PhysicalSize::new(width, height));
        } else {
            let _ = window.center();
        }
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn dismiss_panel_later(window: tauri::Window) {
    if window.label() != "main" {
        return;
    }
    tauri::async_runtime::spawn(async move {
        // Let WebView/native-dialog focus transfers settle before dismissing the panel.
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        let app = window.app_handle().clone();
        let _ = app.run_on_main_thread(move || {
            if window.is_visible().ok() != Some(true) || window.is_focused().ok() != Some(false) {
                return;
            }
            #[cfg(target_os = "windows")]
            {
                use windows::Win32::UI::{
                    Input::KeyboardAndMouse::IsWindowEnabled,
                    WindowsAndMessaging::{GetAncestor, GetForegroundWindow, GA_ROOTOWNER},
                };
                let Ok(owner) = window.hwnd() else {
                    return;
                };
                unsafe {
                    if !IsWindowEnabled(owner).as_bool()
                        || GetAncestor(GetForegroundWindow(), GA_ROOTOWNER) == owner
                    {
                        return;
                    }
                }
            }
            let _ = window.hide();
        });
    });
}

#[cfg(target_os = "windows")]
fn toggle_panel(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        match window.is_visible() {
            Ok(true) => {
                let _ = window.hide();
            }
            Ok(false) => show_panel(app),
            Err(_) => {}
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, args, _| {
            if !args.iter().any(|arg| arg == "--autostart") {
                show_panel(app);
            }
        }))
        .setup(|app| {
            let database_path = app.path().app_data_dir()?.join("someday.db");
            let restore_path = database_path.clone();
            let restore_report = std::thread::spawn(move || backup::apply_pending(&restore_path))
                .join()
                .unwrap_or_else(|_| backup::RestoreReport {
                    message: "Restore could not finish. Check your data and the backups folder before retrying.".into(),
                    rescue_path: None,
                });
            let show_restore = !restore_report.message.is_empty();
            app.manage(backup::Backups {
                report: restore_report,
                ..Default::default()
            });
            app.manage(DatabasePath(database_path));
            app.manage(shortcuts::CaptureState::default());
            #[cfg(target_os = "windows")]
            app.manage(auto_images::AutoImages::default());
            #[cfg(target_os = "windows")]
            shortcuts::register(app.handle());
            #[cfg(target_os = "windows")]
            auto_images::start(app.handle());
            let open = MenuItem::with_id(app, "open", "Open Someday", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open, &quit])?;
            TrayIconBuilder::new()
                .icon(app.default_window_icon().ok_or("missing app icon")?.clone())
                .tooltip("Someday")
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open" => show_panel(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;
            if show_restore || !std::env::args_os().any(|arg| arg == "--autostart") {
                show_panel(app.handle());
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if matches!(event, tauri::WindowEvent::Focused(false)) {
                dismiss_panel_later(window.clone());
            }
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_foundation_status,
            create_backup,
            prepare_restore,
            restore_backup,
            set_shortcuts,
            set_auto_save_images,
            set_launch_at_login,
            list_items,
            save_note,
            update_note,
            set_theme,
            export_image,
            import_image,
            paste_image,
            load_item_image,
            copy_item,
            open_item,
            delete_item,
            set_item_archived,
            set_item_category,
            create_category,
            rename_category,
            delete_category,
            hide_panel,
            drag_panel
        ])
        .run(tauri::generate_context!())
        .expect("could not run Someday");
}

#[cfg(test)]
mod panel_tests {
    #[test]
    fn centers_on_negative_monitor_and_clamps_to_work_area_with_dpi() {
        assert_eq!(
            super::panel_geometry(-1920, 0, 1920, 1040, 420, 480, 1.0),
            (-1170, 280, 420, 480)
        );
        assert_eq!(
            super::panel_geometry(1920, 0, 1280, 720, 420, 480, 1.5),
            (2245, 0, 630, 720)
        );
        assert_eq!(
            super::panel_geometry(0, 40, 800, 560, 1200, 1000, 1.0),
            (0, 40, 800, 560)
        );
    }
}
