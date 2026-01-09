<template>
  <div class="container">
    <h1>🎵 SoundTouch AudioWorklet Demo</h1>
    
    <div class="warning">
      <strong>⚠️ Note:</strong> This example demonstrates real-time audio processing 
      using AudioWorklet with full SoundTouch WASM support:
      <ul>
        <li>Serve this page over HTTP/HTTPS (not file://)</li>
        <li><strong>Click "Start Processing"</strong> - AudioContext requires user interaction</li>
        <li>Real-time tempo and pitch adjustment using SoundTouch</li>
      </ul>
    </div>

    <div class="control">
      <label for="audioSource">Audio Source:</label>
      <select id="audioSource" v-model="audioSource" @change="handleSourceChange">
        <option value="file">Audio File</option>
        <option value="oscillator">Oscillator (440 Hz)</option>
        <option value="microphone">Microphone</option>
      </select>
      <input v-show="audioSource === 'file'" type="file" id="audioFile" accept="audio/*" @change="handleFileChange">
    </div>

    <div class="control">
      <label for="tempo">Tempo: <span>{{ tempoValue }}x</span></label>
      <input type="range" id="tempo" min="0.5" max="2.0" step="0.01" v-model="tempo" @input="updateTempo">
    </div>

    <div class="control">
      <label for="pitch">Pitch: <span>{{ pitchValue }}</span></label>
      <input type="range" id="pitch" min="-12" max="12" step="0.1" v-model="pitch" @input="updatePitch">
    </div>

    <div class="control">
      <button @click="startProcessing" :disabled="isProcessing">Start Processing</button>
      <button @click="stopProcessing" :disabled="!isProcessing">Stop Processing</button>
    </div>

    <div id="status">{{ statusText }}</div>
  </div>
</template>

<script setup>
import { ref, computed } from 'vue'
import processorUrl from '../utils/soundtouch-processor.js?url'
import wasmUrl from 'soundtouch/soundtouch_bg.wasm?url'
import soundtouchCode from 'soundtouch/soundtouch_worklet.js?raw'

const audioSource = ref('file')
const tempo = ref(1.0)
const pitch = ref(0)
const isProcessing = ref(false)
const statusText = ref('Ready. Select an audio source and click "Start Processing". (User interaction required for AudioContext - this is a browser security feature)')

let audioContext = null
let sourceNode = null
let soundtouchNode = null
let workletLoaded = false
let wasmBytes = null
let selectedFile = null

const tempoValue = computed(() => parseFloat(tempo.value).toFixed(2))
const pitchValue = computed(() => {
  const value = parseFloat(pitch.value)
  return `${value > 0 ? '+' : ''}${value.toFixed(1)} semitones`
})

function handleSourceChange() {
  // Reset file selection when changing source
  if (audioSource.value !== 'file') {
    selectedFile = null
  }
}

function handleFileChange(e) {
  selectedFile = e.target.files[0]
}

function updateTempo() {
  if (soundtouchNode) {
    soundtouchNode.port.postMessage({
      type: 'setTempo',
      value: parseFloat(tempo.value)
    })
  }
}

function updatePitch() {
  if (soundtouchNode) {
    soundtouchNode.port.postMessage({
      type: 'setPitch',
      value: parseFloat(pitch.value)
    })
  }
}

async function startProcessing() {
  try {
    // Initialize AudioContext if needed
    if (!audioContext) {
      statusText.value = 'Initializing AudioContext...'
      const AudioContextClass = window.AudioContext || window.webkitAudioContext
      
      if (!AudioContextClass) {
        throw new Error('AudioContext is not supported in your browser')
      }
      
      audioContext = new AudioContextClass()
      
      if (!audioContext.audioWorklet) {
        audioContext.close()
        audioContext = null
        throw new Error('AudioWorklet is not supported. Make sure you are accessing via HTTP/HTTPS (not file://)')
      }
      
      statusText.value = `AudioContext created (state: ${audioContext.state})`
    }
    
    // Resume AudioContext if suspended
    if (audioContext.state === 'suspended') {
      statusText.value = 'Activating AudioContext (browser autoplay policy)...'
      await audioContext.resume()
      statusText.value = 'AudioContext activated!'
    }
    
    // Load WASM bytes if not already loaded
    if (!wasmBytes) {
      statusText.value = 'Loading WASM module...'
      try {
        const response = await fetch(wasmUrl)
        if (!response.ok) {
          throw new Error(`Failed to fetch WASM: ${response.status}`)
        }
        wasmBytes = await response.arrayBuffer()
        statusText.value = 'WASM module loaded!'
      } catch (err) {
        throw new Error(`Failed to load WASM: ${err.message}`)
      }
    }
    
    // Load AudioWorklet processor
    if (!workletLoaded) {
      statusText.value = 'Loading AudioWorklet module...'
      try {
        await audioContext.audioWorklet.addModule(processorUrl)
        workletLoaded = true
        statusText.value = 'AudioWorklet loaded successfully!'
      } catch (err) {
        throw new Error(`Failed to load AudioWorklet module: ${err.message}. Make sure you're serving this page via HTTP (not file://) and that soundtouch-processor.js exists.`)
      }
    }

    // Create source based on selection
    const sourceType = audioSource.value
    
    if (sourceType === 'oscillator') {
      sourceNode = audioContext.createOscillator()
      sourceNode.frequency.value = 440
      sourceNode.type = 'sine'
    } else if (sourceType === 'microphone') {
      const stream = await navigator.mediaDevices.getUserMedia({ audio: true })
      sourceNode = audioContext.createMediaStreamSource(stream)
    } else if (sourceType === 'file') {
      if (!selectedFile) {
        statusText.value = 'Please select an audio file first!'
        return
      }
      const arrayBuffer = await selectedFile.arrayBuffer()
      const audioBuffer = await audioContext.decodeAudioData(arrayBuffer)
      sourceNode = audioContext.createBufferSource()
      sourceNode.buffer = audioBuffer
      sourceNode.loop = true
    }

    // Create SoundTouch worklet node
    soundtouchNode = new AudioWorkletNode(audioContext, 'soundtouch-processor', {
      numberOfInputs: 1,
      numberOfOutputs: 1,
      processorOptions: {
        sampleRate: audioContext.sampleRate,
        tempo: parseFloat(tempo.value),
        pitch: parseFloat(pitch.value),
        wasmBytes: wasmBytes,
        soundtouchCode: soundtouchCode
      }
    })

    // Connect: source -> soundtouch -> destination
    sourceNode.connect(soundtouchNode)
    soundtouchNode.connect(audioContext.destination)

    // Start source
    if (sourceNode.start) {
      sourceNode.start()
    }

    isProcessing.value = true
    statusText.value = `Processing audio from ${sourceType}...`

  } catch (err) {
    statusText.value = `Error: ${err.message}`
    console.error(err)
  }
}

function stopProcessing() {
  if (sourceNode) {
    if (sourceNode.stop) {
      sourceNode.stop()
    }
    sourceNode.disconnect()
    sourceNode = null
  }
  if (soundtouchNode) {
    soundtouchNode.disconnect()
    soundtouchNode = null
  }

  isProcessing.value = false
  statusText.value = 'Processing stopped.'
}
</script>

<style scoped>
.container {
  background: white;
  border-radius: 10px;
  padding: 30px;
  box-shadow: 0 2px 10px rgba(0,0,0,0.1);
  max-width: 800px;
  margin: 0 auto;
}

h1 {
  color: #333;
}

.control {
  margin: 20px 0;
}

label {
  display: block;
  margin-bottom: 5px;
  font-weight: bold;
}

input[type="range"] {
  width: 100%;
}

select {
  width: 100%;
  padding: 8px;
  border-radius: 5px;
  border: 1px solid #ddd;
  margin-bottom: 10px;
}

input[type="file"] {
  margin-top: 10px;
}

button {
  background: #4CAF50;
  color: white;
  border: none;
  padding: 10px 20px;
  border-radius: 5px;
  cursor: pointer;
  margin: 5px;
}

button:hover {
  background: #45a049;
}

button:disabled {
  background: #ccc;
  cursor: not-allowed;
}

#status {
  padding: 10px;
  background: #f0f0f0;
  border-radius: 5px;
  margin: 10px 0;
}

.warning {
  background: #fff3cd;
  border: 1px solid #ffc107;
  padding: 15px;
  border-radius: 5px;
  margin: 20px 0;
}

.warning ul {
  margin: 10px 0;
  padding-left: 20px;
}
</style>

