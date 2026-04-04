import { useRef, useState } from "react";
import type { AppConfig } from "../types";

interface Props {
  buttonId: number;
  config: AppConfig;
  updateConfig: (updater: (prev: AppConfig) => AppConfig) => void;
  onClose: () => void;
}

export default function PinEditModal({ buttonId, config, updateConfig, onClose }: Props) {
  const { row_pins, col_pins, button_pins } = config.hardware;
  const existing = button_pins?.[String(buttonId)];

  const [selectedRowPin, setSelectedRowPin] = useState<number>(existing?.row_pin ?? row_pins[0] ?? 2);
  const [selectedColPin, setSelectedColPin] = useState<number>(existing?.col_pin ?? col_pins[0] ?? 5);

  const overlayRef = useRef<HTMLDivElement>(null);

  const handleOverlayClick = (e: React.MouseEvent) => {
    if (e.target === overlayRef.current) onClose();
  };

  const handleSave = () => {
    updateConfig((prev) => {
      const currentPins = prev.hardware.button_pins || {};
      return {
        ...prev,
        hardware: {
          ...prev.hardware,
          button_pins: {
            ...currentPins,
            [String(buttonId)]: {
              row_pin: selectedRowPin,
              col_pin: selectedColPin,
            },
          },
        },
      };
    });
    onClose();
  };

  // Check if another button is using this pin combination
  const getPinConflict = (): string | null => {
    if (!button_pins) return null;
    
    for (const [id, mapping] of Object.entries(button_pins)) {
      if (id !== String(buttonId) && 
          mapping.row_pin === selectedRowPin && 
          mapping.col_pin === selectedColPin) {
        return id;
      }
    }
    return null;
  };

  const conflict = getPinConflict();

  return (
    <div className="modal-overlay" ref={overlayRef} onClick={handleOverlayClick}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <div className="modal-title">Configure Pins for Button {buttonId}</div>

        <div className="pin-edit-description">
          Select which physical button (row pin + column pin combination) should trigger this UI button.
        </div>

        {/* Row Pin Selection */}
        <div className="pin-select-group">
          <div className="modal-section-label">Row Pin</div>
          <div className="pin-options">
            {row_pins.map((pin) => (
              <button
                key={pin}
                className={`pin-option ${selectedRowPin === pin ? "selected" : ""}`}
                onClick={() => setSelectedRowPin(pin)}
              >
                D{pin}
              </button>
            ))}
          </div>
        </div>

        {/* Column Pin Selection */}
        <div className="pin-select-group">
          <div className="modal-section-label">Column Pin</div>
          <div className="pin-options">
            {col_pins.map((pin) => (
              <button
                key={pin}
                className={`pin-option ${selectedColPin === pin ? "selected" : ""}`}
                onClick={() => setSelectedColPin(pin)}
              >
                D{pin}
              </button>
            ))}
          </div>
        </div>

        {/* Preview */}
        <div className="pin-preview">
          <span className="pin-preview-label">Selected:</span>
          <span className="pin-preview-value">D{selectedRowPin} - D{selectedColPin}</span>
        </div>

        {/* Conflict Warning */}
        {conflict && (
          <div className="pin-conflict-warning">
            ⚠️ Button {conflict} is already using this pin combination. Saving will swap the assignments.
          </div>
        )}

        {/* Actions */}
        <div className="modal-actions">
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
