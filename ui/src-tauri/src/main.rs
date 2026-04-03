#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod config;
mod daemon;
mod keyboard;
mod serial;
mod startup;
#[cfg(windows)]
mod voicemeeter;

use config::AppConfig;
use daemon::Daemon;
use once_cell::sync::OnceCell;
use parking_lot::Mutex;
use serial::{ConnectionState, PortInfo};
use std::sync::Arc;

/// Global daemon instance
static DAEMON: OnceCell<Arc<Mutex<Daemon>>> = OnceCell::new();

fn get_daemon() -> Arc<Mutex<Daemon>> {
    Arc::clone(DAEMON.get_or_init(|| Arc::new(Mutex::new(Daemon::new()))))
}

// ============================================================================
// Tauri Commands - Serial
// ============================================================================

#[tauri::command]
fn list_serial_ports() -> Vec<PortInfo> {
    serial::SerialManager::list_ports()
}

#[tauri::command]
fn connect_serial(port: String) -> Result<(), String> {
    let daemon = get_daemon();
    let daemon = daemon.lock();
    daemon.connect(&port)
}

#[tauri::command]
fn disconnect_serial() {
    let daemon = get_daemon();
    let daemon = daemon.lock();
    daemon.disconnect();
}

#[tauri::command]
fn get_connection_status() -> ConnectionStatus {
    let daemon = get_daemon();
    let daemon = daemon.lock();
    
    match daemon.connection_state() {
        ConnectionState::Disconnected => ConnectionStatus {
            connected: false,
            port: None,
            error: None,
        },
        ConnectionState::Connecting => ConnectionStatus {
            connected: false,
            port: None,
            error: Some("Connecting...".into()),
        },
        ConnectionState::Connected(port) => ConnectionStatus {
            connected: true,
            port: Some(port),
            error: None,
        },
        ConnectionState::Error(msg) => ConnectionStatus {
            connected: false,
            port: None,
            error: Some(msg),
        },
    }
}

#[derive(serde::Serialize)]
struct ConnectionStatus {
    connected: bool,
    port: Option<String>,
    error: Option<String>,
}

// ============================================================================
// Tauri Commands - Voicemeeter
// ============================================================================

#[tauri::command]
fn init_voicemeeter() -> Result<bool, String> {
    let daemon = get_daemon();
    let daemon = daemon.lock();
    Ok(daemon.init_voicemeeter())
}

#[tauri::command]
fn is_voicemeeter_available() -> bool {
    let daemon = get_daemon();
    let daemon = daemon.lock();
    daemon.is_voicemeeter_available()
}

// ============================================================================
// Tauri Commands - Config
// ============================================================================

#[tauri::command]
fn notify_config_updated(config: AppConfig) {
    let daemon = get_daemon();
    let daemon = daemon.lock();
    daemon.update_config(config);
}

#[tauri::command]
fn reload_daemon_config() {
    let daemon = get_daemon();
    let daemon = daemon.lock();
    daemon.reload_config();
}

// ============================================================================
// Tauri Commands - Startup
// ============================================================================

#[tauri::command]
fn get_launch_on_startup() -> bool {
    startup::is_enabled()
}

#[tauri::command]
fn set_launch_on_startup(enabled: bool) -> Result<(), String> {
    startup::set_enabled(enabled)?;
    
    // Also update config
    if let Ok(mut config) = config::load_config() {
        config.launch_on_startup = enabled;
        config::save_config(&config)?;
    }
    
    Ok(())
}

// ============================================================================
// App Initialization
// ============================================================================

fn main() {
    // Initialize daemon
    let daemon = get_daemon();
    
    // Try to init Voicemeeter
    {
        let daemon = daemon.lock();
        if daemon.init_voicemeeter() {
            println!("Voicemeeter initialized successfully");
        } else {
            println!("Voicemeeter not available (app will still work for keyboard shortcuts)");
        }
    }
    
    // Auto-connect if configured
    {
        let daemon = daemon.lock();
        if let Ok(config) = config::load_config() {
            if config.auto_connect && !config.serial_port.is_empty() {
                println!("Auto-connecting to {}...", config.serial_port);
                if let Err(e) = daemon.connect(&config.serial_port) {
                    eprintln!("Auto-connect failed: {}", e);
                }
            }
        }
    }

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            // Serial
            list_serial_ports,
            connect_serial,
            disconnect_serial,
            get_connection_status,
            // Voicemeeter
            init_voicemeeter,
            is_voicemeeter_available,
            // Config
            notify_config_updated,
            reload_daemon_config,
            // Startup
            get_launch_on_startup,
            set_launch_on_startup,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
