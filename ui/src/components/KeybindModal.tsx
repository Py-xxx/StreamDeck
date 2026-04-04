import { useEffect, useRef, useState } from "react";
import type { ActionType, AppConfig } from "../types";
import { pickExecutable } from "../configStore";

interface Props {
  buttonId: number;
  config: AppConfig;
  updateConfig: (updater: (prev: AppConfig) => AppConfig) => void;
  onClose: () => void;
}

const KEY_MAP: Record<string, string> = {
  ArrowLeft: "left",
  ArrowRight: "right",
  ArrowUp: "up",
  ArrowDown: "down",
  " ": "space",
  Enter: "enter",
  Escape: "esc",
  Tab: "tab",
  Backspace: "backspace",
  Delete: "delete",
  Insert: "insert",
  Home: "home",
  End: "end",
  PageUp: "page up",
  PageDown: "page down",
  MediaPlayPause: "play/pause media",
  MediaTrackNext: "next track",
  MediaTrackPrevious: "previous track",
  AudioVolumeUp: "volume up",
  AudioVolumeDown: "volume down",
  AudioVolumeMute: "volume mute",
};

// Generate F1–F24 entries
for (let i = 1; i <= 24; i++) {
  KEY_MAP[`F${i}`] = `f${i}`;
}

const MODIFIER_KEYS = new Set(["Control", "Alt", "Shift", "Meta"]);

function normalizeKey(key: string): string {
  if (KEY_MAP[key]) return KEY_MAP[key];
  return key.toLowerCase();
}

const MOUSE_OPTIONS = [
  { value: "mouse_left", label: "Left Click" },
  { value: "mouse_right", label: "Right Click" },
  { value: "mouse_middle", label: "Middle Click" },
  { value: "mouse_double", label: "Double Click" },
];

const MULTIMEDIA_OPTIONS = [
  { value: "playpause", label: "Play / Pause" },
  { value: "medianexttrack", label: "Next Track" },
  { value: "mediaprevtrack", label: "Previous Track" },
  { value: "mediastop", label: "Stop" },
  { value: "volumemute", label: "Mute" },
  { value: "volumeup", label: "Volume Up" },
  { value: "volumedown", label: "Volume Down" },
];

const VM_STRIPS = [
  { value: 0, label: "HW Input 1" },
  { value: 1, label: "HW Input 2" },
  { value: 2, label: "HW Input 3" },
  { value: 3, label: "Virtual 1 (VAIO)" },
  { value: 4, label: "Virtual 2 (AUX)" },
];

const VM_ACTIONS = [
  { value: "mute", label: "🔇 Mute" },
  { value: "solo", label: "🎧 Solo" },
  { value: "mono", label: "◉ Mono" },
  { value: "A1", label: "A1" },
  { value: "A2", label: "A2" },
  { value: "A3", label: "A3" },
  { value: "A4", label: "A4" },
  { value: "A5", label: "A5" },
  { value: "B1", label: "B1" },
  { value: "B2", label: "B2" },
  { value: "B3", label: "B3" },
];

type Category = "keyboard" | "mouse" | "multimedia" | "launch" | "voicemeeter";

interface CategoryInfo {
  id: Category;
  label: string;
  icon: string;
}

const CATEGORIES: CategoryInfo[] = [
  { id: "keyboard", label: "Keyboard", icon: "⌨️" },
  { id: "mouse", label: "Mouse", icon: "🖱️" },
  { id: "multimedia", label: "Multimedia", icon: "🎵" },
  { id: "launch", label: "Launch App", icon: "🚀" },
  { id: "voicemeeter", label: "Voicemeeter", icon: "🎚️" },
];

function detectCategory(action: string): Category {
  if (!action) return "keyboard";
  if (action.startsWith("voicemeeter:")) return "voicemeeter";
  if (action.startsWith("mouse_")) return "mouse";
  if (action.startsWith("launch:")) return "launch";
  if (MULTIMEDIA_OPTIONS.some((opt) => opt.value === action)) return "multimedia";
  return "keyboard";
}

export default function KeybindModal({ buttonId, config, updateConfig, onClose }: Props) {
  const profile = config.profiles[config.active_profile];
  const existing = profile?.buttons[String(buttonId)];

  const [labelText, setLabelText] = useState(existing?.label ?? "");
  const [capturedAction, setCapturedAction] = useState(existing?.action ?? "");
  const [isCapturing, setIsCapturing] = useState(false);
  const [activeCategory, setActiveCategory] = useState<Category>(
    existing?.action_type ?? detectCategory(existing?.action ?? "")
  );
  
  // Parse voicemeeter action if exists
  const parsedVm = existing?.action.startsWith("voicemeeter:") 
    ? existing.action.substring(12).split(":")
    : null;
  const [vmAction, setVmAction] = useState<string>(parsedVm?.[0] ?? "mute");
  const [vmStrip, setVmStrip] = useState<number>(parsedVm?.[1] ? parseInt(parsedVm[1]) : 0);

  const overlayRef = useRef<HTMLDivElement>(null);

  // Key capture listener
  useEffect(() => {
    if (!isCapturing) return;

    const handler = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();

      const key = e.key;

      // Pressing Escape cancels capture without updating
      if (key === "Escape") {
        setIsCapturing(false);
        return;
      }

      // Ignore lone modifiers
      if (MODIFIER_KEYS.has(key)) return;

      const parts: string[] = [];
      if (e.ctrlKey) parts.push("ctrl");
      if (e.altKey) parts.push("alt");
      if (e.shiftKey) parts.push("shift");
      if (e.metaKey) parts.push("win");

      parts.push(normalizeKey(key));

      const combo = parts.join("+");
      setCapturedAction(combo);
      setIsCapturing(false);
    };

    window.addEventListener("keydown", handler, true);
    return () => window.removeEventListener("keydown", handler, true);
  }, [isCapturing]);

  // Close on overlay click
  const handleOverlayClick = (e: React.MouseEvent) => {
    if (e.target === overlayRef.current) onClose();
  };

  const handleSave = () => {
    const finalLabel = labelText.trim() || getDefaultLabel();
    let finalAction = capturedAction.trim();
    
    // Build voicemeeter action string
    if (activeCategory === "voicemeeter") {
      finalAction = `voicemeeter:${vmAction}:${vmStrip}`;
    }

    updateConfig((prev) => {
      const p = prev.profiles[prev.active_profile];
      return {
        ...prev,
        profiles: {
          ...prev.profiles,
          [prev.active_profile]: {
            ...p,
            buttons: {
              ...p?.buttons,
              [String(buttonId)]: {
                label: finalLabel,
                action: finalAction,
                action_type: activeCategory as ActionType,
              },
            },
          },
        },
      };
    });
    onClose();
  };

  const getDefaultLabel = () => {
    if (activeCategory === "voicemeeter") {
      const stripLabel = VM_STRIPS.find((s) => s.value === vmStrip)?.label ?? `Strip ${vmStrip}`;
      const actionLabel = VM_ACTIONS.find((a) => a.value === vmAction)?.label ?? vmAction;
      return `${actionLabel} ${stripLabel}`;
    }
    if (activeCategory === "mouse") {
      return MOUSE_OPTIONS.find((o) => o.value === capturedAction)?.label ?? "Mouse";
    }
    if (activeCategory === "multimedia") {
      return MULTIMEDIA_OPTIONS.find((o) => o.value === capturedAction)?.label ?? "Media";
    }
    if (activeCategory === "launch") {
      const path = capturedAction.replace("launch:", "");
      const name = path.split("\\").pop()?.split("/").pop() ?? "App";
      return name.replace(".exe", "");
    }
    return capturedAction;
  };

  const handleClear = () => {
    setLabelText("");
    setCapturedAction("");

    updateConfig((prev) => {
      const p = prev.profiles[prev.active_profile];
      if (!p) return prev;
      const nextButtons = { ...p.buttons };
      delete nextButtons[String(buttonId)];
      return {
        ...prev,
        profiles: {
          ...prev.profiles,
          [prev.active_profile]: {
            ...p,
            buttons: nextButtons,
          },
        },
      };
    });
    onClose();
  };

  const handleBrowse = async () => {
    const path = await pickExecutable();
    if (path) {
      setCapturedAction(`launch:${path}`);
      // Auto-set label to exe name
      const name = path.split("\\").pop()?.split("/").pop() ?? "App";
      if (!labelText.trim()) {
        setLabelText(name.replace(".exe", ""));
      }
    }
  };

  const captureAreaClass =
    "modal-capture-area" +
    (isCapturing ? " active" : capturedAction ? " has-binding" : "");

  const renderCategoryContent = () => {
    switch (activeCategory) {
      case "keyboard":
        return (
          <>
            {/* Capture area */}
            <div className="modal-section-label">Press a key combination</div>
            <div
              className={captureAreaClass}
              onClick={() => setIsCapturing(true)}
            >
              {isCapturing ? (
                <span style={{ color: "var(--accent)", fontSize: 13 }}>
                  Press keys now…
                </span>
              ) : capturedAction && activeCategory === "keyboard" ? (
                capturedAction
              ) : (
                <span className="capture-placeholder">
                  Click here, then press keys…
                </span>
              )}
            </div>

            {/* OR divider */}
            <div className="modal-or-divider" style={{ margin: "10px 0" }}>
              or type manually
            </div>

            {/* Manual text input */}
            <input
              className="modal-input"
              type="text"
              value={activeCategory === "keyboard" ? capturedAction : ""}
              placeholder="e.g. ctrl+shift+s"
              onChange={(e) => {
                setCapturedAction(e.target.value);
                setIsCapturing(false);
              }}
            />
          </>
        );

      case "mouse":
        return (
          <>
            <div className="modal-section-label">Select mouse action</div>
            <div className="keybind-option-list">
              {MOUSE_OPTIONS.map((opt) => (
                <button
                  key={opt.value}
                  className={`keybind-option ${capturedAction === opt.value ? "selected" : ""}`}
                  onClick={() => setCapturedAction(opt.value)}
                >
                  {opt.label}
                </button>
              ))}
            </div>
          </>
        );

      case "multimedia":
        return (
          <>
            <div className="modal-section-label">Select media action</div>
            <div className="keybind-option-list">
              {MULTIMEDIA_OPTIONS.map((opt) => (
                <button
                  key={opt.value}
                  className={`keybind-option ${capturedAction === opt.value ? "selected" : ""}`}
                  onClick={() => setCapturedAction(opt.value)}
                >
                  {opt.label}
                </button>
              ))}
            </div>
          </>
        );

      case "voicemeeter":
        return (
          <>
            <div className="modal-section-label">Action</div>
            <div className="keybind-option-list">
              {VM_ACTIONS.map((opt) => (
                <button
                  key={opt.value}
                  className={`keybind-option ${vmAction === opt.value ? "selected" : ""}`}
                  onClick={() => setVmAction(opt.value)}
                >
                  {opt.label}
                </button>
              ))}
            </div>
            <div className="modal-section-label" style={{ marginTop: 16 }}>Strip</div>
            <div className="keybind-option-list">
              {VM_STRIPS.map((opt) => (
                <button
                  key={opt.value}
                  className={`keybind-option ${vmStrip === opt.value ? "selected" : ""}`}
                  onClick={() => setVmStrip(opt.value)}
                >
                  {opt.label}
                </button>
              ))}
            </div>
          </>
        );

      case "launch":
        return (
          <>
            <div className="modal-section-label">Select application to launch</div>
            <button className="btn-primary browse-btn" onClick={handleBrowse}>
              📂 Browse for .exe
            </button>
            {capturedAction.startsWith("launch:") && (
              <div className="selected-app">
                <span className="app-icon">🎯</span>
                <span className="app-path">{capturedAction.replace("launch:", "")}</span>
              </div>
            )}
          </>
        );

      default:
        return null;
    }
  };

  return (
    <div className="modal-overlay" ref={overlayRef} onClick={handleOverlayClick}>
      <div className="keybind-modal" onClick={(e) => e.stopPropagation()}>
        <div className="keybind-modal-header">
          <div className="modal-title">Configure Button {buttonId}</div>
        </div>

        <div className="keybind-modal-body">
          {/* Sidebar */}
          <div className="keybind-sidebar">
            {CATEGORIES.map((cat) => (
              <button
                key={cat.id}
                className={`sidebar-item ${activeCategory === cat.id ? "active" : ""}`}
                onClick={() => {
                  setActiveCategory(cat.id);
                  // Clear action when switching categories
                  if (cat.id !== activeCategory) {
                    setCapturedAction("");
                  }
                }}
              >
                <span className="sidebar-icon">{cat.icon}</span>
                <span className="sidebar-label">{cat.label}</span>
              </button>
            ))}
          </div>

          {/* Content */}
          <div className="keybind-content">
            {/* Label section */}
            <div style={{ marginBottom: 16 }}>
              <div className="modal-section-label">Label (optional)</div>
              <input
                className="modal-label-input"
                type="text"
                value={labelText}
                placeholder="Button label"
                onChange={(e) => setLabelText(e.target.value)}
              />
            </div>

            {/* Category-specific content */}
            {renderCategoryContent()}
          </div>
        </div>

        {/* Actions */}
        <div className="modal-actions">
          <button className="btn-danger" onClick={handleClear}>
            Clear
          </button>
          <button className="btn-secondary" onClick={onClose}>
            Cancel
          </button>
          <button className="btn-primary" onClick={handleSave}>
            Save
          </button>
        </div>
      </div>
    </div>
  );
}
