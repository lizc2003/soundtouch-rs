#!/bin/bash
# Advanced SoundTouch tests (Rust version)
# Tests various parameter combinations and edge cases

SOUND_DIR=./
OUT_DIR=./test_output/advanced
TEST_FILE=sample.wav
SS=../target/release/soundstretch

# Create output directory
mkdir -p $OUT_DIR

echo "=== Advanced SoundTouch Tests (Rust) ==="
echo ""

# Check if soundstretch exists
if [ ! -f "$SS" ]; then
    echo "Error: soundstretch not found"
    echo "Please build: cargo build --release --bin soundstretch"
    exit 1
fi

# Test extreme pitch shifts
echo "Test 1: Extreme pitch shifts"
echo "  - Pitch -12 semitones (one octave down)"
$SS $SOUND_DIR/$TEST_FILE $OUT_DIR/pitch_down_octave.wav -pitch=-12

echo "  - Pitch +12 semitones (one octave up)"
$SS $SOUND_DIR/$TEST_FILE $OUT_DIR/pitch_up_octave.wav -pitch=12

echo "  - Pitch -6 semitones"
$SS $SOUND_DIR/$TEST_FILE $OUT_DIR/pitch_down_6.wav -pitch=-6

echo "  - Pitch +6 semitones"
$SS $SOUND_DIR/$TEST_FILE $OUT_DIR/pitch_up_6.wav -pitch=6
echo ""

# Test extreme tempo changes
echo "Test 2: Extreme tempo changes"
echo "  - Tempo -50% (half speed)"
$SS $SOUND_DIR/$TEST_FILE $OUT_DIR/tempo_half.wav -tempo=-50

echo "  - Tempo +100% (double speed)"
$SS $SOUND_DIR/$TEST_FILE $OUT_DIR/tempo_double.wav -tempo=100

echo "  - Tempo -30%"
$SS $SOUND_DIR/$TEST_FILE $OUT_DIR/tempo_slow.wav -tempo=-30

echo "  - Tempo +50%"
$SS $SOUND_DIR/$TEST_FILE $OUT_DIR/tempo_fast.wav -tempo=50
echo ""

# Test rate changes
echo "Test 3: Rate changes (pitch + tempo together)"
echo "  - Rate -25%"
$SS $SOUND_DIR/$TEST_FILE $OUT_DIR/rate_slow.wav -rate=-25

echo "  - Rate +25%"
$SS $SOUND_DIR/$TEST_FILE $OUT_DIR/rate_fast.wav -rate=25
echo ""

# Test complex combinations
echo "Test 4: Complex parameter combinations"
echo "  - Pitch +5, Tempo -10"
$SS $SOUND_DIR/$TEST_FILE $OUT_DIR/combo1.wav -pitch=5 -tempo=-10

echo "  - Pitch -4, Tempo +15"
$SS $SOUND_DIR/$TEST_FILE $OUT_DIR/combo2.wav -pitch=-4 -tempo=15

echo "  - Pitch +7, Tempo +20"
$SS $SOUND_DIR/$TEST_FILE $OUT_DIR/combo3.wav -pitch=7 -tempo=20
echo ""

# Test with BPM detection
echo "Test 5: BPM detection with modifications"
echo "  - Original BPM"
$SS $SOUND_DIR/$TEST_FILE -bpm

echo "  - Pitch -3 with BPM"
$SS $SOUND_DIR/$TEST_FILE $OUT_DIR/pitch_with_bpm.wav -pitch=-3 -bpm
echo ""

echo "=== Tests completed. Output files in $OUT_DIR ==="

