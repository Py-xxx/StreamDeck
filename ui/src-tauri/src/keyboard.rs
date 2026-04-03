//! Keyboard simulation for executing keybinds.
//!
//! Parses action strings like "ctrl+alt+m" and simulates the key combination.

use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use std::thread;
use std::time::Duration;

/// Parse and execute a keybind action string.
///
/// # Format
/// Action strings use `+` as separator:
/// - `ctrl+c` → Ctrl+C
/// - `ctrl+shift+s` → Ctrl+Shift+S
/// - `alt+tab` → Alt+Tab
/// - `win+d` → Win+D
/// - `f1` → F1
/// - `play/pause media` → Media Play/Pause
///
/// # Examples
/// ```ignore
/// send_keys("ctrl+alt+m");  // Mute mic hotkey
/// send_keys("volume up");   // Volume up media key
/// ```
pub fn send_keys(action: &str) {
    let action = action.trim().to_lowercase();
    if action.is_empty() {
        return;
    }

    let mut enigo = match Enigo::new(&Settings::default()) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Failed to create Enigo: {:?}", e);
            return;
        }
    };
    
    let parts: Vec<&str> = action.split('+').map(|s| s.trim()).collect();

    // Collect modifiers and the main key
    let mut modifiers: Vec<Key> = Vec::new();
    let mut main_key: Option<Key> = None;

    for part in &parts {
        match *part {
            "ctrl" | "control" => modifiers.push(Key::Control),
            "alt" => modifiers.push(Key::Alt),
            "shift" => modifiers.push(Key::Shift),
            "win" | "meta" | "super" | "cmd" => modifiers.push(Key::Meta),
            _ => {
                // This is the main key
                if let Some(key) = parse_key(part) {
                    main_key = Some(key);
                }
            }
        }
    }

    // Handle special cases where the action is just a media key
    if main_key.is_none() && modifiers.is_empty() {
        if let Some(key) = parse_special_action(&action) {
            main_key = Some(key);
        }
    }

    // Execute the key combination
    if let Some(key) = main_key {
        // Press modifiers
        for m in &modifiers {
            let _ = enigo.key(*m, Direction::Press);
        }

        // Small delay for reliability
        thread::sleep(Duration::from_millis(10));

        // Press and release main key
        let _ = enigo.key(key, Direction::Click);

        // Small delay
        thread::sleep(Duration::from_millis(10));

        // Release modifiers (reverse order)
        for m in modifiers.iter().rev() {
            let _ = enigo.key(*m, Direction::Release);
        }
    }
}

/// Parse a key name into an enigo Key
fn parse_key(name: &str) -> Option<Key> {
    let name = name.trim().to_lowercase();
    
    // Single character
    if name.len() == 1 {
        let c = name.chars().next()?;
        return Some(Key::Unicode(c));
    }

    // Special keys
    match name.as_str() {
        // Function keys
        "f1" => Some(Key::F1),
        "f2" => Some(Key::F2),
        "f3" => Some(Key::F3),
        "f4" => Some(Key::F4),
        "f5" => Some(Key::F5),
        "f6" => Some(Key::F6),
        "f7" => Some(Key::F7),
        "f8" => Some(Key::F8),
        "f9" => Some(Key::F9),
        "f10" => Some(Key::F10),
        "f11" => Some(Key::F11),
        "f12" => Some(Key::F12),
        
        // Navigation
        "tab" => Some(Key::Tab),
        "enter" | "return" => Some(Key::Return),
        "escape" | "esc" => Some(Key::Escape),
        "space" | "spacebar" => Some(Key::Space),
        "backspace" => Some(Key::Backspace),
        "delete" | "del" => Some(Key::Delete),
        "home" => Some(Key::Home),
        "end" => Some(Key::End),
        "pageup" | "pgup" => Some(Key::PageUp),
        "pagedown" | "pgdn" => Some(Key::PageDown),
        
        // Arrow keys
        "up" | "uparrow" | "arrowup" => Some(Key::UpArrow),
        "down" | "downarrow" | "arrowdown" => Some(Key::DownArrow),
        "left" | "leftarrow" | "arrowleft" => Some(Key::LeftArrow),
        "right" | "rightarrow" | "arrowright" => Some(Key::RightArrow),
        
        // Media keys
        "volumeup" | "volup" => Some(Key::VolumeUp),
        "volumedown" | "voldown" => Some(Key::VolumeDown),
        "volumemute" | "mute" => Some(Key::VolumeMute),
        "medianexttrack" | "nexttrack" | "next" => Some(Key::MediaNextTrack),
        "mediaprevtrack" | "prevtrack" | "prev" | "previous" => Some(Key::MediaPrevTrack),
        "mediaplaypause" | "playpause" | "play" => Some(Key::MediaPlayPause),
        
        // Misc
        "capslock" | "caps" => Some(Key::CapsLock),
        
        _ => None,
    }
}

/// Parse special action phrases (for backwards compatibility)
fn parse_special_action(action: &str) -> Option<Key> {
    match action {
        "volume up" => Some(Key::VolumeUp),
        "volume down" => Some(Key::VolumeDown),
        "volume mute" | "mute" => Some(Key::VolumeMute),
        "play/pause media" | "play/pause" => Some(Key::MediaPlayPause),
        "next track" => Some(Key::MediaNextTrack),
        "previous track" | "prev track" => Some(Key::MediaPrevTrack),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_single_char() {
        assert!(matches!(parse_key("a"), Some(Key::Unicode('a'))));
        assert!(matches!(parse_key("1"), Some(Key::Unicode('1'))));
    }

    #[test]
    fn test_parse_function_keys() {
        assert!(matches!(parse_key("f1"), Some(Key::F1)));
        assert!(matches!(parse_key("F12"), Some(Key::F12)));
    }

    #[test]
    fn test_parse_special_keys() {
        assert!(matches!(parse_key("tab"), Some(Key::Tab)));
        assert!(matches!(parse_key("enter"), Some(Key::Return)));
        assert!(matches!(parse_key("escape"), Some(Key::Escape)));
    }

    #[test]
    fn test_parse_media_keys() {
        assert!(matches!(parse_key("volumeup"), Some(Key::VolumeUp)));
        assert!(matches!(parse_key("playpause"), Some(Key::MediaPlayPause)));
    }

    #[test]
    fn test_parse_special_action() {
        assert!(matches!(parse_special_action("volume up"), Some(Key::VolumeUp)));
        assert!(matches!(parse_special_action("play/pause media"), Some(Key::MediaPlayPause)));
    }
}
