"""
StreamDeck – host-side controller
Reads serial from Arduino → Voicemeeter Banana gain + keyboard shortcuts.
"""

import time
import serial
import serial.tools.list_ports
import keyboard
import voicemeeterlib

# ================================================================
#  CONFIG
# ================================================================
SERIAL_PORT = "COM3"
BAUD_RATE   = 115200

GAIN_MIN   = -60
GAIN_MAX   =  12
GAIN_RANGE = GAIN_MAX - GAIN_MIN   # 72

# Potentiometer index (0–3, matching A0–A3 on Arduino) → Voicemeeter strip index.
# Voicemeeter Banana strip layout:
#   0 = Hardware Input 1
#   1 = Hardware Input 2
#   2 = Hardware Input 3
#   3 = Virtual Input 1  (VAIO)
#   4 = Virtual Input 2  (VAIO AUX)
POT_TO_STRIP: dict[int, int] = {
    0: 0,   # A0 → Hardware Input 1
    1: 1,   # A1 → Hardware Input 2
    2: 3,   # A2 → Virtual Input 1
    3: 4,   # A3 → Virtual Input 2
}

# ================================================================
#  BUTTON MAP
# ================================================================
BUTTON_ACTIONS: dict[int, str] = {
    0:  "ctrl+alt+m",
    1:  "ctrl+shift+s",
    2:  "alt+tab",
    3:  "ctrl+c",
    4:  "ctrl+v",
    5:  "volume up",
    6:  "volume down",
    7:  "play/pause media",
    8:  "next track",
    9:  "previous track",
    10: "win+d",
    11: "win+e",
    12: "win+r",
    13: "ctrl+z",
    14: "ctrl+y",
    15: "f1",
    16: "f2",
    17: "f3",
    18: "f4",
    19: "f5",
}

# ================================================================
#  HELPERS
# ================================================================
def raw_to_gain(raw: int) -> int:
    """
    Map ADC value (0–1023) to an integer dB gain (-60 to +12).
    Clamped so extreme pot positions never go out of range.
    """
    return max(GAIN_MIN, min(GAIN_MAX, round((raw / 1023.0) * GAIN_RANGE + GAIN_MIN)))


def find_port() -> str:
    """Return SERIAL_PORT, or the first detected Arduino/CH340/CP210 if that fails."""
    for p in serial.tools.list_ports.comports():
        desc = (p.description or "").lower()
        mfr  = (p.manufacturer or "").lower()
        if any(k in desc or k in mfr for k in ("arduino", "ch340", "cp210", "ftdi")):
            return p.device
    return SERIAL_PORT


# ================================================================
class StreamDeck:
    def __init__(self) -> None:
        self._ser: serial.Serial | None = None
        self._vm = voicemeeterlib.api("banana")
        self._vm.login()

        # Last integer-dB value sent to VM per channel.
        # None = not yet synced; forces a VM write on the first message.
        self._last_sent: list[int | None] = [None] * 4

        self._line_buf = bytearray()
        self._running  = False

    # ------------------------------------------------------------------
    def _open_serial(self, port: str) -> bool:
        try:
            if self._ser and self._ser.is_open:
                self._ser.close()
            s = serial.Serial()
            s.port     = port
            s.baudrate = BAUD_RATE
            s.timeout  = 0        # non-blocking — we poll in_waiting ourselves
            s.dsrdtr   = False    # do NOT toggle DTR → prevents Arduino auto-reset on open
            s.rtscts   = False
            s.open()
            # Give the USB stack and Arduino sketch time to settle,
            # then flush any bootloader noise from the buffer.
            time.sleep(2.0)
            s.reset_input_buffer()
            self._line_buf.clear()
            self._ser = s
            return True
        except (serial.SerialException, OSError) as exc:
            print(f"  Cannot open {port}: {exc}")
            return False

    def _wait_for_connection(self) -> None:
        """Block until the serial port opens. Retries silently forever."""
        port = SERIAL_PORT
        while True:
            if self._open_serial(port):
                # Reset sync state so the Arduino's startup dump forces a VM update
                self._last_sent = [None] * 4
                print(f"Connected on {self._ser.port}.")
                return
            # Try auto-detection as fallback
            detected = find_port()
            if detected != port:
                port = detected
            else:
                time.sleep(2.0)

    # ------------------------------------------------------------------
    def _handle_pot(self, pot_id: int, raw: int) -> None:
        strip_id = POT_TO_STRIP.get(pot_id)
        if strip_id is None:
            return
        gain = raw_to_gain(raw)
        if gain == self._last_sent[pot_id]:
            return   # same 1 dB bucket — nothing to do
        try:
            self._vm.strip[strip_id].gain = float(gain)
            self._last_sent[pot_id] = gain   # only mark sent after confirmed success
            print(f"  P{pot_id} -> strip[{strip_id}] = {gain:+d} dB")
        except Exception as exc:
            print(f"  VM error: strip[{strip_id}] gain={gain}: {exc}")

    def _handle_button(self, btn_id: int) -> None:
        action = BUTTON_ACTIONS.get(btn_id)
        if action:
            keyboard.send(action)

    def _process_line(self, line: str) -> None:
        if len(line) < 3:
            return
        try:
            colon = line.index(":")
            kind  = line[0]
            id_   = int(line[1:colon])
            val   = int(line[colon + 1:])
        except (ValueError, IndexError):
            return
        if kind == "P":
            self._handle_pot(id_, val)
        elif kind == "B" and val == 1:
            self._handle_button(id_)

    def _read_available(self) -> None:
        """
        Drain bytes that are already in the OS buffer — never blocks.
        Avoids holding a pending Windows async read that can fail on device reset.
        """
        waiting = self._ser.in_waiting
        if not waiting:
            return
        self._line_buf.extend(self._ser.read(waiting))
        while b"\n" in self._line_buf:
            idx  = self._line_buf.index(b"\n")
            line = self._line_buf[:idx].decode("ascii", errors="ignore").strip()
            del self._line_buf[:idx + 1]
            self._process_line(line)

    # ------------------------------------------------------------------
    def run(self) -> None:
        print(f"Waiting for Arduino on {SERIAL_PORT}…")
        self._wait_for_connection()
        print("Voicemeeter Banana ready.  Press Ctrl+C to exit.\n")

        self._running = True
        try:
            while self._running:
                try:
                    self._read_available()
                except (serial.SerialException, OSError) as exc:
                    print(f"Serial error: {exc}\nWaiting for Arduino…")
                    self._wait_for_connection()
                    print("Resumed.")
                    continue

                time.sleep(0.005)   # ~200 Hz loop; prevents CPU spin when pot is idle

        except KeyboardInterrupt:
            print("\nShutting down…")
        finally:
            self._running = False
            self._vm.logout()
            if self._ser and self._ser.is_open:
                self._ser.close()
            print("Disconnected cleanly.")


# ================================================================
if __name__ == "__main__":
    StreamDeck().run()
