use arboard::Clipboard;
use serde::{Deserialize, Serialize};
use std::{
    ffi::OsStr,
    fs,
    os::windows::ffi::OsStrExt,
    path::PathBuf,
    thread,
    time::{Duration, Instant},
};
use tauri::{
    menu::{AboutMetadata, AboutMetadataBuilder, Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager, PhysicalPosition, PhysicalSize, WindowEvent,
};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
use windows_sys::Win32::{
    Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS, POINT},
    System::Registry::{
        RegCloseKey, RegCreateKeyW, RegDeleteValueW, RegSetValueExW, HKEY, HKEY_CURRENT_USER,
        REG_SZ,
    },
    UI::{
        Input::KeyboardAndMouse::{
            GetAsyncKeyState, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT,
            KEYEVENTF_KEYUP, VK_CONTROL, VK_V,
        },
        WindowsAndMessaging::{GetCursorPos, GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN},
    },
};

const COMPACT_WINDOW_SIZE: (i32, i32) = (320, 220);
const FULL_WINDOW_SIZE: (i32, i32) = (420, 620);
const POPUP_GAP: i32 = 12;
const SCREEN_MARGIN: i32 = 8;
const DEFAULT_CTRL_HOLD_SECONDS: u64 = 5;
const MIN_CTRL_HOLD_SECONDS: u64 = 1;
const MAX_CTRL_HOLD_SECONDS: u64 = 30;
const CTRL_POLL_INTERVAL: Duration = Duration::from_millis(50);
const AUTO_START_APP_NAME: &str = "AI Quick Reply";
const RUN_KEY_PATH: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Phrase {
    pub id: String,
    pub label: String,
    pub text: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub hotkey: String,
    #[serde(default = "default_ctrl_hold_seconds")]
    pub ctrl_hold_seconds: u64,
    #[serde(default = "default_auto_start_enabled")]
    pub auto_start_enabled: bool,
    pub phrases: Vec<Phrase>,
}

fn default_ctrl_hold_seconds() -> u64 {
    DEFAULT_CTRL_HOLD_SECONDS
}

fn default_auto_start_enabled() -> bool {
    true
}

pub fn default_config() -> AppConfig {
    AppConfig {
        hotkey: "Ctrl+Alt+Space".to_string(),
        ctrl_hold_seconds: default_ctrl_hold_seconds(),
        auto_start_enabled: default_auto_start_enabled(),
        phrases: vec![
            Phrase {
                id: "go-on".to_string(),
                label: "Go on".to_string(),
                text: "go on".to_string(),
                enabled: true,
            },
            Phrase {
                id: "commit".to_string(),
                label: "Commit".to_string(),
                text: "commit".to_string(),
                enabled: true,
            },
            Phrase {
                id: "yes".to_string(),
                label: "Yes".to_string(),
                text: "yes".to_string(),
                enabled: true,
            },
            Phrase {
                id: "run-tests".to_string(),
                label: "Run tests".to_string(),
                text: "run tests".to_string(),
                enabled: true,
            },
        ],
    }
}

fn config_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|error| format!("Unable to locate app config directory: {error}"))?;
    fs::create_dir_all(&dir)
        .map_err(|error| format!("Unable to create config directory: {error}"))?;
    Ok(dir.join("config.json"))
}

#[tauri::command]
fn load_config(app: tauri::AppHandle) -> Result<AppConfig, String> {
    let path = config_path(&app)?;
    if !path.exists() {
        let config = default_config();
        save_config_to_path(&path, &config)?;
        return Ok(config);
    }

    let content =
        fs::read_to_string(&path).map_err(|error| format!("Unable to read config: {error}"))?;
    serde_json::from_str(&content).map_err(|error| format!("Invalid config JSON: {error}"))
}

#[tauri::command]
fn save_config(app: tauri::AppHandle, config: AppConfig) -> Result<(), String> {
    let path = config_path(&app)?;
    save_config_to_path(&path, &config)?;
    sync_auto_start(&app, config.auto_start_enabled)
}

fn save_config_to_path(path: &PathBuf, config: &AppConfig) -> Result<(), String> {
    let content = serde_json::to_string_pretty(config)
        .map_err(|error| format!("Unable to serialize config: {error}"))?;
    fs::write(path, content).map_err(|error| format!("Unable to write config: {error}"))
}

#[tauri::command]
fn send_phrase(app: tauri::AppHandle, text: String) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window
            .hide()
            .map_err(|error| format!("Unable to hide quick reply window: {error}"))?;
    }

    Clipboard::new()
        .map_err(|error| format!("Unable to open clipboard: {error}"))?
        .set_text(text)
        .map_err(|error| format!("Unable to write clipboard: {error}"))?;

    thread::sleep(Duration::from_millis(140));
    send_ctrl_v()?;
    Ok(())
}

#[tauri::command]
fn set_window_mode(app: tauri::AppHandle, mode: String) -> Result<(), String> {
    match mode.as_str() {
        "compact" => {
            show_compact_window(&app);
            Ok(())
        }
        "full" => set_main_window_size(&app, FULL_WINDOW_SIZE),
        _ => Err(format!("Unknown window mode: {mode}")),
    }
}

fn toggle_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let visible = window.is_visible().unwrap_or(false);
        if visible {
            let _ = window.hide();
        } else {
            show_compact_window(app);
        }
    }
}

fn show_compact_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let cursor = cursor_position().unwrap_or_else(|| {
            let screen = screen_size();
            (screen.0 / 2, screen.1 / 2)
        });
        let screen = screen_size();
        let (x, y) = popup_position_above_cursor(cursor, COMPACT_WINDOW_SIZE, screen);

        let _ = window.set_size(PhysicalSize::new(
            COMPACT_WINDOW_SIZE.0 as u32,
            COMPACT_WINDOW_SIZE.1 as u32,
        ));
        let _ = window.set_position(PhysicalPosition::new(x, y));
        let _ = window.emit("view-mode", "compact");
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn set_main_window_size(app: &tauri::AppHandle, size: (i32, i32)) -> Result<(), String> {
    let Some(window) = app.get_webview_window("main") else {
        return Err("Main window is not available".to_string());
    };

    window
        .set_size(PhysicalSize::new(size.0 as u32, size.1 as u32))
        .map_err(|error| format!("Unable to resize window: {error}"))?;
    window
        .show()
        .map_err(|error| format!("Unable to show window: {error}"))?;
    window
        .set_focus()
        .map_err(|error| format!("Unable to focus window: {error}"))
}

fn popup_position_above_cursor(
    cursor: (i32, i32),
    window_size: (i32, i32),
    screen_size: (i32, i32),
) -> (i32, i32) {
    let max_x = (screen_size.0 - window_size.0 - SCREEN_MARGIN).max(SCREEN_MARGIN);
    let max_y = (screen_size.1 - window_size.1 - SCREEN_MARGIN).max(SCREEN_MARGIN);

    let x = (cursor.0 - window_size.0 / 2).clamp(SCREEN_MARGIN, max_x);
    let above_y = cursor.1 - window_size.1 - POPUP_GAP;
    let y = if above_y >= SCREEN_MARGIN {
        above_y
    } else {
        cursor.1 + POPUP_GAP
    }
    .clamp(SCREEN_MARGIN, max_y);

    (x, y)
}

fn cursor_position() -> Option<(i32, i32)> {
    let mut point = POINT { x: 0, y: 0 };
    let ok = unsafe { GetCursorPos(&mut point) } != 0;
    ok.then_some((point.x, point.y))
}

fn screen_size() -> (i32, i32) {
    let width = unsafe { GetSystemMetrics(SM_CXSCREEN) };
    let height = unsafe { GetSystemMetrics(SM_CYSCREEN) };
    (
        width.max(COMPACT_WINDOW_SIZE.0),
        height.max(COMPACT_WINDOW_SIZE.1),
    )
}

fn start_ctrl_hold_listener(app: tauri::AppHandle) {
    thread::spawn(move || {
        let mut pressed_since: Option<Instant> = None;
        let mut fired_for_current_press = false;

        loop {
            if ctrl_is_down() {
                let since = pressed_since.get_or_insert_with(Instant::now);
                if !fired_for_current_press
                    && since.elapsed() >= configured_ctrl_hold_duration(&app)
                {
                    show_compact_window(&app);
                    fired_for_current_press = true;
                }
            } else {
                pressed_since = None;
                fired_for_current_press = false;
            }

            thread::sleep(CTRL_POLL_INTERVAL);
        }
    });
}

fn configured_ctrl_hold_duration(app: &tauri::AppHandle) -> Duration {
    let seconds = load_config(app.clone())
        .map(|config| config.ctrl_hold_seconds)
        .unwrap_or(DEFAULT_CTRL_HOLD_SECONDS);
    ctrl_hold_duration_from_seconds(seconds)
}

fn ctrl_hold_duration_from_seconds(seconds: u64) -> Duration {
    Duration::from_secs(seconds.clamp(MIN_CTRL_HOLD_SECONDS, MAX_CTRL_HOLD_SECONDS))
}

fn sync_auto_start(_app: &tauri::AppHandle, enabled: bool) -> Result<(), String> {
    let exe = std::env::current_exe()
        .map_err(|error| format!("Unable to locate current exe: {error}"))?;
    set_auto_start_registry_value(enabled, &exe.to_string_lossy())
}

fn set_auto_start_registry_value(enabled: bool, exe_path: &str) -> Result<(), String> {
    let key_path = wide_null(RUN_KEY_PATH);
    let value_name = wide_null(AUTO_START_APP_NAME);
    let mut key: HKEY = std::ptr::null_mut();
    let create_result = unsafe { RegCreateKeyW(HKEY_CURRENT_USER, key_path.as_ptr(), &mut key) };

    if create_result != ERROR_SUCCESS {
        return Err(format!(
            "Unable to open auto-start registry key: {create_result}"
        ));
    }

    let result = if enabled {
        let value = wide_null(&auto_start_run_value(exe_path));
        unsafe {
            RegSetValueExW(
                key,
                value_name.as_ptr(),
                0,
                REG_SZ,
                value.as_ptr() as *const u8,
                (value.len() * std::mem::size_of::<u16>()) as u32,
            )
        }
    } else {
        unsafe { RegDeleteValueW(key, value_name.as_ptr()) }
    };

    unsafe {
        RegCloseKey(key);
    }

    if result == ERROR_SUCCESS || (!enabled && result == ERROR_FILE_NOT_FOUND) {
        Ok(())
    } else {
        Err(format!(
            "Unable to update auto-start registry value: {result}"
        ))
    }
}

fn auto_start_run_value(exe_path: &str) -> String {
    format!("\"{exe_path}\"")
}

fn wide_null(value: &str) -> Vec<u16> {
    OsStr::new(value)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

fn ctrl_is_down() -> bool {
    unsafe { GetAsyncKeyState(VK_CONTROL as i32) < 0 }
}

fn tray_tooltip_text() -> &'static str {
    AUTO_START_APP_NAME
}

fn tray_about_menu_text() -> &'static str {
    "About"
}

fn tray_about_metadata() -> AboutMetadata<'static> {
    AboutMetadataBuilder::new()
        .name(Some(AUTO_START_APP_NAME))
        .version(Some(env!("CARGO_PKG_VERSION")))
        .build()
}

fn setup_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let show_hide = MenuItem::with_id(app, "show_hide", "Show/Hide", true, None::<&str>)?;
    let about = PredefinedMenuItem::about(
        app,
        Some(tray_about_menu_text()),
        Some(tray_about_metadata()),
    )?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show_hide, &about, &quit])?;

    let mut builder = TrayIconBuilder::new()
        .tooltip(tray_tooltip_text())
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                toggle_window(tray.app_handle());
            }
        })
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show_hide" => toggle_window(app),
            "quit" => app.exit(0),
            _ => {}
        });

    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }

    builder.build(app)?;
    Ok(())
}

fn register_hotkey(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let shortcut = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::Space);
    app.handle().global_shortcut().register(shortcut)?;
    Ok(())
}

fn send_ctrl_v() -> Result<(), String> {
    let inputs = [
        keyboard_input(VK_CONTROL as u16, 0),
        keyboard_input(VK_V as u16, 0),
        keyboard_input(VK_V as u16, KEYEVENTF_KEYUP),
        keyboard_input(VK_CONTROL as u16, KEYEVENTF_KEYUP),
    ];
    send_inputs(&inputs)
}

fn keyboard_input(vk: u16, flags: u32) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

fn send_inputs(inputs: &[INPUT]) -> Result<(), String> {
    let sent = unsafe {
        SendInput(
            inputs.len() as u32,
            inputs.as_ptr(),
            std::mem::size_of::<INPUT>() as i32,
        )
    };
    if sent == inputs.len() as u32 {
        Ok(())
    } else {
        Err(format!("SendInput sent {sent} of {} events", inputs.len()))
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        toggle_window(app);
                    }
                })
                .build(),
        )
        .setup(|app| {
            setup_tray(app)?;
            register_hotkey(app)?;
            if let Ok(config) = load_config(app.handle().clone()) {
                let _ = sync_auto_start(app.handle(), config.auto_start_enabled);
            }
            start_ctrl_hold_listener(app.handle().clone());
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            load_config,
            save_config,
            send_phrase,
            set_window_mode
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_contains_safe_starter_phrases() {
        let config = default_config();

        assert_eq!(config.hotkey, "Ctrl+Alt+Space");
        assert!(config.phrases.iter().any(|phrase| phrase.id == "go-on"));
        assert!(config.phrases.iter().any(|phrase| phrase.id == "commit"));
    }

    #[test]
    fn auto_start_run_value_quotes_the_exe_path() {
        assert_eq!(
            auto_start_run_value(r"C:\\Program Files\\AI Quick Reply\\ai-quick-reply.exe"),
            r#""C:\\Program Files\\AI Quick Reply\\ai-quick-reply.exe""#
        );
    }

    #[test]
    fn ctrl_hold_duration_clamps_to_a_usable_range() {
        assert_eq!(ctrl_hold_duration_from_seconds(0), Duration::from_secs(1));
        assert_eq!(ctrl_hold_duration_from_seconds(7), Duration::from_secs(7));
        assert_eq!(ctrl_hold_duration_from_seconds(99), Duration::from_secs(30));
    }

    #[test]
    fn tray_tooltip_uses_the_app_name() {
        assert_eq!(tray_tooltip_text(), "AI Quick Reply");
    }

    #[test]
    fn tray_about_menu_uses_clear_label() {
        assert_eq!(tray_about_menu_text(), "About");
    }

    #[test]
    fn tray_about_metadata_uses_app_identity() {
        let metadata = tray_about_metadata();

        assert_eq!(metadata.name.as_deref(), Some("AI Quick Reply"));
        assert_eq!(metadata.version.as_deref(), Some(env!("CARGO_PKG_VERSION")));
    }

    #[test]
    fn config_round_trips_through_json() {
        let config = default_config();
        let json = serde_json::to_string_pretty(&config).unwrap();
        let parsed: AppConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed, config);
    }

    #[test]
    fn popup_position_appears_above_cursor_without_leaving_screen() {
        let position = popup_position_above_cursor((500, 420), (320, 220), (1920, 1080));

        assert_eq!(position, (340, 188));
    }

    #[test]
    fn popup_position_clamps_near_screen_edges() {
        let top_left = popup_position_above_cursor((20, 30), (320, 220), (1920, 1080));
        let bottom_right = popup_position_above_cursor((1900, 1060), (320, 220), (1920, 1080));

        assert_eq!(top_left, (8, 42));
        assert_eq!(bottom_right, (1592, 828));
    }
}
