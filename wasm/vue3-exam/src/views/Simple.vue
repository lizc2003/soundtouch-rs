<template>
  <div class="container">
    <h1>🎵 SoundTouch WASM Demo</h1>
    
    <div class="info">
      <strong>Instructions:</strong>
      <ol>
        <li>Load an audio file (WAV or MP3)</li>
        <li>Adjust tempo and pitch using the sliders</li>
        <li>Click "Process Audio" to apply effects</li>
        <li>Play the processed audio</li>
      </ol>
    </div>

    <div class="control-group">
      <label for="audioFile">Select Audio File:</label>
      <input type="file" id="audioFile" accept="audio/*" @change="handleFileSelect">
    </div>

    <canvas ref="waveformCanvas" id="waveform"></canvas>

    <div class="control-group">
      <label for="tempo">
        Tempo (Speed): <span class="value-display">{{ tempoValue }}x</span>
      </label>
      <input type="range" id="tempo" min="0.5" max="2.0" step="0.01" v-model="tempo" @input="updateTempoValue">
      <small>0.5x (slower) ← → 2.0x (faster)</small>
    </div>

    <div class="control-group">
      <label for="pitch">
        Pitch: <span class="value-display">{{ pitchValue }}</span>
      </label>
      <input type="range" id="pitch" min="-12" max="12" step="0.1" v-model="pitch" @input="updatePitchValue">
      <small>-12 semitones (lower) ← → +12 semitones (higher)</small>
    </div>

    <div class="control-group">
      <button @click="processAudio" :disabled="!originalBuffer || processing">
        {{ processing ? 'Processing...' : 'Process Audio' }}
      </button>
      <button @click="playOriginal" :disabled="!originalBuffer">Play Original</button>
      <button @click="playProcessed" :disabled="!processedBuffer">Play Processed</button>
      <button @click="stopAudio">Stop</button>
      <button @click="resetSettings">Reset Settings</button>
    </div>

    <div id="status">{{ statusText }}</div>
  </div>
</template>

<script setup>
import { ref, onMounted } from 'vue'
import init, { SoundTouchWasm, getVersionId } from 'soundtouch'

const tempo = ref(1.0)
const pitch = ref(0)
const tempoValue = ref('1.00')
const pitchValue = ref('0 semitones')
const statusText = ref('Ready. Please load an audio file.')
const processing = ref(false)

const waveformCanvas = ref(null)
let canvasCtx = null

let audioContext = null
let originalBuffer = null
let processedBuffer = null
let currentSource = null
let soundtouch = null

onMounted(async () => {
  canvasCtx = waveformCanvas.value.getContext('2d')
  await initWasm()
})

async function initWasm() {
  try {
    await init()
    const version = getVersionId()
    statusText.value = `SoundTouch WASM Version-${version} initialized successfully!`
  } catch (err) {
    statusText.value = `Error initializing WASM: ${err}`
    console.error(err)
  }
}

function initAudio() {
  if (!audioContext) {
    audioContext = new (window.AudioContext || window.webkitAudioContext)()
  }
}

function updateTempoValue() {
  tempoValue.value = parseFloat(tempo.value).toFixed(2)
}

function updatePitchValue() {
  const value = parseFloat(pitch.value)
  pitchValue.value = `${value > 0 ? '+' : ''}${value.toFixed(1)} semitones`
}

function resetSettings() {
  tempo.value = 1.0
  pitch.value = 0
  tempoValue.value = '1.00'
  pitchValue.value = '0 semitones'
}

async function handleFileSelect(e) {
  const file = e.target.files[0]
  if (!file) return

  initAudio()
  statusText.value = 'Loading audio file...'

  try {
    const arrayBuffer = await file.arrayBuffer()
    originalBuffer = await audioContext.decodeAudioData(arrayBuffer)
    
    statusText.value = `Loaded: ${file.name} (${originalBuffer.duration.toFixed(2)}s, ${originalBuffer.sampleRate}Hz, ${originalBuffer.numberOfChannels} channels)`
    
    drawWaveform(originalBuffer)
  } catch (err) {
    statusText.value = `Error loading file: ${err}`
    console.error(err)
  }
}

function drawWaveform(buffer) {
  const canvas = waveformCanvas.value
  const width = canvas.width = canvas.offsetWidth
  const height = canvas.height = canvas.offsetHeight
  const data = buffer.getChannelData(0)
  const step = Math.ceil(data.length / width)
  const amp = height / 2

  canvasCtx.fillStyle = '#f0f0f0'
  canvasCtx.fillRect(0, 0, width, height)
  canvasCtx.beginPath()
  canvasCtx.strokeStyle = '#667eea'
  canvasCtx.lineWidth = 2

  for (let i = 0; i < width; i++) {
    let min = 1.0
    let max = -1.0
    for (let j = 0; j < step; j++) {
      const datum = data[(i * step) + j]
      if (datum < min) min = datum
      if (datum > max) max = datum
    }
    canvasCtx.moveTo(i, (1 + min) * amp)
    canvasCtx.lineTo(i, (1 + max) * amp)
  }
  canvasCtx.stroke()
}

async function processAudio() {
  if (!originalBuffer) return

  statusText.value = 'Processing audio...'
  processing.value = true

  try {
    const tempoVal = parseFloat(tempo.value)
    const pitchVal = parseFloat(pitch.value)

    // Create SoundTouch instance
    soundtouch = new SoundTouchWasm(
      originalBuffer.sampleRate,
      originalBuffer.numberOfChannels,
      true, false
    )

    // Set parameters
    soundtouch.setTempo(tempoVal)
    soundtouch.setPitchSemitones(pitchVal)

    // Get audio data
    const channels = originalBuffer.numberOfChannels
    const length = originalBuffer.length
    
    // Convert to interleaved format
    const inputData = new Float32Array(length * channels)
    for (let ch = 0; ch < channels; ch++) {
      const channelData = originalBuffer.getChannelData(ch)
      for (let i = 0; i < length; i++) {
        inputData[i * channels + ch] = channelData[i]
      }
    }

    // Process in chunks
    const chunkSize = 8192
    const outputChunks = []
    
    for (let i = 0; i < inputData.length; i += chunkSize * channels) {
      const chunk = inputData.slice(i, i + chunkSize * channels)
      soundtouch.putSamples(chunk)
      
      // Receive output
      let outputChunk = new Float32Array(chunkSize * channels * 2)
      const received = soundtouch.receiveSamples(outputChunk)
      if (received > 0) {
        outputChunks.push(outputChunk.slice(0, received * channels))
      }
    }

    // Flush remaining samples
    soundtouch.flush()
    let flushChunk = new Float32Array(chunkSize * channels * 2)
    const flushed = soundtouch.receiveSamples(flushChunk)
    if (flushed > 0) {
      outputChunks.push(flushChunk.slice(0, flushed * channels))
    }

    // Concatenate output chunks
    const totalLength = outputChunks.reduce((sum, chunk) => sum + chunk.length, 0)
    const outputData = new Float32Array(totalLength)
    let offset = 0
    for (const chunk of outputChunks) {
      outputData.set(chunk, offset)
      offset += chunk.length
    }

    // Create output buffer
    const outputLength = outputData.length / channels
    processedBuffer = audioContext.createBuffer(
      channels,
      outputLength,
      originalBuffer.sampleRate
    )

    // Deinterleave
    for (let ch = 0; ch < channels; ch++) {
      const channelData = processedBuffer.getChannelData(ch)
      for (let i = 0; i < outputLength; i++) {
        channelData[i] = outputData[i * channels + ch]
      }
    }

    statusText.value = `Processed! Output duration: ${processedBuffer.duration.toFixed(2)}s`

  } catch (err) {
    statusText.value = `Error processing audio: ${err}`
    console.error(err)
  } finally {
    processing.value = false
  }
}

function playOriginal() {
  stopAudio()
  currentSource = audioContext.createBufferSource()
  currentSource.buffer = originalBuffer
  currentSource.connect(audioContext.destination)
  currentSource.start(0)
  statusText.value = 'Playing original...'
}

function playProcessed() {
  stopAudio()
  currentSource = audioContext.createBufferSource()
  currentSource.buffer = processedBuffer
  currentSource.connect(audioContext.destination)
  currentSource.start(0)
  statusText.value = 'Playing processed...'
}

function stopAudio() {
  if (currentSource) {
    try {
      currentSource.stop()
    } catch (e) {
      // Already stopped
    }
    currentSource = null
  }
}
</script>

<style scoped>
.container {
  background: white;
  border-radius: 10px;
  padding: 30px;
  box-shadow: 0 10px 40px rgba(0,0,0,0.2);
  max-width: 800px;
  margin: 0 auto;
}

h1 {
  color: #667eea;
  margin-top: 0;
}

.control-group {
  margin: 20px 0;
}

label {
  display: block;
  margin-bottom: 5px;
  font-weight: bold;
  color: #555;
}

input[type="range"] {
  width: 100%;
  margin: 10px 0;
}

.value-display {
  display: inline-block;
  min-width: 60px;
  text-align: right;
  font-weight: bold;
  color: #667eea;
}

button {
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  color: white;
  border: none;
  padding: 12px 30px;
  border-radius: 5px;
  cursor: pointer;
  font-size: 16px;
  margin: 10px 5px;
  transition: transform 0.2s, box-shadow 0.2s;
}

button:hover {
  transform: translateY(-2px);
  box-shadow: 0 5px 15px rgba(102, 126, 234, 0.4);
}

button:disabled {
  background: #ccc;
  cursor: not-allowed;
  transform: none;
}

#status {
  padding: 10px;
  margin: 10px 0;
  border-radius: 5px;
  background: #f0f0f0;
  font-family: monospace;
  font-size: 14px;
}

.info {
  background: #e7f3ff;
  padding: 15px;
  border-radius: 5px;
  margin: 20px 0;
  border-left: 4px solid #667eea;
}

#waveform {
  width: 100%;
  height: 150px;
  border: 1px solid #ddd;
  border-radius: 5px;
  margin: 20px 0;
}

small {
  color: #888;
  font-size: 12px;
}
</style>

