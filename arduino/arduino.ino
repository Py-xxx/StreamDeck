// ============================================================
//  StreamDeck – Arduino firmware
//  Potentiometers + button matrix → Serial (115200 baud)
// ============================================================

// === POTENTIOMETERS ===
static const uint8_t  POT_COUNT     = 4;
static const uint8_t  potPins[POT_COUNT] = {A0, A1, A2, A3};
static const uint8_t  OVERSAMPLE    = 8;    // reads averaged; keeps 0–1023 range, kills noise
static const uint8_t  POT_THRESHOLD = 3;    // min change to transmit (post-oversampling)
static int16_t lastPotValues[POT_COUNT];

// === BUTTON MATRIX ===
static const uint8_t NUM_ROWS = 6;
static const uint8_t NUM_COLS = 4;
static const uint8_t rowPins[NUM_ROWS] = {2, 3, 4, 5, 6, 7};
static const uint8_t colPins[NUM_COLS] = {8, 9, 10, 11};

static bool     lastState[NUM_ROWS][NUM_COLS];
static uint32_t debounceTimer[NUM_ROWS][NUM_COLS];
static const uint16_t DEBOUNCE_MS = 18;   // ms to wait after first edge

// Reusable serial message buffer
static char msgBuf[12];

// ---------------------------------------------------------------
//  ADC oversampling: sum N reads, right-shift back to 10-bit
// ---------------------------------------------------------------
static inline int16_t oversampledRead(uint8_t pin) {
  int32_t sum = 0;
  for (uint8_t i = 0; i < OVERSAMPLE; i++) {
    sum += analogRead(pin);
  }
  // OVERSAMPLE = 8 → >>3  (keeps result in 0–1023)
  return (int16_t)(sum >> 3);
}

// ---------------------------------------------------------------
void setup() {
  Serial.begin(115200);

  // Seed pot history so first loop doesn't send spurious updates
  for (uint8_t i = 0; i < POT_COUNT; i++) {
    lastPotValues[i] = oversampledRead(potPins[i]);
  }

  // Rows = driven outputs (active-LOW)
  for (uint8_t r = 0; r < NUM_ROWS; r++) {
    pinMode(rowPins[r], OUTPUT);
    digitalWrite(rowPins[r], HIGH);
  }

  // Columns = inputs with internal pull-ups
  for (uint8_t c = 0; c < NUM_COLS; c++) {
    pinMode(colPins[c], INPUT_PULLUP);
  }

  memset(lastState,     false, sizeof(lastState));
  memset(debounceTimer, 0,     sizeof(debounceTimer));
}

// ---------------------------------------------------------------
static void scanPots() {
  for (uint8_t i = 0; i < POT_COUNT; i++) {
    int16_t val = oversampledRead(potPins[i]);

    if (abs(val - lastPotValues[i]) > POT_THRESHOLD) {
      snprintf(msgBuf, sizeof(msgBuf), "P%u:%d", i, val);
      Serial.println(msgBuf);
      lastPotValues[i] = val;
    }
  }
}

// ---------------------------------------------------------------
static void scanButtons() {
  uint32_t now = millis();

  for (uint8_t r = 0; r < NUM_ROWS; r++) {
    digitalWrite(rowPins[r], LOW);
    delayMicroseconds(10);           // let GPIO settle before reading

    for (uint8_t c = 0; c < NUM_COLS; c++) {
      bool pressed = (digitalRead(colPins[c]) == LOW);

      if (pressed != lastState[r][c]) {
        if ((now - debounceTimer[r][c]) >= DEBOUNCE_MS) {
          debounceTimer[r][c] = now;
          lastState[r][c]     = pressed;

          if (pressed) {
            snprintf(msgBuf, sizeof(msgBuf), "B%u:1", (uint8_t)(r * NUM_COLS + c));
            Serial.println(msgBuf);
          }
        }
      }
    }

    digitalWrite(rowPins[r], HIGH);
  }
}

// ---------------------------------------------------------------
void loop() {
  scanPots();
  scanButtons();
  // No delay() — loop runs as fast as the hardware allows (~1 ms per cycle)
}
