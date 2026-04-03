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
  },
  profile_toggle: {
    button_id: -1,
    mode: "hold",
    hold_ms: 500,
  },
  profiles: {
    Default: {
      buttons: {
        "0": { label: "Mute Mic", action: "ctrl+alt+m" },
        "1": { label: "Screenshot", action: "ctrl+shift+s" },
        "2": { label: "Alt+Tab", action: "alt+tab" },
        "3": { label: "Copy", action: "ctrl+c" },
        "4": { label: "Paste", action: "ctrl+v" },
        "5": { label: "Vol Up", action: "volumeup" },
        "6": { label: "Vol Down", action: "volumedown" },
        "7": { label: "Play/Pause", action: "playpause" },
        "8": { label: "Next", action: "medianexttrack" },
        "9": { label: "Prev", action: "mediaprevtrack" },
        "10": { label: "Desktop", action: "win+d" },
        "11": { label: "Explorer", action: "win+e" },
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

async function getStreamdeckDir(): Promise<string> {
  const home = await homeDir();
  return await join(home, ".streamdeck");
}

export async function getConfigPath(): Promise<string> {
  const dir = await getStreamdeckDir();
  return await join(dir, "config.json");
}

export async function loadConfig(): Promise<AppConfig> {
  const dir = await getStreamdeckDir();
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
