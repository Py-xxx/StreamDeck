import { useEffect, useRef, useState } from "react";
import type { AppConfig } from "./types";
import { loadConfig, saveConfig, getConfigPath, DEFAULT_CONFIG } from "./configStore";
import ProfileBar from "./components/ProfileBar";
import ButtonGrid from "./components/ButtonGrid";
import PotRow from "./components/PotRow";
import KeybindModal from "./components/KeybindModal";
import PinEditModal from "./components/PinEditModal";
import PotCalibrationModal from "./components/PotCalibrationModal";
import QuickAssignModal from "./components/QuickAssignModal";
import AdvancedSettings from "./components/AdvancedSettings";

export default function App() {
  const [config, setConfig] = useState<AppConfig | null>(null);
  const [modalButtonId, setModalButtonId] = useState<number | null>(null);
  const [pinEditButtonId, setPinEditButtonId] = useState<number | null>(null);
  const [calibratePotId, setCalibratePotId] = useState<number | null>(null);
  const [showQuickAssign, setShowQuickAssign] = useState(false);
  const [showAdvanced, setShowAdvanced] = useState(false);
  const [configPath, setConfigPath] = useState<string>("");

  const saveTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const pollTimer = useRef<ReturnType<typeof setInterval> | null>(null);

  // Initial load
  useEffect(() => {
    loadConfig()
      .then((cfg) => setConfig(cfg))
      .catch((err) => {
        console.error("Failed to load config:", err);
        // Use default config as fallback
        setConfig(structuredClone(DEFAULT_CONFIG) as AppConfig);
      });
    getConfigPath()
      .then((p) => setConfigPath(p))
      .catch((err) => console.error("Failed to get config path:", err));
  }, []);

  // Poll every 1500ms for external changes (Python daemon switching profiles)
  useEffect(() => {
    if (!config) return;

    pollTimer.current = setInterval(async () => {
      try {
        const fresh = await loadConfig();
        setConfig((prev) => {
          if (!prev) return fresh;
          // Only update if active_profile changed (Python toggled it)
          if (fresh.active_profile !== prev.active_profile) {
            return { ...prev, active_profile: fresh.active_profile };
          }
          return prev;
        });
      } catch {
        // Ignore poll errors silently
      }
    }, 1500);

    return () => {
      if (pollTimer.current) clearInterval(pollTimer.current);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [config !== null]);

  const updateConfig = (updater: (prev: AppConfig) => AppConfig) => {
    setConfig((prev) => {
      if (!prev) return prev;
      const next = updater(prev);

      // Debounced save
      if (saveTimer.current) clearTimeout(saveTimer.current);
      saveTimer.current = setTimeout(() => {
        saveConfig(next).catch(console.error);
      }, 300);

      return next;
    });
  };

  if (!config) {
    return (
      <div className="app" style={{ alignItems: "center", justifyContent: "center" }}>
        <div style={{ color: "var(--text-muted)", fontSize: 13 }}>Loading config…</div>
      </div>
    );
  }

  return (
    <div className="app">
      {/* Top bar */}
      <div className="topbar">
        <span className="topbar-title">Deckling</span>
        <span className="topbar-hint">config auto-saves • right-click buttons to edit pins</span>
      </div>

      {/* Profile bar */}
      <ProfileBar config={config} updateConfig={updateConfig} />

      {/* Main scrollable area */}
      <div className="main-scroll">
        {/* Deck panel */}
        <div className="deck-panel">
          <ButtonGrid
            config={config}
            updateConfig={updateConfig}
            onButtonClick={(id) => setModalButtonId(id)}
            onPinEditClick={(id) => setPinEditButtonId(id)}
          />
          <PotRow 
            config={config} 
            updateConfig={updateConfig} 
            onCalibrateClick={(id) => setCalibratePotId(id)}
          />
        </div>

        {/* Advanced settings collapsible */}
        <AdvancedSettings
          config={config}
          updateConfig={updateConfig}
          configPath={configPath}
          expanded={showAdvanced}
          onToggle={() => setShowAdvanced((v) => !v)}
          onQuickAssign={() => setShowQuickAssign(true)}
        />
      </div>

      {/* Keybind modal */}
      {modalButtonId !== null && (
        <KeybindModal
          buttonId={modalButtonId}
          config={config}
          updateConfig={updateConfig}
          onClose={() => setModalButtonId(null)}
        />
      )}

      {/* Pin edit modal */}
      {pinEditButtonId !== null && (
        <PinEditModal
          buttonId={pinEditButtonId}
          config={config}
          updateConfig={updateConfig}
          onClose={() => setPinEditButtonId(null)}
        />
      )}

      {/* Pot calibration modal */}
      {calibratePotId !== null && (
        <PotCalibrationModal
          potId={calibratePotId}
          config={config}
          updateConfig={updateConfig}
          onClose={() => setCalibratePotId(null)}
        />
      )}

      {/* Quick assign modal */}
      {showQuickAssign && (
        <QuickAssignModal
          config={config}
          updateConfig={updateConfig}
          onClose={() => setShowQuickAssign(false)}
        />
      )}
    </div>
  );
}
