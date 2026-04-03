# StreamDeck Controller

A Tauri + React/TypeScript desktop app for the StreamDeck hardware controller. This single app handles everything:
- Serial communication with Arduino
- Voicemeeter Banana gain control
- Keyboard shortcut execution
- Configuration UI

---

## Prerequisites

- **Node.js 18+** — [nodejs.org](https://nodejs.org)
- **Rust** — install via [rustup.rs](https://rustup.rs)

---

## Install Dependencies

```bash
npm install
```

---

## Run in Development

```bash
npm run tauri dev
```

This starts the Vite dev server and opens the Tauri window with hot-reload.

---

## Build for Production

```bash
npm run tauri build
```

The compiled binary and installer will be in `src-tauri/target/release/bundle/`.

---

## First Run

On first launch, the app automatically creates:
- `~/.streamdeck/` directory
- `~/.streamdeck/config.json` with default settings

---

## Settings

In Advanced Settings you can configure:
- **Serial Port** — Select the COM port for your Arduino
- **Auto-Connect** — Automatically connect on app startup
- **Launch on Startup** — Start StreamDeck when Windows boots
- **Display Grid** — Button grid dimensions
- **Number of Pots** — How many potentiometers you have
- **Profile Toggle** — Assign a button to switch profiles
- **Hardware Pins** — Arduino pin assignments

---

## Arduino Firmware

The Arduino firmware lives in `../arduino/`. Open `arduino.ino` in Arduino IDE:

1. Select the correct board (e.g., Arduino Nano or Uno)
2. Select the correct COM port
3. Click Upload

Make sure the COM port in the app matches the one your Arduino is connected to.
