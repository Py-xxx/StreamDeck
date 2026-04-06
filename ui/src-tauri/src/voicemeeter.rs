//! Voicemeeter Banana integration via VoicemeeterRemote64.dll
//!
//! This module dynamically loads the Voicemeeter Remote API DLL and provides
//! functions to control strip gain levels.
//!
//! Only available on Windows.

#![cfg(windows)]

use libloading::Library;
use once_cell::sync::OnceCell;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::ffi::CString;
use std::path::PathBuf;

/// Voicemeeter API error codes
#[derive(Debug, Clone, PartialEq)]
pub enum VmError {
    NotInstalled,
    DllNotFound,
    LoginFailed(i32),
    NotLoggedIn,
    ParameterError(i32),
    Other(String),
}

impl std::fmt::Display for VmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VmError::NotInstalled => write!(f, "Voicemeeter is not installed"),
            VmError::DllNotFound => write!(f, "VoicemeeterRemote64.dll not found"),
            VmError::LoginFailed(code) => write!(f, "Voicemeeter login failed (code {})", code),
            VmError::NotLoggedIn => write!(f, "Not logged in to Voicemeeter"),
            VmError::ParameterError(code) => write!(f, "Parameter error (code {})", code),
            VmError::Other(msg) => write!(f, "{}", msg),
        }
    }
}

/// Type aliases for Voicemeeter API functions
type LoginFn = unsafe extern "C" fn() -> i32;
type LogoutFn = unsafe extern "C" fn() -> i32;
type SetParameterFloatFn = unsafe extern "C" fn(*const i8, f32) -> i32;
type GetParameterFloatFn = unsafe extern "C" fn(*const i8, *mut f32) -> i32;
type IsParametersDirtyFn = unsafe extern "C" fn() -> i32;

/// Global Voicemeeter instance
static VOICEMEETER: OnceCell<Mutex<Option<Voicemeeter>>> = OnceCell::new();

/// Cache of boolean parameter states (mute, A1, A2, etc.) that WE have set.
/// Avoids re-reading from VM — which races with continuous pot/gain SetParameterFloat
/// calls that cause spurious dirty flags and stale buffer reads.
static BOOL_STATE_CACHE: OnceCell<Mutex<HashMap<String, bool>>> = OnceCell::new();

fn bool_cache() -> &'static Mutex<HashMap<String, bool>> {
    BOOL_STATE_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Voicemeeter API wrapper
pub struct Voicemeeter {
    _library: Library,
    login: LoginFn,
    logout: LogoutFn,
    set_param_float: SetParameterFloatFn,
    get_param_float: GetParameterFloatFn,
    is_params_dirty: IsParametersDirtyFn,
    logged_in: bool,
}

impl Voicemeeter {
    /// Find the Voicemeeter installation directory from registry
    fn find_install_path() -> Option<PathBuf> {
        use winreg::enums::*;
        use winreg::RegKey;

        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        
        // Try 64-bit path first
        if let Ok(key) = hklm.open_subkey("SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\VB:Voicemeeter {17359A74-1236-5467}") {
            if let Ok(path) = key.get_value::<String, _>("UninstallString") {
                let path = PathBuf::from(path);
                if let Some(parent) = path.parent() {
                    return Some(parent.to_path_buf());
                }
            }
        }

        // Try Wow6432Node (32-bit on 64-bit Windows)
        if let Ok(key) = hklm.open_subkey("SOFTWARE\\WOW6432Node\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\VB:Voicemeeter {17359A74-1236-5467}") {
            if let Ok(path) = key.get_value::<String, _>("UninstallString") {
                let path = PathBuf::from(path);
                if let Some(parent) = path.parent() {
                    return Some(parent.to_path_buf());
                }
            }
        }

        // Fallback: common installation paths
        let program_files = std::env::var("ProgramFiles").unwrap_or_else(|_| "C:\\Program Files".into());
        let fallback = PathBuf::from(program_files).join("VB\\Voicemeeter");
        if fallback.exists() {
            return Some(fallback);
        }

        None
    }

    /// Load the Voicemeeter DLL and initialize
    pub fn new() -> Result<Self, VmError> {
        let install_path = Self::find_install_path().ok_or(VmError::NotInstalled)?;
        let dll_path = install_path.join("VoicemeeterRemote64.dll");

        if !dll_path.exists() {
            return Err(VmError::DllNotFound);
        }

        // SAFETY: We're loading a known DLL with expected function signatures
        unsafe {
            let library = Library::new(&dll_path)
                .map_err(|e| VmError::Other(format!("Failed to load DLL: {}", e)))?;

            // Get symbols and immediately dereference to raw function pointers
            // This avoids borrow issues since we copy the function pointers
            let login: LoginFn = *library
                .get::<LoginFn>(b"VBVMR_Login\0")
                .map_err(|e| VmError::Other(format!("Symbol not found: {}", e)))?;
            let logout: LogoutFn = *library
                .get::<LogoutFn>(b"VBVMR_Logout\0")
                .map_err(|e| VmError::Other(format!("Symbol not found: {}", e)))?;
            let set_param_float: SetParameterFloatFn = *library
                .get::<SetParameterFloatFn>(b"VBVMR_SetParameterFloat\0")
                .map_err(|e| VmError::Other(format!("Symbol not found: {}", e)))?;
            let get_param_float: GetParameterFloatFn = *library
                .get::<GetParameterFloatFn>(b"VBVMR_GetParameterFloat\0")
                .map_err(|e| VmError::Other(format!("Symbol not found: {}", e)))?;
            let is_params_dirty: IsParametersDirtyFn = *library
                .get::<IsParametersDirtyFn>(b"VBVMR_IsParametersDirty\0")
                .map_err(|e| VmError::Other(format!("Symbol not found: {}", e)))?;

            Ok(Self {
                _library: library,
                login,
                logout,
                set_param_float,
                get_param_float,
                is_params_dirty,
                logged_in: false,
            })
        }
    }

    /// Login to Voicemeeter
    pub fn login(&mut self) -> Result<(), VmError> {
        if self.logged_in {
            return Ok(());
        }

        // SAFETY: Calling FFI function with no arguments
        let result = unsafe { (self.login)() };
        
        // 0 = OK, 1 = OK but Voicemeeter not running (will start)
        if result == 0 || result == 1 {
            self.logged_in = true;
            // Give Voicemeeter time to start if it wasn't running
            if result == 1 {
                std::thread::sleep(std::time::Duration::from_secs(2));
            }
            Ok(())
        } else {
            Err(VmError::LoginFailed(result))
        }
    }

    /// Logout from Voicemeeter
    pub fn logout(&mut self) {
        if self.logged_in {
            // SAFETY: Calling FFI function with no arguments
            unsafe { (self.logout)() };
            self.logged_in = false;
        }
    }

    /// Set a strip's gain level
    /// 
    /// # Arguments
    /// * `strip` - Strip index (0-4 for Banana: HW1, HW2, HW3, Virtual1, Virtual2)
    /// * `gain_db` - Gain in dB (-60.0 to +12.0)
    pub fn set_strip_gain(&self, strip: u8, gain_db: f32) -> Result<(), VmError> {
        if !self.logged_in {
            return Err(VmError::NotLoggedIn);
        }

        let param = format!("Strip[{}].Gain", strip);
        let param_cstr = CString::new(param).unwrap();
        let gain = gain_db.clamp(-60.0, 12.0);

        // SAFETY: Calling FFI function with valid CString pointer
        let result = unsafe { (self.set_param_float)(param_cstr.as_ptr(), gain) };

        if result == 0 {
            Ok(())
        } else {
            Err(VmError::ParameterError(result))
        }
    }

    /// Get a parameter value
    fn get_parameter(&self, param_name: &str) -> Result<f32, VmError> {
        if !self.logged_in {
            return Err(VmError::NotLoggedIn);
        }

        // VBVMR_IsParametersDirty must be called to sync the internal parameter cache.
        // It only updates the buffer when it returns > 0. Poll with retries because
        // after a Set, VM processes the change asynchronously and reports dirty on the
        // next cycle — a single call that returns 0 would leave us with stale data.
        for _ in 0..5 {
            if unsafe { (self.is_params_dirty)() } > 0 {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }

        let param_cstr = CString::new(param_name).unwrap();
        let mut value: f32 = 0.0;

        let result = unsafe { (self.get_param_float)(param_cstr.as_ptr(), &mut value) };

        if result == 0 {
            Ok(value)
        } else {
            Err(VmError::ParameterError(result))
        }
    }

    /// Set a parameter value
    fn set_parameter(&self, param_name: &str, value: f32) -> Result<(), VmError> {
        if !self.logged_in {
            return Err(VmError::NotLoggedIn);
        }

        let param_cstr = CString::new(param_name).unwrap();
        let result = unsafe { (self.set_param_float)(param_cstr.as_ptr(), value) };

        if result == 0 {
            Ok(())
        } else {
            Err(VmError::ParameterError(result))
        }
    }

    /// Toggle strip mute
    pub fn toggle_strip_mute(&self, strip: u8) -> Result<(), VmError> {
        let param = format!("Strip[{}].Mute", strip);
        let current_value = self.get_parameter(&param)?;
        let new_value = if current_value > 0.5 { 0.0 } else { 1.0 };
        println!("Strip[{}].Mute: current={}, new={}", strip, current_value, new_value);
        self.set_parameter(&param, new_value)
    }

    /// Set strip mute to a specific value
    pub fn set_strip_mute(&self, strip: u8, muted: bool) -> Result<(), VmError> {
        let param = format!("Strip[{}].Mute", strip);
        let value = if muted { 1.0 } else { 0.0 };
        self.set_parameter(&param, value)
    }

    /// Get strip mute state
    pub fn get_strip_mute(&self, strip: u8) -> Result<bool, VmError> {
        let param = format!("Strip[{}].Mute", strip);
        let value = self.get_parameter(&param)?;
        Ok(value > 0.5)
    }

    /// Toggle strip solo
    pub fn toggle_strip_solo(&self, strip: u8) -> Result<(), VmError> {
        let param = format!("Strip[{}].Solo", strip);
        let current_value = self.get_parameter(&param)?;
        let new_value = if current_value > 0.5 { 0.0 } else { 1.0 };
        self.set_parameter(&param, new_value)
    }

    /// Set strip solo to a specific value
    pub fn set_strip_solo(&self, strip: u8, solo: bool) -> Result<(), VmError> {
        let param = format!("Strip[{}].Solo", strip);
        let value = if solo { 1.0 } else { 0.0 };
        self.set_parameter(&param, value)
    }

    /// Get strip solo state
    pub fn get_strip_solo(&self, strip: u8) -> Result<bool, VmError> {
        let param = format!("Strip[{}].Solo", strip);
        let value = self.get_parameter(&param)?;
        Ok(value > 0.5)
    }

    /// Toggle strip mono
    pub fn toggle_strip_mono(&self, strip: u8) -> Result<(), VmError> {
        let param = format!("Strip[{}].Mono", strip);
        let current_value = self.get_parameter(&param)?;
        let new_value = if current_value > 0.5 { 0.0 } else { 1.0 };
        self.set_parameter(&param, new_value)
    }

    /// Set strip mono to a specific value
    pub fn set_strip_mono(&self, strip: u8, mono: bool) -> Result<(), VmError> {
        let param = format!("Strip[{}].Mono", strip);
        let value = if mono { 1.0 } else { 0.0 };
        self.set_parameter(&param, value)
    }

    /// Get strip mono state
    pub fn get_strip_mono(&self, strip: u8) -> Result<bool, VmError> {
        let param = format!("Strip[{}].Mono", strip);
        let value = self.get_parameter(&param)?;
        Ok(value > 0.5)
    }

    /// Toggle strip bus routing (A1, A2, A3, A4, A5, B1, B2, B3)
    pub fn toggle_strip_bus(&self, strip: u8, bus: &str) -> Result<(), VmError> {
        let param = format!("Strip[{}].{}", strip, bus);
        let current_value = self.get_parameter(&param)?;
        let new_value = if current_value > 0.5 { 0.0 } else { 1.0 };
        self.set_parameter(&param, new_value)
    }

    /// Set strip bus routing to a specific value
    pub fn set_strip_bus(&self, strip: u8, bus: &str, enabled: bool) -> Result<(), VmError> {
        let param = format!("Strip[{}].{}", strip, bus);
        let value = if enabled { 1.0 } else { 0.0 };
        self.set_parameter(&param, value)
    }

    /// Get strip bus routing state
    pub fn get_strip_bus(&self, strip: u8, bus: &str) -> Result<bool, VmError> {
        let param = format!("Strip[{}].{}", strip, bus);
        let value = self.get_parameter(&param)?;
        Ok(value > 0.5)
    }
}

impl Drop for Voicemeeter {
    fn drop(&mut self) {
        self.logout();
    }
}

// Global API for convenience

/// Initialize the global Voicemeeter instance
pub fn init() -> Result<(), VmError> {
    let vm_mutex = VOICEMEETER.get_or_init(|| Mutex::new(None));
    let mut vm_opt = vm_mutex.lock();
    
    if vm_opt.is_some() {
        return Ok(());
    }

    let mut vm = Voicemeeter::new()?;
    vm.login()?;
    *vm_opt = Some(vm);
    Ok(())
}

/// Check if Voicemeeter is available and logged in
pub fn is_available() -> bool {
    if let Some(vm_mutex) = VOICEMEETER.get() {
        let vm_opt = vm_mutex.lock();
        if let Some(vm) = vm_opt.as_ref() {
            return vm.logged_in;
        }
    }
    false
}

/// Set strip gain (global convenience function)
pub fn set_strip_gain(strip: u8, gain_db: f32) -> Result<(), VmError> {
    let vm_mutex = VOICEMEETER.get().ok_or(VmError::NotLoggedIn)?;
    let vm_opt = vm_mutex.lock();
    let vm = vm_opt.as_ref().ok_or(VmError::NotLoggedIn)?;
    vm.set_strip_gain(strip, gain_db)
}

/// Shutdown the global Voicemeeter instance
pub fn shutdown() {
    if let Some(vm_mutex) = VOICEMEETER.get() {
        let mut vm_opt = vm_mutex.lock();
        *vm_opt = None;
    }
}

/// Convert raw ADC value (0-1023) to dB gain (-60 to +12)
/// The pot is inverted: 0 = max, 1023 = min
/// 
/// # Arguments
/// * `raw` - Raw ADC value (0-1023)
/// * `pot_ohms` - Potentiometer resistance in ohms (affects curve)
pub fn raw_to_gain_db_with_curve(raw: u16, pot_ohms: u32) -> f32 {
    let raw = raw.clamp(0, 1023) as f32;
    // Inverted: 1023 - raw gives us 0 at pot min, 1023 at pot max
    let normalized = (1023.0 - raw) / 1023.0;
    
    // Apply curve based on pot resistance
    // Higher ohm pots tend to have more logarithmic taper
    // We use a power curve: lower exponent = more linear, higher = more log-like
    let curve_factor = match pot_ohms {
        0..=2000 => 1.0,       // 1kΩ: linear
        2001..=7500 => 1.2,    // 5kΩ: slight curve
        7501..=25000 => 1.0,   // 10kΩ: linear (reference)
        25001..=75000 => 0.85, // 50kΩ: slight anti-log
        _ => 0.7,              // 100kΩ+: more anti-log
    };
    
    let curved = normalized.powf(curve_factor);
    
    // Map 0-1 to -60..+12 (72 dB range)
    curved * 72.0 - 60.0
}

/// Convert raw ADC value (0-1023) to dB gain (-60 to +12)
/// The pot is inverted: 0 = max, 1023 = min
/// Uses default 10kΩ linear mapping
pub fn raw_to_gain_db(raw: u16) -> f32 {
    raw_to_gain_db_with_curve(raw, 10000)
}

/// Convert raw ADC value to dB gain with calibration
/// 
/// # Arguments
/// * `raw` - Raw ADC value from Arduino
/// * `cal_min` - Raw value at pot's minimum position (typically highest ADC value due to inversion)
/// * `cal_max` - Raw value at pot's maximum position (typically lowest ADC value due to inversion)
pub fn raw_to_gain_db_calibrated(raw: u16, cal_min: u16, cal_max: u16) -> f32 {
    // Handle edge cases
    if cal_min == cal_max {
        return -60.0; // No range, return minimum
    }
    
    let raw = raw as f32;
    let cal_min = cal_min as f32;
    let cal_max = cal_max as f32;
    
    // Normalize to 0-1 range based on calibrated min/max
    // cal_min is the raw value when pot is at minimum (e.g., 1023)
    // cal_max is the raw value when pot is at maximum (e.g., 0)
    let normalized = if cal_min > cal_max {
        // Normal inverted pot: high raw = min volume, low raw = max volume
        ((cal_min - raw) / (cal_min - cal_max)).clamp(0.0, 1.0)
    } else {
        // Non-inverted pot: low raw = min volume, high raw = max volume
        ((raw - cal_min) / (cal_max - cal_min)).clamp(0.0, 1.0)
    };
    
    // Map 0-1 to -60..+12 (72 dB range)
    normalized * 72.0 - 60.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_raw_to_gain() {
        // Min position (inverted, so raw=1023 → -60 dB)
        assert!((raw_to_gain_db(1023) - (-60.0)).abs() < 0.1);
        
        // Max position (inverted, so raw=0 → +12 dB)
        assert!((raw_to_gain_db(0) - 12.0).abs() < 0.1);
        
        // Middle
        let mid = raw_to_gain_db(511);
        assert!(mid > -30.0 && mid < -20.0);
    }

    #[test]
    fn test_raw_to_gain_with_curve() {
        // 10kΩ should be linear (same as default)
        let linear = raw_to_gain_db_with_curve(512, 10000);
        let default = raw_to_gain_db(512);
        assert!((linear - default).abs() < 0.01);
        
        // Extremes should be the same regardless of curve
        assert!((raw_to_gain_db_with_curve(0, 1000) - 12.0).abs() < 0.1);
        assert!((raw_to_gain_db_with_curve(1023, 1000) - (-60.0)).abs() < 0.1);
        assert!((raw_to_gain_db_with_curve(0, 100000) - 12.0).abs() < 0.1);
        assert!((raw_to_gain_db_with_curve(1023, 100000) - (-60.0)).abs() < 0.1);
    }
}

/// Global convenience functions

/// Toggle strip mute
pub fn toggle_strip_mute(strip: u8) -> Result<(), VmError> {
    let vm_opt = VOICEMEETER.get_or_init(|| Mutex::new(None));
    let vm = vm_opt.lock();
    match vm.as_ref() {
        Some(voicemeeter) => voicemeeter.toggle_strip_mute(strip),
        None => Err(VmError::NotLoggedIn),
    }
}

/// Toggle strip solo
pub fn toggle_strip_solo(strip: u8) -> Result<(), VmError> {
    let vm_opt = VOICEMEETER.get_or_init(|| Mutex::new(None));
    let vm = vm_opt.lock();
    match vm.as_ref() {
        Some(voicemeeter) => voicemeeter.toggle_strip_solo(strip),
        None => Err(VmError::NotLoggedIn),
    }
}

/// Toggle strip mono
pub fn toggle_strip_mono(strip: u8) -> Result<(), VmError> {
    let vm_opt = VOICEMEETER.get_or_init(|| Mutex::new(None));
    let vm = vm_opt.lock();
    match vm.as_ref() {
        Some(voicemeeter) => voicemeeter.toggle_strip_mono(strip),
        None => Err(VmError::NotLoggedIn),
    }
}

/// Toggle strip bus routing (A1-A5, B1-B3)
pub fn toggle_strip_bus(strip: u8, bus: &str) -> Result<(), VmError> {
    let vm_opt = VOICEMEETER.get_or_init(|| Mutex::new(None));
    let vm = vm_opt.lock();
    match vm.as_ref() {
        Some(voicemeeter) => voicemeeter.toggle_strip_bus(strip, bus),
        None => Err(VmError::NotLoggedIn),
    }
}

/// Get strip mute state — returns cached value if known, otherwise reads from VM.
pub fn get_strip_mute(strip: u8) -> Result<bool, VmError> {
    let key = format!("Strip[{}].Mute", strip);
    if let Some(&cached) = bool_cache().lock().get(&key) {
        return Ok(cached);
    }
    let vm_opt = VOICEMEETER.get_or_init(|| Mutex::new(None));
    let vm = vm_opt.lock();
    match vm.as_ref() {
        Some(voicemeeter) => voicemeeter.get_strip_mute(strip),
        None => Err(VmError::NotLoggedIn),
    }
}

/// Set strip mute and update the local cache.
pub fn set_strip_mute(strip: u8, muted: bool) -> Result<(), VmError> {
    let vm_opt = VOICEMEETER.get_or_init(|| Mutex::new(None));
    let vm = vm_opt.lock();
    match vm.as_ref() {
        Some(voicemeeter) => {
            voicemeeter.set_strip_mute(strip, muted)?;
            bool_cache().lock().insert(format!("Strip[{}].Mute", strip), muted);
            Ok(())
        }
        None => Err(VmError::NotLoggedIn),
    }
}

/// Get strip solo state — returns cached value if known, otherwise reads from VM.
pub fn get_strip_solo(strip: u8) -> Result<bool, VmError> {
    let key = format!("Strip[{}].Solo", strip);
    if let Some(&cached) = bool_cache().lock().get(&key) {
        return Ok(cached);
    }
    let vm_opt = VOICEMEETER.get_or_init(|| Mutex::new(None));
    let vm = vm_opt.lock();
    match vm.as_ref() {
        Some(voicemeeter) => voicemeeter.get_strip_solo(strip),
        None => Err(VmError::NotLoggedIn),
    }
}

/// Set strip solo and update the local cache.
pub fn set_strip_solo(strip: u8, solo: bool) -> Result<(), VmError> {
    let vm_opt = VOICEMEETER.get_or_init(|| Mutex::new(None));
    let vm = vm_opt.lock();
    match vm.as_ref() {
        Some(voicemeeter) => {
            voicemeeter.set_strip_solo(strip, solo)?;
            bool_cache().lock().insert(format!("Strip[{}].Solo", strip), solo);
            Ok(())
        }
        None => Err(VmError::NotLoggedIn),
    }
}

/// Get strip mono state — returns cached value if known, otherwise reads from VM.
pub fn get_strip_mono(strip: u8) -> Result<bool, VmError> {
    let key = format!("Strip[{}].Mono", strip);
    if let Some(&cached) = bool_cache().lock().get(&key) {
        return Ok(cached);
    }
    let vm_opt = VOICEMEETER.get_or_init(|| Mutex::new(None));
    let vm = vm_opt.lock();
    match vm.as_ref() {
        Some(voicemeeter) => voicemeeter.get_strip_mono(strip),
        None => Err(VmError::NotLoggedIn),
    }
}

/// Set strip mono and update the local cache.
pub fn set_strip_mono(strip: u8, mono: bool) -> Result<(), VmError> {
    let vm_opt = VOICEMEETER.get_or_init(|| Mutex::new(None));
    let vm = vm_opt.lock();
    match vm.as_ref() {
        Some(voicemeeter) => {
            voicemeeter.set_strip_mono(strip, mono)?;
            bool_cache().lock().insert(format!("Strip[{}].Mono", strip), mono);
            Ok(())
        }
        None => Err(VmError::NotLoggedIn),
    }
}

/// Get strip bus state — returns cached value if known, otherwise reads from VM.
pub fn get_strip_bus(strip: u8, bus: &str) -> Result<bool, VmError> {
    let key = format!("Strip[{}].{}", strip, bus);
    if let Some(&cached) = bool_cache().lock().get(&key) {
        return Ok(cached);
    }
    let vm_opt = VOICEMEETER.get_or_init(|| Mutex::new(None));
    let vm = vm_opt.lock();
    match vm.as_ref() {
        Some(voicemeeter) => voicemeeter.get_strip_bus(strip, bus),
        None => Err(VmError::NotLoggedIn),
    }
}

/// Set strip bus and update the local cache.
pub fn set_strip_bus(strip: u8, bus: &str, enabled: bool) -> Result<(), VmError> {
    let vm_opt = VOICEMEETER.get_or_init(|| Mutex::new(None));
    let vm = vm_opt.lock();
    match vm.as_ref() {
        Some(voicemeeter) => {
            voicemeeter.set_strip_bus(strip, bus, enabled)?;
            bool_cache().lock().insert(format!("Strip[{}].{}", strip, bus), enabled);
            Ok(())
        }
        None => Err(VmError::NotLoggedIn),
    }
}
