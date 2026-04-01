import serial
import keyboard
import voicemeeterlib

# ==== CONNECT TO ARDUINO ====
ser = serial.Serial('COM3', 115200)  # CHANGE THIS

# ==== CONNECT TO VOICEMEETER ====
vm = voicemeeterlib.api("banana")
vm.login()

print("Connected!")

try:
    while True:
        line = ser.readline().decode().strip()

        # print("Received:", line)

        # ===== POTENTIOMETERS =====
        if line.startswith("P"):
            pot, val = line.split(":")
            pot_id = int(pot[1])
            value = int(val)

            volume = value / 1023

            # Map pots to Voicemeeter strips
            if pot_id == 0:
                vm.strip[0].gain = volume * 12 - 60
            elif pot_id == 1:
                vm.strip[1].gain = volume * 12 - 60
            elif pot_id == 2:
                vm.strip[2].gain = volume * 12 - 60
            elif pot_id == 3:
                vm.strip[3].gain = volume * 12 - 60

        # ===== BUTTONS =====
        elif line.startswith("B"):
            btn, state = line.split(":")
            btn_id = int(btn[1:])

            print(f"Button {btn_id} pressed")

            # === BUTTON ACTIONS ===
            if btn_id == 0:
                keyboard.send("ctrl+alt+m")   # Example: mute mic
            elif btn_id == 1:
                keyboard.send("ctrl+shift+s")
            elif btn_id == 2:
                keyboard.send("alt+tab")
            elif btn_id == 3:
                keyboard.send("ctrl+c")
            elif btn_id == 4:
                keyboard.send("ctrl+v")
            elif btn_id == 5:
                keyboard.send("volume up")
            elif btn_id == 6:
                keyboard.send("volume down")
            elif btn_id == 7:
                keyboard.send("play/pause media")
            elif btn_id == 8:
                keyboard.send("next track")
            elif btn_id == 9:
                keyboard.send("previous track")
            elif btn_id == 10:
                keyboard.send("win+d")  # show desktop
            elif btn_id == 11:
                keyboard.send("win+e")  # file explorer
            elif btn_id == 12:
                keyboard.send("win+r")  # run dialog
            elif btn_id == 13:
                keyboard.send("ctrl+z")
            elif btn_id == 14:
                keyboard.send("ctrl+y")
            elif btn_id == 15:
                keyboard.send("f1")
            elif btn_id == 16:
                keyboard.send("f2")
            elif btn_id == 17:
                keyboard.send("f3")
            elif btn_id == 18:
                keyboard.send("f4")
            elif btn_id == 19:
                keyboard.send("f5")
finally:
    vm.logout()