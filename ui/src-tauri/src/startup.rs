//! Launch on startup functionality using Windows Registry.

use auto_launch::AutoLaunchBuilder;
use std::env;

/// Get the auto-launch manager
fn get_auto_launch() -> Result<auto_launch::AutoLaunch, String> {
    let exe_path = env::current_exe()
        .map_err(|e| format!("Failed to get exe path: {}", e))?;
    
    let exe_path_str = exe_path
        .to_str()
        .ok_or("Invalid exe path")?;
    
    AutoLaunchBuilder::new()
        .set_app_name("StreamDeck")
        .set_app_path(exe_path_str)
        .set_use_launch_agent(false) // Use registry on Windows
        .build()
        .map_err(|e| format!("Failed to create auto-launch: {}", e))
}

/// Check if launch on startup is enabled
pub fn is_enabled() -> bool {
    match get_auto_launch() {
        Ok(al) => al.is_enabled().unwrap_or(false),
        Err(_) => false,
    }
}

/// Enable launch on startup
pub fn enable() -> Result<(), String> {
    let al = get_auto_launch()?;
    al.enable().map_err(|e| format!("Failed to enable auto-launch: {}", e))
}

/// Disable launch on startup
pub fn disable() -> Result<(), String> {
    let al = get_auto_launch()?;
    al.disable().map_err(|e| format!("Failed to disable auto-launch: {}", e))
}

/// Set launch on startup state
pub fn set_enabled(enabled: bool) -> Result<(), String> {
    if enabled {
        enable()
    } else {
        disable()
    }
}
