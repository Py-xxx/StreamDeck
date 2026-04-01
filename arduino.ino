// === POTENTIOMETERS ===
const int potPins[4] = {A0, A1, A2, A3};
int lastPotValues[4];

// === BUTTON MATRIX ===
const int rowPins[6] = {2, 3, 4, 5, 6, 7};        // adjust if needed
const int colPins[4] = {8, 9, 10, 11};    // adjust if needed

const int numRows = 6;
const int numCols = 4;

bool lastState[4][5];

void setup() {
  Serial.begin(115200);

  // === POT SETUP ===
  for (int i = 0; i < 4; i++) {
    lastPotValues[i] = 0;
  }

  // === MATRIX SETUP ===
  // Rows = outputs
  for (int r = 0; r < numRows; r++) {
    pinMode(rowPins[r], OUTPUT);
    digitalWrite(rowPins[r], HIGH);
  }

  // Columns = inputs with pullups
  for (int c = 0; c < numCols; c++) {
    pinMode(colPins[c], INPUT_PULLUP);
  }
}

void loop() {
  // ===== POTENTIOMETERS =====
  for (int i = 0; i < 4; i++) {
    int val = analogRead(potPins[i]);

    if (abs(val - lastPotValues[i]) > 5) {
      Serial.print("P");
      Serial.print(i);
      Serial.print(":");
      Serial.println(val);
      lastPotValues[i] = val;
    }
  }

  // ===== BUTTON MATRIX =====
  for (int r = 0; r < numRows; r++) {

    // Activate one row
    digitalWrite(rowPins[r], LOW);

    for (int c = 0; c < numCols; c++) {
      bool pressed = (digitalRead(colPins[c]) == LOW);

      if (pressed != lastState[r][c]) {
        if (pressed) {
          int buttonID = r * numCols + c;

          Serial.print("B");
          Serial.print(buttonID);
          Serial.println(":1");
        }

        lastState[r][c] = pressed;
      }
    }

    // Deactivate row
    digitalWrite(rowPins[r], HIGH);
  }

  delay(10);
}