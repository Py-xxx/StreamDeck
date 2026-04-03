//! Configuration types and file I/O.
//!
//! Config is stored at `~/.streamdeck/config.json` and shared with the UI.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Button configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ButtonConfig {
    pub label: String,
    pub action: String,
}

/// Potentiometer configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PotConfig {
    pub label: String,
    pub strip: i8, // -1 = disabled, 0-4 = Voicemeeter strip
}

/// Profile with button and pot bindings
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Profile {
    pub buttons: HashMap<String, ButtonConfig>,
    pub pots: HashMap<String, PotConfig>,
}

/// Profile toggle button settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileToggle {
    pub button_id: i8, // -1 = disabled
    pub mode: String,  // "hold" or "tap"
    pub hold_ms: u32,
}

impl Default for ProfileToggle {
    fn default() -> Self {
        Self {
            button_id: -1,
            mode: "hold".into(),
            hold_ms: 500,
        }
    }
}

/// Display settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Display {
    pub grid_rows: u8,
    pub grid_cols: u8,
    pub num_pots: u8,
}

impl Default for Display {
    fn default() -> Self {
        Self {
            grid_rows: 3,
            grid_cols: 4,
            num_pots: 4,
        }
    }
}

/// Hardware pin configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hardware {
    pub row_pins: Vec<u8>,
    pub col_pins: Vec<u8>,
    pub pot_pins: Vec<u8>,
}

impl Default for Hardware {
    fn default() -> Self {
        Self {
            row_pins: vec![2, 3, 4],
            col_pins: vec![5, 6, 7, 8],
            pot_pins: vec![0, 1, 2, 3],
        }
    }
}

/// Main application config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub serial_port: String,
    pub active_profile: String,
    pub display: Display,
    pub hardware: Hardware,
    pub profile_toggle: ProfileToggle,
    pub profiles: HashMap<String, Profile>,
    
    // New settings for integrated app
    #[serde(default)]
    pub auto_connect: bool,
    #[serde(default)]
    pub launch_on_startup: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        let mut profiles = HashMap::new();
        
        // Default profile with some useful bindings
        let mut default_profile = Profile::default();
        
        // Default button bindings
        let default_buttons = [
            ("0", "Mute Mic", "ctrl+alt+m"),
            ("1", "Screenshot", "ctrl+shift+s"),
            ("2", "Alt+Tab", "alt+tab"),
            ("3", "Copy", "ctrl+c"),
            ("4", "Paste", "ctrl+v"),
            ("5", "Vol Up", "volumeup"),
            ("6", "Vol Down", "volumedown"),
            ("7", "Play/Pause", "playpause"),
            ("8", "Next", "medianexttrack"),
            ("9", "Prev", "mediaprevtrack"),
            ("10", "Desktop", "win+d"),
            ("11", "Explorer", "win+e"),
        ];
        
        for (id, label, action) in default_buttons {
            default_profile.buttons.insert(
                id.into(),
                ButtonConfig {
                    label: label.into(),
                    action: action.into(),
                },
            );
        }
        
        // Default pot bindings
        let default_pots = [
            ("0", "HW Input 1", 0),
            ("1", "HW Input 2", 1),
            ("2", "Virtual 1", 3),
            ("3", "Virtual 2", 4),
        ];
        
        for (id, label, strip) in default_pots {
            default_profile.pots.insert(
                id.into(),
                PotConfig {
                    label: label.into(),
                    strip,
                },
            );
        }
        
        profiles.insert("Default".into(), default_profile);
        
        Self {
            serial_port: "COM3".into(),
            active_profile: "Default".into(),
            display: Display::default(),
            hardware: Hardware::default(),
            profile_toggle: ProfileToggle::default(),
            profiles,
            auto_connect: true,
            launch_on_startup: false,
        }
    }
}

/// Get the config directory path
pub fn config_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".streamdeck")
}

/// Get the config file path
pub fn config_path() -> PathBuf {
    config_dir().join("config.json")
}

/// Load config from disk, creating default if not exists
pub fn load_config() -> Result<AppConfig, String> {
    let dir = config_dir();
    let path = config_path();
    
    // Ensure directory exists
    if !dir.exists() {
        fs::create_dir_all(&dir).map_err(|e| format!("Failed to create config dir: {}", e))?;
    }
    
    // Create default config if not exists
    if !path.exists() {
        let config = AppConfig::default();
        save_config(&config)?;
        return Ok(config);
    }
    
    // Read and parse config
    let content = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read config: {}", e))?;
    
    serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse config: {}", e))
}

/// Save config to disk
pub fn save_config(config: &AppConfig) -> Result<(), String> {
    let path = config_path();
    
    let content = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;
    
    // Write atomically via temp file
    let tmp_path = path.with_extension("tmp");
    fs::write(&tmp_path, &content)
        .map_err(|e| format!("Failed to write config: {}", e))?;
    fs::rename(&tmp_path, &path)
        .map_err(|e| format!("Failed to rename config: {}", e))?;
    
    Ok(())
}
