import type { AppConfig } from "../types";

interface Props {
  config: AppConfig;
  updateConfig: (updater: (prev: AppConfig) => AppConfig) => void;
  onButtonClick: (buttonId: number) => void;
  onPinEditClick?: (buttonId: number) => void;
}

export default function ButtonGrid({ config, onButtonClick, onPinEditClick }: Props) {
  const { grid_rows, grid_cols } = config.display;
  const total = grid_rows * grid_cols;
  const profile = config.profiles[config.active_profile];
  const toggleId = config.profile_toggle.button_id;
  const buttonPins = config.hardware.button_pins || {};

  const cells = Array.from({ length: total }, (_, i) => i);

  const getPinLabel = (id: number): string => {
    const mapping = buttonPins[String(id)];
    if (mapping) {
      return `D${mapping.row_pin}-D${mapping.col_pin}`;
    }
    return "";
  };

  return (
    <div
      className="button-grid"
      style={{ gridTemplateColumns: `repeat(${grid_cols}, 88px)` }}
    >
      {cells.map((id) => {
        const binding = profile?.buttons[String(id)];
        const isToggle = id === toggleId;
        const isEmpty = !binding;
        const pinLabel = getPinLabel(id);

        let classes = "btn-cell";
        if (isToggle) classes += " is-toggle";
        else if (binding?.action) classes += " has-action";
        if (isEmpty && !isToggle) classes += " btn-empty";

        const handleClick = () => {
          // Don't allow editing the profile toggle button
          if (isToggle) return;
          onButtonClick(id);
        };

        const handleRightClick = (e: React.MouseEvent) => {
          e.preventDefault();
          if (onPinEditClick) {
            onPinEditClick(id);
          }
        };

        return (
          <div
            key={id}
            className={classes}
            onClick={handleClick}
            onContextMenu={handleRightClick}
            style={isToggle ? { cursor: "default" } : undefined}
            title={
              isToggle
                ? `Profile Toggle (${pinLabel})\nConfigured in Advanced Settings`
                : binding
                  ? `${binding.label} — ${binding.action}\nPins: ${pinLabel}\nRight-click to edit pins`
                  : `Button ${id}\nPins: ${pinLabel}\nRight-click to edit pins`
            }
          >
            <span className="btn-id">{id}</span>

            {isToggle ? (
              <>
                <span className="btn-label" style={{ color: "var(--toggle-color)" }}>
                  {binding?.label || ""}
                </span>
                <span className="btn-toggle-indicator">PROFILE</span>
              </>
            ) : (
              <>
                <span className="btn-label">{binding?.label || "—"}</span>
                {binding?.action && (
                  <span className="btn-action">{binding.action}</span>
                )}
              </>
            )}
            
            {/* Pin indicator */}
            <span className="btn-pin-label">{pinLabel}</span>
          </div>
        );
      })}
    </div>
  );
}
