import { useRef, useState, useEffect } from "react";
import type { AppConfig, PortInfo, ConnectionStatus } from "../types";
import {
  listSerialPorts,
  connectSerial,
  disconnectSerial,
  getConnectionStatus,
  getLaunchOnStartup,
  setLaunchOnStartup,
  isVoicemeeterAvailable,
} from "../configStore";

interface Props {
  config: AppConfig;
  updateConfig: (updater: (prev: AppConfig) => AppConfig) => void;
  configPath: string;
  expanded: boolean;
  onToggle: () => void;
}

function parsePins(str: string): number[] {
  return str
    .split(",")
    .map((s) => s.trim())
    .filter((s) => s !== "")
    .map((s) => parseInt(s, 10))
    .filter((n) => !isNaN(n));
}

export default function AdvancedSettings({
  config,
  updateConfig,
  configPath,
  expanded,
  onToggle,
}: Props) {
  // Debounce timers
  const timers = useRef<Record<string, ReturnType<typeof setTimeout>>>({});
  
  // Connection state
  const [ports, setPorts] = useState<PortInfo[]>([]);
  const [connectionStatus, setConnectionStatus] = useState<ConnectionStatus>({
    connected: false,
    port: null,
    error: null,
  });
  const [launchOnStartup, setLaunchOnStartupState] = useState(false);
  const [vmAvailable, setVmAvailable] = useState(false);
  const [isConnecting, setIsConnecting] = useState(false);

  // Load initial state
  useEffect(() => {
    refreshPorts();
    refreshConnectionStatus();
    getLaunchOnStartup().then(setLaunchOnStartupState).catch(console.error);
    isVoicemeeterAvailable().then(setVmAvailable).catch(console.error);
    
    // Poll connection status
    const interval = setInterval(refreshConnectionStatus, 2000);
    return () => clearInterval(interval);
  }, []);

  const refreshPorts = async () => {
    try {
      const portList = await listSerialPorts();
      setPorts(portList);
    } catch (e) {
      console.error("Failed to list ports:", e);
    }
  };

  const refreshConnectionStatus = async () => {
    try {
      const status = await getConnectionStatus();
      setConnectionStatus(status);
      setIsConnecting(false);
    } catch (e) {
      console.error("Failed to get connection status:", e);
    }
  };

  const handleConnect = async () => {
    setIsConnecting(true);
    try {
      await connectSerial(config.serial_port);
      await refreshConnectionStatus();
    } catch (e) {
      console.error("Connect failed:", e);
      setConnectionStatus({
        connected: false,
        port: null,
        error: String(e),
      });
    }
    setIsConnecting(false);
  };

  const handleDisconnect = async () => {
    try {
      await disconnectSerial();
      await refreshConnectionStatus();
    } catch (e) {
      console.error("Disconnect failed:", e);
    }
  };

  const handleLaunchOnStartupChange = async (enabled: boolean) => {
    try {
      await setLaunchOnStartup(enabled);
      setLaunchOnStartupState(enabled);
      updateConfig((prev) => ({ ...prev, launch_on_startup: enabled }));
    } catch (e) {
      console.error("Failed to set launch on startup:", e);
    }
  };

  const debounce = (key: string, fn: () => void, delay = 300) => {
    if (timers.current[key]) clearTimeout(timers.current[key]);
    timers.current[key] = setTimeout(fn, delay);
  };

  // Serial port selection
  const handleSerialPort = (value: string) => {
    updateConfig((prev) => ({ ...prev, serial_port: value }));
  };

  // Auto connect toggle
  const handleAutoConnect = (enabled: boolean) => {
    updateConfig((prev) => ({ ...prev, auto_connect: enabled }));
  };

  // Grid rows
  const handleGridRows = (value: string) => {
    const n = parseInt(value, 10);
    if (isNaN(n) || n < 1 || n > 10) return;
    debounce("grid_rows", () => {
      updateConfig((prev) => ({
        ...prev,
        display: { ...prev.display, grid_rows: n },
      }));
    });
  };

  // Grid cols
  const handleGridCols = (value: string) => {
    const n = parseInt(value, 10);
    if (isNaN(n) || n < 1 || n > 10) return;
    debounce("grid_cols", () => {
      updateConfig((prev) => ({
        ...prev,
        display: { ...prev.display, grid_cols: n },
      }));
    });
  };

  // Num pots
  const handleNumPots = (value: string) => {
    const n = parseInt(value, 10);
    if (isNaN(n) || n < 1 || n > 8) return;
    debounce("num_pots", () => {
      updateConfig((prev) => ({
        ...prev,
        display: { ...prev.display, num_pots: n },
      }));
    });
  };

  // Toggle button selection (immediate)
  const handleToggleButtonSelect = (id: number) => {
    updateConfig((prev) => ({
      ...prev,
      profile_toggle: { ...prev.profile_toggle, button_id: id },
    }));
  };

  // Disable toggle (immediate)
  const handleDisableToggle = () => {
    updateConfig((prev) => ({
      ...prev,
      profile_toggle: { ...prev.profile_toggle, button_id: -1 },
    }));
  };

  // Toggle mode (immediate)
  const handleToggleMode = (mode: "hold" | "tap") => {
    updateConfig((prev) => ({
      ...prev,
      profile_toggle: { ...prev.profile_toggle, mode },
    }));
  };

  // Hold ms (debounced)
  const handleHoldMs = (value: number) => {
    debounce("hold_ms", () => {
      updateConfig((prev) => ({
        ...prev,
        profile_toggle: { ...prev.profile_toggle, hold_ms: value },
      }));
    });
  };

  // Row pins
  const handleRowPins = (value: string) => {
    debounce("row_pins", () => {
      updateConfig((prev) => ({
        ...prev,
        hardware: { ...prev.hardware, row_pins: parsePins(value) },
      }));
    });
  };

  // Col pins
  const handleColPins = (value: string) => {
    debounce("col_pins", () => {
      updateConfig((prev) => ({
        ...prev,
        hardware: { ...prev.hardware, col_pins: parsePins(value) },
      }));
    });
  };

  // Pot pins
  const handlePotPins = (value: string) => {
    debounce("pot_pins", () => {
      updateConfig((prev) => ({
        ...prev,
        hardware: { ...prev.hardware, pot_pins: parsePins(value) },
      }));
    });
  };

  const { grid_rows, grid_cols } = config.display;
  const total = grid_rows * grid_cols;
  const toggleId = config.profile_toggle.button_id;
  const profile = config.profiles[config.active_profile];

  return (
    <div className="advanced-section">
      <button
        className={`advanced-toggle ${expanded ? "expanded" : "collapsed"}`}
        onClick={onToggle}
      >
        <span>Advanced Settings</span>
        <span className={`advanced-toggle-chevron${expanded ? " open" : ""}`}>▼</span>
      </button>

      {expanded && (
        <div className="advanced-body">
          {/* 1. Connection Status */}
          <div className="settings-group">
            <label>Connection</label>
            <div className="connection-status">
              <span className={`status-indicator ${connectionStatus.connected ? "connected" : "disconnected"}`}>
                ●
              </span>
              <span className="status-text">
                {connectionStatus.connected
                  ? `Connected to ${connectionStatus.port}`
                  : connectionStatus.error || "Disconnected"}
              </span>
            </div>
            
            {/* Voicemeeter status */}
            <div className="connection-status" style={{ marginTop: 4 }}>
              <span className={`status-indicator ${vmAvailable ? "connected" : "disconnected"}`}>
                ●
              </span>
              <span className="status-text">
                Voicemeeter {vmAvailable ? "available" : "not found"}
              </span>
            </div>
          </div>

          {/* 2. Serial Port */}
          <div className="settings-group">
            <label>Serial Port</label>
            <div className="port-select-row">
              <select
                className="settings-input port-select"
                value={config.serial_port}
                onChange={(e) => handleSerialPort(e.target.value)}
              >
                <option value="">Select a port...</option>
                {ports.map((p) => (
                  <option key={p.name} value={p.name}>
                    {p.name} {p.is_arduino ? "★" : ""} ({p.description})
                  </option>
                ))}
              </select>
              <button
                className="refresh-ports-btn"
                onClick={refreshPorts}
                title="Refresh port list"
              >
                ↻
              </button>
            </div>
            <div className="connect-row">
              {connectionStatus.connected ? (
                <button className="connect-btn disconnect" onClick={handleDisconnect}>
                  Disconnect
                </button>
              ) : (
                <button
                  className="connect-btn"
                  onClick={handleConnect}
                  disabled={!config.serial_port || isConnecting}
                >
                  {isConnecting ? "Connecting..." : "Connect"}
                </button>
              )}
              <label className="auto-connect-label">
                <input
                  type="checkbox"
                  checked={config.auto_connect}
                  onChange={(e) => handleAutoConnect(e.target.checked)}
                />
                Auto-connect on startup
              </label>
            </div>
          </div>

          {/* 3. Launch on Startup */}
          <div className="settings-group">
            <label>Startup</label>
            <label className="startup-toggle">
              <input
                type="checkbox"
                checked={launchOnStartup}
                onChange={(e) => handleLaunchOnStartupChange(e.target.checked)}
              />
              Launch StreamDeck when Windows starts
            </label>
          </div>

          {/* 4. Display Grid */}
          <div className="settings-group">
            <label>Display Grid</label>
            <div className="settings-row">
              <div className="settings-group">
                <label>Rows</label>
                <input
                  className="settings-input"
                  type="number"
                  min={1}
                  max={10}
                  defaultValue={config.display.grid_rows}
                  onChange={(e) => handleGridRows(e.target.value)}
                />
              </div>
              <div className="settings-group">
                <label>Columns</label>
                <input
                  className="settings-input"
                  type="number"
                  min={1}
                  max={10}
                  defaultValue={config.display.grid_cols}
                  onChange={(e) => handleGridCols(e.target.value)}
                />
              </div>
            </div>
            <span className="settings-helper">Visual layout of the StreamDeck</span>
          </div>

          {/* 5. Number of Pots */}
          <div className="settings-group">
            <label>Number of Pots</label>
            <input
              className="settings-input"
              type="number"
              min={1}
              max={8}
              defaultValue={config.display.num_pots}
              onChange={(e) => handleNumPots(e.target.value)}
              style={{ maxWidth: 120 }}
            />
          </div>

          {/* 6. Profile Toggle Button */}
          <div className="settings-group">
            <label>Profile Toggle Button</label>
            <span className="settings-helper" style={{ marginBottom: 8 }}>
              Click a cell to assign the profile-switch button
            </span>
            <div
              className="pin-toggle-grid"
              style={{ gridTemplateColumns: `repeat(${grid_cols}, 48px)` }}
            >
              {Array.from({ length: total }, (_, i) => {
                const binding = profile?.buttons[String(i)];
                const isSelected = i === toggleId;
                return (
                  <div
                    key={i}
                    className={`btn-cell${isSelected ? " is-toggle-picker" : ""}`}
                    style={{ width: 48, height: 48 }}
                    onClick={() => handleToggleButtonSelect(i)}
                    title={`Button ${i}${binding ? `: ${binding.label}` : ""}`}
                  >
                    <span style={{ fontSize: 9, color: isSelected ? "var(--toggle-color)" : "var(--text-muted)" }}>
                      {i}
                    </span>
                    {binding && (
                      <span style={{ fontSize: 8, color: "var(--text-muted)", textAlign: "center", lineHeight: 1.2 }}>
                        {binding.label.slice(0, 6)}
                      </span>
                    )}
                  </div>
                );
              })}
            </div>
            <button
              className="disable-toggle-btn"
              onClick={handleDisableToggle}
            >
              Disable toggle ({toggleId === -1 ? "currently off" : `btn ${toggleId}`})
            </button>
          </div>

          {/* 7. Toggle Mode */}
          <div className="settings-group">
            <label>Toggle Mode</label>
            <div className="toggle-row">
              <button
                className={`toggle-option${config.profile_toggle.mode === "hold" ? " selected" : ""}`}
                onClick={() => handleToggleMode("hold")}
              >
                Hold
              </button>
              <button
                className={`toggle-option${config.profile_toggle.mode === "tap" ? " selected" : ""}`}
                onClick={() => handleToggleMode("tap")}
              >
                Tap
              </button>
            </div>
            {config.profile_toggle.mode === "hold" && (
              <div className="hold-ms-row">
                <span style={{ fontSize: 12, color: "var(--text-muted)" }}>Hold duration</span>
                <input
                  className="hold-ms-slider"
                  type="range"
                  min={100}
                  max={2000}
                  step={50}
                  defaultValue={config.profile_toggle.hold_ms}
                  onChange={(e) => handleHoldMs(parseInt(e.target.value, 10))}
                />
                <span className="hold-ms-value">{config.profile_toggle.hold_ms} ms</span>
              </div>
            )}
          </div>

          {/* 8. Hardware Pins */}
          <div className="settings-group">
            <label>Hardware Pins</label>
            <div className="settings-group">
              <label>Row Pins</label>
              <input
                className="settings-input"
                type="text"
                defaultValue={config.hardware.row_pins.join(", ")}
                placeholder="e.g. 2,3,4,5,6,7"
                onChange={(e) => handleRowPins(e.target.value)}
              />
            </div>
            <div className="settings-group" style={{ marginTop: 8 }}>
              <label>Column Pins</label>
              <input
                className="settings-input"
                type="text"
                defaultValue={config.hardware.col_pins.join(", ")}
                placeholder="e.g. 8,9,10,11"
                onChange={(e) => handleColPins(e.target.value)}
              />
            </div>
            <div className="settings-group" style={{ marginTop: 8 }}>
              <label>Pot Pins (Analog)</label>
              <input
                className="settings-input"
                type="text"
                defaultValue={config.hardware.pot_pins.join(", ")}
                placeholder="e.g. 0,1,2,3"
                onChange={(e) => handlePotPins(e.target.value)}
              />
            </div>
          </div>

          {/* 9. Config File */}
          <div className="settings-group">
            <label>Config File</label>
            <div className="config-path">{configPath || "Resolving…"}</div>
          </div>
        </div>
      )}
    </div>
  );
}
