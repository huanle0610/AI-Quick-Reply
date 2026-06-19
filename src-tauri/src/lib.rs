use arboard::Clipboard;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf, thread, time::Duration};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager,
};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VK_CONTROL, VK_V,
};

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
    pub phrases: Vec<Phrase>,
}

pub fn default_config() -> AppConfig {
    AppConfig {
        hotkey: "Ctrl+Alt+Space".to_string(),
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
    fs::create_dir_all(&dir).map_err(|error| format!("Unable to create config directory: {error}"))?;
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

    let content = fs::read_to_string(&path).map_err(|error| format!("Unable to read config: {error}"))?;
    serde_json::from_str(&content).map_err(|error| format!("Invalid config JSON: {error}"))
}

#[tauri::command]
fn save_config(app: tauri::AppHandle, config: AppConfig) -> Result<(), String> {
    let path = config_path(&app)?;
    save_config_to_path(&path, &config)
}

fn save_config_to_path(path: &PathBuf, config: &AppConfig) -> Result<(), String> {
    let content = serde_json::to_string_pretty(config).map_err(|error| format!("Unable to serialize config: {error}"))?;
    fs::write(path, content).map_err(|error| format!("Unable to write config: {error}"))
}

#[tauri::command]
fn send_phrase(app: tauri::AppHandle, text: String) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window.hide().map_err(|error| format!("Unable to hide quick reply window: {error}"))?;
    }

    Clipboard::new()
        .map_err(|error| format!("Unable to open clipboard: {error}"))?
        .set_text(text)
        .map_err(|error| format!("Unable to write clipboard: {error}"))?;

    thread::sleep(Duration::from_millis(140));
    send_ctrl_v()?;
    Ok(())
}

fn toggle_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let visible = window.is_visible().unwrap_or(false);
        if visible {
            let _ = window.hide();
        } else {
            let _ = window.show();
            let _ = window.set_focus();
        }
    }
}

fn setup_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let show_hide = MenuItem::with_id(app, "show_hide", "Show/Hide", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show_hide, &quit])?;

    let mut builder = TrayIconBuilder::new()
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
    let sent = unsafe { SendInput(inputs.len() as u32, inputs.as_ptr(), std::mem::size_of::<INPUT>() as i32) };
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
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![load_config, save_config, send_phrase])
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
    fn config_round_trips_through_json() {
        let config = default_config();
        let json = serde_json::to_string_pretty(&config).unwrap();
        let parsed: AppConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed, config);
    }
}



