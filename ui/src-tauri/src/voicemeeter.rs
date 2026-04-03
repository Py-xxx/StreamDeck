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

/// Voicemeeter API wrapper
pub struct Voicemeeter {
    _library: Library,
    login: LoginFn,
    logout: LogoutFn,
    set_param_float: SetParameterFloatFn,
    #[allow(dead_code)]
    get_param_float: GetParameterFloatFn,
    #[allow(dead_code)]
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
pub fn raw_to_gain_db(raw: u16) -> f32 {
    let raw = raw.clamp(0, 1023) as f32;
    // Inverted: 1023 - raw
    // Map 0-1023 to -60..+12 (72 dB range)
    ((1023.0 - raw) / 1023.0) * 72.0 - 60.0
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
}
