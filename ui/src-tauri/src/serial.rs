//! Serial port communication with Arduino.
//!
//! Protocol: Lines of the form `<TYPE><ID>:<VALUE>\n`
//! - `P0:512` → Potentiometer 0, raw ADC value 512
//! - `B3:1`   → Button 3 pressed
//! - `B3:0`   → Button 3 released

use parking_lot::Mutex;
use serialport::{SerialPort, SerialPortInfo, SerialPortType};
use std::io::{BufRead, BufReader};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

/// Message types from Arduino
#[derive(Debug, Clone)]
pub enum ArduinoMessage {
    Pot { id: u8, value: u16 },
    Button { id: u8, pressed: bool },
}

/// Connection state
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected(String), // port name
    Error(String),
}

/// Callback type for incoming messages
pub type MessageCallback = Box<dyn Fn(ArduinoMessage) + Send + 'static>;

/// Serial connection manager
pub struct SerialManager {
    state: Arc<Mutex<ConnectionState>>,
    running: Arc<AtomicBool>,
    thread_handle: Mutex<Option<JoinHandle<()>>>,
    callback: Arc<Mutex<Option<MessageCallback>>>,
}

impl SerialManager {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(ConnectionState::Disconnected)),
            running: Arc::new(AtomicBool::new(false)),
            thread_handle: Mutex::new(None),
            callback: Arc::new(Mutex::new(None)),
        }
    }

    /// Set callback for incoming messages
    pub fn set_callback<F>(&self, callback: F)
    where
        F: Fn(ArduinoMessage) + Send + 'static,
    {
        *self.callback.lock() = Some(Box::new(callback));
    }

    /// Get current connection state
    pub fn state(&self) -> ConnectionState {
        self.state.lock().clone()
    }

    /// List available COM ports with Arduino-like devices highlighted
    pub fn list_ports() -> Vec<PortInfo> {
        serialport::available_ports()
            .unwrap_or_default()
            .into_iter()
            .map(|p| {
                let is_arduino = is_arduino_port(&p);
                let description = port_description(&p);
                PortInfo {
                    name: p.port_name,
                    description,
                    is_arduino,
                }
            })
            .collect()
    }

    /// Connect to a serial port
    pub fn connect(&self, port_name: &str) -> Result<(), String> {
        // Stop any existing connection
        self.disconnect();

        *self.state.lock() = ConnectionState::Connecting;

        let port = serialport::new(port_name, 115_200)
            .timeout(Duration::from_millis(100))
            .open()
            .map_err(|e| {
                let msg = format!("Failed to open {}: {}", port_name, e);
                *self.state.lock() = ConnectionState::Error(msg.clone());
                msg
            })?;

        // Wait for Arduino to reset after connection
        thread::sleep(Duration::from_secs(2));

        let port_name_owned = port_name.to_string();
        *self.state.lock() = ConnectionState::Connected(port_name_owned.clone());
        self.running.store(true, Ordering::SeqCst);

        // Spawn reader thread
        let state = Arc::clone(&self.state);
        let running = Arc::clone(&self.running);
        let callback = Arc::clone(&self.callback);

        let handle = thread::spawn(move || {
            Self::reader_loop(port, state, running, callback, port_name_owned);
        });

        *self.thread_handle.lock() = Some(handle);
        Ok(())
    }

    /// Disconnect from serial port
    pub fn disconnect(&self) {
        self.running.store(false, Ordering::SeqCst);

        if let Some(handle) = self.thread_handle.lock().take() {
            let _ = handle.join();
        }

        *self.state.lock() = ConnectionState::Disconnected;
    }

    /// Reader loop that runs in a background thread
    fn reader_loop(
        port: Box<dyn SerialPort>,
        state: Arc<Mutex<ConnectionState>>,
        running: Arc<AtomicBool>,
        callback: Arc<Mutex<Option<MessageCallback>>>,
        port_name: String,
    ) {
        let mut reader = BufReader::new(port);
        let mut line = String::new();

        while running.load(Ordering::SeqCst) {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) => {
                    // EOF - port disconnected
                    *state.lock() = ConnectionState::Error("Port disconnected".into());
                    break;
                }
                Ok(_) => {
                    if let Some(msg) = parse_message(&line) {
                        if let Some(ref cb) = *callback.lock() {
                            cb(msg);
                        }
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {
                    // Timeout is normal, continue
                    continue;
                }
                Err(e) => {
                    *state.lock() = ConnectionState::Error(format!("Read error: {}", e));
                    break;
                }
            }
        }

        // Update state if we exited due to error
        let current = state.lock().clone();
        if running.load(Ordering::SeqCst) {
            // We didn't disconnect intentionally
            if matches!(current, ConnectionState::Connected(_)) {
                *state.lock() = ConnectionState::Error(format!("Lost connection to {}", port_name));
            }
        }
    }
}

impl Drop for SerialManager {
    fn drop(&mut self) {
        self.disconnect();
    }
}

/// Port information for UI
#[derive(Debug, Clone, serde::Serialize)]
pub struct PortInfo {
    pub name: String,
    pub description: String,
    pub is_arduino: bool,
}

/// Parse a line from Arduino into a message
fn parse_message(line: &str) -> Option<ArduinoMessage> {
    let line = line.trim();
    if line.len() < 3 {
        return None;
    }

    let kind = line.chars().next()?;
    let colon_pos = line.find(':')?;
    let id: u8 = line[1..colon_pos].parse().ok()?;
    let value: i32 = line[colon_pos + 1..].parse().ok()?;

    match kind {
        'P' => Some(ArduinoMessage::Pot {
            id,
            value: value.clamp(0, 1023) as u16,
        }),
        'B' => Some(ArduinoMessage::Button {
            id,
            pressed: value == 1,
        }),
        _ => None,
    }
}

/// Check if a port looks like an Arduino
fn is_arduino_port(port: &SerialPortInfo) -> bool {
    let keywords = ["arduino", "ch340", "cp210", "ftdi", "usb serial"];

    if let SerialPortType::UsbPort(usb) = &port.port_type {
        let desc = usb.product.as_deref().unwrap_or("").to_lowercase();
        let mfr = usb.manufacturer.as_deref().unwrap_or("").to_lowercase();
        return keywords.iter().any(|k| desc.contains(k) || mfr.contains(k));
    }
    false
}

/// Get a human-readable description for a port
fn port_description(port: &SerialPortInfo) -> String {
    match &port.port_type {
        SerialPortType::UsbPort(usb) => {
            usb.product.clone().unwrap_or_else(|| "USB Serial".into())
        }
        SerialPortType::BluetoothPort => "Bluetooth".into(),
        SerialPortType::PciPort => "PCI".into(),
        SerialPortType::Unknown => "Unknown".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_pot() {
        let msg = parse_message("P0:512\n");
        assert!(matches!(msg, Some(ArduinoMessage::Pot { id: 0, value: 512 })));
    }

    #[test]
    fn test_parse_button_press() {
        let msg = parse_message("B3:1\n");
        assert!(matches!(msg, Some(ArduinoMessage::Button { id: 3, pressed: true })));
    }

    #[test]
    fn test_parse_button_release() {
        let msg = parse_message("B3:0\n");
        assert!(matches!(msg, Some(ArduinoMessage::Button { id: 3, pressed: false })));
    }

    #[test]
    fn test_parse_invalid() {
        assert!(parse_message("X0:0\n").is_none());
        assert!(parse_message("").is_none());
        assert!(parse_message("P").is_none());
    }
}
