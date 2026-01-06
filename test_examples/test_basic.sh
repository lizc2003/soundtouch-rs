#!/bin/bash
# Basic SoundTouch test script (Rust version)
# Tests basic pitch and tempo modifications

SOUND_DIR=./
OUT_DIR=./test_output/basic
TEST_NAME=sample
SS=../target/release/soundstretch

# Create output directory
mkdir -p $OUT_DIR

echo "=== Basic SoundTouch Tests (Rust) ==="
echo ""

# Check if soundstretch-compat exists
if [ ! -f "$SS" ]; then
    echo "Error: soundstretch-compat not found at $SS"
    echo "Please build it first:"
    echo "  cd .."
    echo "  cargo build --release --bin soundstretch"
    exit 1
fi

# Test 1: Pitch shift down by 3 semitones
echo "Test 1: Pitch -3 semitones"
$SS $SOUND_DIR/$TEST_NAME.wav $OUT_DIR/${TEST_NAME}_pitch-3.wav -pitch=-3
echo ""

# Test 2: Pitch shift up by 3 semitones
echo "Test 2: Pitch +3 semitones"
$SS $SOUND_DIR/$TEST_NAME.wav $OUT_DIR/${TEST_NAME}_pitch+3.wav -pitch=3
echo ""

# Test 3: Tempo change +120%
echo "Test 3: Tempo +120% (2.2x faster)"
$SS $SOUND_DIR/$TEST_NAME.wav $OUT_DIR/${TEST_NAME}_tempo+120.wav -tempo=120
echo ""

# Test 4: Slow down by 20%
echo "Test 4: Tempo -20%"
$SS $SOUND_DIR/$TEST_NAME.wav $OUT_DIR/${TEST_NAME}_tempo-20.wav -tempo=-20
echo ""

# Test 5: Speed up by 20%
echo "Test 5: Tempo +20%"
$SS $SOUND_DIR/$TEST_NAME.wav $OUT_DIR/${TEST_NAME}_tempo+20.wav -tempo=20
echo ""

# Test 6: Combined pitch and tempo
echo "Test 6: Pitch +2, Tempo +10%"
$SS $SOUND_DIR/$TEST_NAME.wav $OUT_DIR/${TEST_NAME}_combined.wav -pitch=2 -tempo=10
echo ""

# Test 7: BPM detection
echo "Test 7: BPM Detection"
$SS $SOUND_DIR/$TEST_NAME.wav -bpm
echo ""

echo "=== Tests completed. Output files in $OUT_DIR ==="
