import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import type { AppConfig } from "../types";
import { startQuickAssign, stopQuickAssign } from "../configStore";

interface Props {
  config: AppConfig;
  updateConfig: (updater: (prev: AppConfig) => AppConfig) => void;
  onClose: () => void;
}

export default function QuickAssignModal({ config, updateConfig, onClose }: Props) {
  const [currentButtonIndex, setCurrentButtonIndex] = useState(0);
  const [isListening, setIsListening] = useState(false);
  const [status, setStatus] = useState("Press the button on your hardware that matches the highlighted button");

  const { grid_rows, grid_cols } = config.display;
  const totalButtons = grid_rows * grid_cols;

  useEffect(() => {
    let unlisten: (() => void) | null = null;

    const setupListener = async () => {
      // Start quick assign mode
      await startQuickAssign();
      setIsListening(true);

      // Listen for button press events
      unlisten = await listen<[number, number]>("quick-assign-button-pressed", (event) => {
        const [rowPin, colPin] = event.payload;
        
        // Assign this pin pair to the current button
        updateConfig((prev) => {
          const currentPins = prev.hardware.button_pins || {};
          
          // Remove this pin pair from any other button
          const newPins = { ...currentPins };
          for (const [id, mapping] of Object.entries(currentPins)) {
            if (mapping.row_pin === rowPin && mapping.col_pin === colPin) {
              delete newPins[id];
            }
          }
          
          // Assign to current button
          newPins[String(currentButtonIndex)] = {
            row_pin: rowPin,
            col_pin: colPin,
          };
          
          return {
            ...prev,
            hardware: {
              ...prev.hardware,
              button_pins: newPins,
            },
          };
        });

        // Move to next button
        if (currentButtonIndex < totalButtons - 1) {
          setCurrentButtonIndex(currentButtonIndex + 1);
          setStatus("Press the button on your hardware that matches the highlighted button");
        } else {
          // All done!
          setStatus("All buttons assigned! Closing...");
          setTimeout(() => {
            handleClose();
          }, 1000);
        }
      });
    };

    setupListener();

    return () => {
      if (unlisten) unlisten();
    };
  }, [currentButtonIndex, totalButtons]);

  const handleClose = async () => {
    if (isListening) {
      await stopQuickAssign();
      setIsListening(false);
    }
    onClose();
  };

  const handleCancel = async () => {
    await handleClose();
  };

  return (
    <div className="modal-overlay" onClick={(e) => e.stopPropagation()}>
      <div className="modal quick-assign-modal">
        <div className="modal-title">Quick Button Assignment</div>

        <div className="quick-assign-instructions">
          <p>{status}</p>
          <p className="quick-assign-progress">
            Button {currentButtonIndex + 1} of {totalButtons}
          </p>
        </div>

        {/* Button Grid Preview */}
        <div className="quick-assign-grid" style={{
          display: "grid",
          gridTemplateColumns: `repeat(${grid_cols}, 1fr)`,
          gap: "8px",
          margin: "20px 0",
        }}>
          {Array.from({ length: totalButtons }, (_, i) => (
            <div
              key={i}
              className={`quick-assign-button ${i === currentButtonIndex ? "highlight" : ""} ${i < currentButtonIndex ? "done" : ""}`}
            >
              {i < currentButtonIndex ? "✓" : i + 1}
            </div>
          ))}
        </div>

        <div className="modal-actions">
          <button className="btn-secondary" onClick={handleCancel}>
            Cancel
          </button>
        </div>
      </div>
    </div>
  );
}
