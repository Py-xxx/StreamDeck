import {
  readTextFile,
  writeTextFile,
  createDir,
  exists,
} from "@tauri-apps/api/fs";
import { homeDir, join } from "@tauri-apps/api/path";
import { invoke } from "@tauri-apps/api/tauri";
import type { AppConfig, PortInfo, ConnectionStatus } from "./types";

export const DEFAULT_CONFIG: AppConfig = {
  serial_port: "COM3",
  active_profile: "Default",
  display: {
    grid_rows: 3,
    grid_cols: 4,
    num_pots: 4,
  },
  hardware: {
    row_pins: [2, 3, 4],
    col_pins: [5, 6, 7, 8],
    pot_pins: [0, 1, 2, 3],
    // Default: sequential mapping (button 0 = row0/col0, button 1 = row0/col1, etc.)
    button_pins: {
      "0": { row_pin: 2, col_pin: 5 },
      "1": { row_pin: 2, col_pin: 6 },
      "2": { row_pin: 2, col_pin: 7 },
      "3": { row_pin: 2, col_pin: 8 },
      "4": { row_pin: 3, col_pin: 5 },
      "5": { row_pin: 3, col_pin: 6 },
      "6": { row_pin: 3, col_pin: 7 },
      "7": { row_pin: 3, col_pin: 8 },
      "8": { row_pin: 4, col_pin: 5 },
      "9": { row_pin: 4, col_pin: 6 },
      "10": { row_pin: 4, col_pin: 7 },
      "11": { row_pin: 4, col_pin: 8 },
    },
    invert_pots: false,
  },
  profile_toggle: {
    button_id: -1,
    mode: "hold",
    hold_ms: 500,
    cycle_profiles: [], // Empty = cycle all profiles
  },
  profiles: {
    Default: {
      buttons: {
        "0": { label: "Mute Mic", action: "ctrl+alt+m", action_type: "keyboard" },
        "1": { label: "Screenshot", action: "ctrl+shift+s", action_type: "keyboard" },
        "2": { label: "Alt+Tab", action: "alt+tab", action_type: "keyboard" },
        "3": { label: "Copy", action: "ctrl+c", action_type: "keyboard" },
        "4": { label: "Paste", action: "ctrl+v", action_type: "keyboard" },
        "5": { label: "Vol Up", action: "volumeup", action_type: "multimedia" },
        "6": { label: "Vol Down", action: "volumedown", action_type: "multimedia" },
        "7": { label: "Play/Pause", action: "playpause", action_type: "multimedia" },
        "8": { label: "Next", action: "medianexttrack", action_type: "multimedia" },
        "9": { label: "Prev", action: "mediaprevtrack", action_type: "multimedia" },
        "10": { label: "Desktop", action: "win+d", action_type: "keyboard" },
        "11": { label: "Explorer", action: "win+e", action_type: "keyboard" },
      },
      pots: {
        "0": { label: "HW Input 1", strip: 0 },
        "1": { label: "HW Input 2", strip: 1 },
        "2": { label: "Virtual 1", strip: 3 },
        "3": { label: "Virtual 2", strip: 4 },
      },
    },
  },
  auto_connect: true,
  launch_on_startup: false,
};

async function getDecklingDir(): Promise<string> {
  const home = await homeDir();
  return await join(home, ".deckling");
}

export async function getConfigPath(): Promise<string> {
  const dir = await getDecklingDir();
  return await join(dir, "config.json");
}

export async function loadConfig(): Promise<AppConfig> {
  const dir = await getDecklingDir();
  const configPath = await join(dir, "config.json");

  // Ensure directory exists
  const dirExists = await exists(dir);
  if (!dirExists) {
    await createDir(dir, { recursive: true });
  }

  // Create default config if not exists
  const fileExists = await exists(configPath);
  if (!fileExists) {
    await writeTextFile(configPath, JSON.stringify(DEFAULT_CONFIG, null, 2));
    return structuredClone(DEFAULT_CONFIG);
  }

  const raw = await readTextFile(configPath);
  const parsed = JSON.parse(raw) as AppConfig;
  
  // Ensure new fields have defaults
  if (parsed.auto_connect === undefined) parsed.auto_connect = true;
  if (parsed.launch_on_startup === undefined) parsed.launch_on_startup = false;
  if (parsed.profile_toggle.cycle_profiles === undefined) parsed.profile_toggle.cycle_profiles = [];
  
  // Initialize button_pins if missing (generate default mapping)
  if (!parsed.hardware.button_pins) {
    const buttonPins: Record<string, { row_pin: number; col_pin: number }> = {};
    const { row_pins, col_pins } = parsed.hardware;
    const total = parsed.display.grid_rows * parsed.display.grid_cols;
    
    for (let i = 0; i < total; i++) {
      const rowIdx = Math.floor(i / col_pins.length);
      const colIdx = i % col_pins.length;
      if (row_pins[rowIdx] !== undefined && col_pins[colIdx] !== undefined) {
        buttonPins[String(i)] = {
          row_pin: row_pins[rowIdx],
          col_pin: col_pins[colIdx],
        };
      }
    }
    parsed.hardware.button_pins = buttonPins;
  }
  
  return parsed;
}

export async function saveConfig(config: AppConfig): Promise<void> {
  const configPath = await getConfigPath();
  await writeTextFile(configPath, JSON.stringify(config, null, 2));
  
  // Notify Rust daemon of config change
  try {
    await invoke("notify_config_updated", { config });
  } catch (e) {
    console.error("Failed to notify daemon:", e);
  }
}

// ============================================================================
// Serial port functions
// ============================================================================

export async function listSerialPorts(): Promise<PortInfo[]> {
  return await invoke<PortInfo[]>("list_serial_ports");
}

export async function connectSerial(port: string): Promise<void> {
  await invoke("connect_serial", { port });
}

export async function disconnectSerial(): Promise<void> {
  await invoke("disconnect_serial");
}

export async function getConnectionStatus(): Promise<ConnectionStatus> {
  return await invoke<ConnectionStatus>("get_connection_status");
}

// ============================================================================
// Voicemeeter functions
// ============================================================================

export async function initVoicemeeter(): Promise<boolean> {
  return await invoke<boolean>("init_voicemeeter");
}

export async function isVoicemeeterAvailable(): Promise<boolean> {
  return await invoke<boolean>("is_voicemeeter_available");
}

// ============================================================================
// Startup functions
// ============================================================================

export async function getLaunchOnStartup(): Promise<boolean> {
  return await invoke<boolean>("get_launch_on_startup");
}

export async function setLaunchOnStartup(enabled: boolean): Promise<void> {
  await invoke("set_launch_on_startup", { enabled });
}

// ============================================================================
// File picker
// ============================================================================

export async function pickExecutable(): Promise<string | null> {
  return await invoke<string | null>("pick_executable");
}

// ============================================================================
// Calibration
// ============================================================================

export async function getRawPotValue(potId: number): Promise<number | null> {
  return await invoke<number | null>("get_raw_pot_value", { potId });
}
