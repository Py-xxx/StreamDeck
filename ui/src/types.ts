// Action types for buttons
export type ActionType = "keyboard" | "mouse" | "multimedia" | "launch" | "voicemeeter";

export interface ButtonConfig {
  label: string;
  action: string;
  action_type?: ActionType; // Optional for backwards compatibility
}

export interface PotCalibration {
  enabled: boolean;
  raw_min: number;  // Raw ADC value at pot's minimum position
  raw_max: number;  // Raw ADC value at pot's maximum position
}

export interface PotConfig {
  label: string;
  strip: number;
  calibration?: PotCalibration;
  inverted?: boolean; // Invert pot direction (swap min/max)
}

export interface Profile {
  buttons: Record<string, ButtonConfig>;
  pots: Record<string, PotConfig>;
}

export interface ProfileToggle {
  button_id: number;
  mode: "hold" | "tap";
  hold_ms: number;
  cycle_profiles: string[]; // Which profiles to cycle through
  primary_profile?: string; // For hold mode: which profile is active when button not held
}

export interface Display {
  grid_rows: number;
  grid_cols: number;
  num_pots: number;
}

// Pin assignment for a button (row_pin, col_pin)
export interface ButtonPinMapping {
  row_pin: number;
  col_pin: number;
}

export interface Hardware {
  row_pins: number[];
  col_pins: number[];
  pot_pins: number[];
  // Maps UI button position to (row_pin, col_pin) pair
  // Key is button position (0, 1, 2...), value is pin pair
  button_pins?: Record<string, ButtonPinMapping>;
  // Prevent multiple button presses (ignore when >1 button pressed)
  prevent_multi_press?: boolean;
}

export interface AppConfig {
  serial_port: string;
  active_profile: string;
  display: Display;
  hardware: Hardware;
  profile_toggle: ProfileToggle;
  profiles: Record<string, Profile>;
  auto_connect: boolean;
  launch_on_startup: boolean;
}

// Serial port info from Rust backend
export interface PortInfo {
  name: string;
  description: string;
  is_arduino: boolean;
}

// Connection status from Rust backend
export interface ConnectionStatus {
  connected: boolean;
  port: string | null;
  error: string | null;
}
