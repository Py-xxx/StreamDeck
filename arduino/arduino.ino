// ============================================================
//  StreamDeck – Arduino firmware
//  Potentiometers + button matrix → Serial (115200 baud)
// ============================================================

// === POTENTIOMETERS ===
static const uint8_t POT_COUNT     = 4;
static const uint8_t potPins[POT_COUNT] = {A0, A1, A2, A3};
static const uint8_t OVERSAMPLE    = 8;   // 8x averaging keeps 0–1023 range, kills ADC noise
static const uint8_t POT_THRESHOLD = 5;   // raw units — well below 1dB (~14 raw), stops idle chatter

static int16_t lastPotValues[POT_COUNT];

// === BUTTON MATRIX ===
static const uint8_t NUM_ROWS = 3;
static const uint8_t NUM_COLS = 4;
static const uint8_t rowPins[NUM_ROWS] = {2, 3, 4};
static const uint8_t colPins[NUM_COLS] = {5, 6, 7, 8};

static bool     lastState[NUM_ROWS][NUM_COLS];
static uint32_t debounceTimer[NUM_ROWS][NUM_COLS];
static const uint16_t DEBOUNCE_MS = 18;

static char msgBuf[12];

// ---------------------------------------------------------------
//  8× oversampling: sum 8 reads, right-shift 3 → stays in 0–1023
// ---------------------------------------------------------------
static inline int16_t oversampledRead(uint8_t pin) {
  int32_t sum = 0;
  for (uint8_t i = 0; i < OVERSAMPLE; i++) {
    sum += analogRead(pin);
  }
  return (int16_t)(sum >> 3);
}

// ---------------------------------------------------------------
void setup() {
  Serial.begin(115200);

  // Enable internal pull-ups on every analog pot pin.
  // Unconnected pins will read a stable ~1023 (no delta, no serial traffic).
  // When a real pot is wired in, its low impedance overrides the pull-up
  // and the pin reads normally — no code change needed when adding pots.
  for (uint8_t i = 0; i < POT_COUNT; i++) {
    pinMode(potPins[i], INPUT_PULLUP);
  }

  for (uint8_t i = 0; i < POT_COUNT; i++) {
    lastPotValues[i] = oversampledRead(potPins[i]);
  }

  for (uint8_t r = 0; r < NUM_ROWS; r++) {
    pinMode(rowPins[r], OUTPUT);
    digitalWrite(rowPins[r], HIGH);
  }
  for (uint8_t c = 0; c < NUM_COLS; c++) {
    pinMode(colPins[c], INPUT_PULLUP);
  }

  memset(lastState,     false, sizeof(lastState));
  memset(debounceTimer, 0,     sizeof(debounceTimer));

  // Broadcast current pot positions so the host always knows the
  // starting state — even after an Arduino reset mid-session.
  delay(100);
  for (uint8_t i = 0; i < POT_COUNT; i++) {
    snprintf(msgBuf, sizeof(msgBuf), "P%u:%d", i, lastPotValues[i]);
    Serial.println(msgBuf);
  }
}

// ---------------------------------------------------------------
static void scanPots() {
  for (uint8_t i = 0; i < POT_COUNT; i++) {
    int16_t val = constrain(oversampledRead(potPins[i]), 0, 1023);

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
    delayMicroseconds(10);

    for (uint8_t c = 0; c < NUM_COLS; c++) {
      bool pressed = (digitalRead(colPins[c]) == LOW);

      if (pressed != lastState[r][c]) {
        if ((now - debounceTimer[r][c]) >= DEBOUNCE_MS) {
          debounceTimer[r][c] = now;
          lastState[r][c]     = pressed;

          // Send both press (:1) and release (:0) so Python can detect holds
          snprintf(msgBuf, sizeof(msgBuf), "B%u:%d", (uint8_t)(r * NUM_COLS + c), pressed ? 1 : 0);
          Serial.println(msgBuf);
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
}
