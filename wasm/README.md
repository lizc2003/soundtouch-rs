# SoundTouch WASM

WebAssembly bindings for the SoundTouch audio processing library.

## Building

### Prerequisites

Install rustup:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Install wasm-pack:

```bash
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
```

### Build

```bash
# Build for web (generates ES modules)
./wasm/build.sh web

# Build for Node.js
./wasm/build.sh nodejs

# Build for bundlers (webpack, rollup, etc.)
./wasm/build.sh bundler
```

## Usage

### JavaScript/TypeScript (Web)

```javascript
import init, { SoundTouchWasm } from './pkg-web/soundtouch.js';

async function processAudio() {
  // Initialize the WASM module
  await init();
  
  // Create a SoundTouch instance
  const st = new SoundTouchWasm(44100, 2); // 44.1kHz, stereo
  
  // Set processing parameters
  st.setTempo(1.5);           // 1.5x speed (50% faster)
  st.setPitchSemitones(2.0);  // +2 semitones (whole tone up)
  
  // Process audio (Float32Array, interleaved samples)
  const inputSamples = new Float32Array(8192); // Your input audio
  const outputSamples = new Float32Array(16384); // Output buffer
  
  // Put samples for processing
  st.putSamples(inputSamples);
  
  // Receive processed samples
  const numOutputFrames = st.receiveSamples(outputSamples);
  console.log(`Processed ${numOutputFrames} frames`);
  
  // Or use the convenience method
  const numFrames = st.processInterleaved(inputSamples, outputSamples);
  
  // Flush remaining samples at the end
  st.flush();
  const remaining = st.receiveSamples(outputSamples);
  
  // Clear buffers for new processing
  st.clear();
}
```

### Node.js

```javascript
const { SoundTouchWasm } = require('./pkg-nodejs/soundtouch.js');

const st = new SoundTouchWasm(48000, 2); // 48kHz, stereo
st.setTempo(0.8); // 0.8x speed (20% slower)
st.setPitchSemitones(-3); // -3 semitones (minor third down)

// Process audio...
```

### Web Audio API Integration

```javascript
import init, { SoundTouchWasm } from './pkg-web/soundtouch.js';

class SoundTouchNode extends AudioWorkletProcessor {
  constructor() {
    super();
    this.soundtouch = null;
    this.initialized = false;
    
    // Initialize in the constructor
    this.init();
  }
  
  async init() {
    await init();
    this.soundtouch = new SoundTouchWasm(sampleRate, 2);
    this.soundtouch.setTempo(1.0);
    this.initialized = true;
  }
  
  process(inputs, outputs, parameters) {
    if (!this.initialized || !inputs[0] || !inputs[0][0]) {
      return true;
    }
    
    const input = inputs[0];
    const output = outputs[0];
    
    // Convert to interleaved format
    const channels = input.length;
    const frameCount = input[0].length;
    const interleaved = new Float32Array(frameCount * channels);
    
    for (let frame = 0; frame < frameCount; frame++) {
      for (let ch = 0; ch < channels; ch++) {
        interleaved[frame * channels + ch] = input[ch][frame];
      }
    }
    
    // Process with SoundTouch
    this.soundtouch.putSamples(interleaved);
    
    const outputInterleaved = new Float32Array(frameCount * channels * 2);
    const outFrames = this.soundtouch.receiveSamples(outputInterleaved);
    
    // Convert back to planar format
    for (let frame = 0; frame < outFrames; frame++) {
      for (let ch = 0; ch < channels; ch++) {
        output[ch][frame] = outputInterleaved[frame * channels + ch];
      }
    }
    
    return true;
  }
}

registerProcessor('soundtouch-processor', SoundTouchNode);
```

## API Reference

### Constructor

```typescript
new SoundTouchWasm(sampleRate: number, channels: number): SoundTouchWasm
```

Creates a new SoundTouch instance.

- `sampleRate`: Sample rate in Hz (e.g., 44100, 48000)
- `channels`: Number of channels (1 for mono, 2 for stereo)

### Methods

#### `setTempo(tempo: number): void`

Set tempo (speed) adjustment. Does not affect pitch.

- `tempo`: Tempo multiplier (0.25 - 4.0)
  - 1.0 = original speed
  - 0.5 = half speed
  - 2.0 = double speed

#### `setPitchSemitones(semitones: number): void`

Set pitch adjustment in semitones. Does not affect tempo.

- `semitones`: Pitch shift in semitones
  - 0 = no change
  - +12 = one octave up
  - -12 = one octave down

#### `setRate(rate: number): void`

Set rate adjustment (affects both tempo and pitch).

- `rate`: Rate multiplier
  - 1.0 = original
  - 0.5 = slower and lower pitch
  - 2.0 = faster and higher pitch

#### `putSamples(samples: Float32Array): number`

Put samples into the processing pipeline.

- `samples`: Interleaved audio samples (for stereo: [L, R, L, R, ...])
- Returns: Number of frames processed

#### `receiveSamples(output: Float32Array): number`

Receive processed samples.

- `output`: Pre-allocated Float32Array to receive samples
- Returns: Number of frames written

#### `processInterleaved(input: Float32Array, output: Float32Array): number`

Process audio in one call (combines putSamples + receiveSamples).

- `input`: Input audio samples
- `output`: Pre-allocated output buffer
- Returns: Number of output frames written

#### `flush(): void`

Flush the processing pipeline to get all remaining samples.
Call this at the end of processing.

#### `clear(): void`

Clear all internal buffers.

#### `numSamples(): number`

Get number of available output samples (in frames).

#### `isEmpty(): boolean`

Check if output buffer is empty.

#### `getChannels(): number`

Get the number of channels.

#### `getSampleRate(): number`

Get the sample rate.

### Static Functions

```typescript
getVersionId(): number
```

## Examples

See the `examples/` directory for complete examples:

#### Using in the Browser

1. Start a local server:
```bash
cd wasm/examples
python3 -m http.server 8000
```

2. Open in the browser:
```
http://localhost:8000/simple.html
```

#### Using in Node.js

1. Build the Node.js version:
```bash
./wasm/build.sh nodejs
```

2. Run the example:
```bash
cd wasm/examples
node node-example.js
```

This will generate several test WAV files, demonstrating different audio processing effects.



## Performance Tips

1. **Buffer Sizing**: Use appropriate buffer sizes (typically 4096-8192 samples)
2. **Reuse Buffers**: Pre-allocate and reuse Float32Arrays to avoid GC pressure
3. **Batch Processing**: Process audio in chunks rather than sample-by-sample
4. **Optimize Build**: Use `--release` flag and enable LTO for production builds
