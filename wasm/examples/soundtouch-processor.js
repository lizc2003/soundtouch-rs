/**
 * SoundTouch AudioWorklet Processor
 * 
 * This processor runs in the audio thread and applies SoundTouch processing
 * to incoming audio in real-time.
 * 
 * Note: This is a simplified example. For production use, you may need to:
 * - Handle latency compensation
 * - Implement better buffering
 * - Add error handling and recovery
 */

import { initSync, SoundTouchWasm } from './pkg-web/soundtouch.js';

// Global variable to track initialization
let wasmInitialized = false;

class SoundTouchProcessor extends AudioWorkletProcessor {
    constructor(options) {
        super();
        
        this.sampleRate = options.processorOptions.sampleRate || 48000;
        this.tempo = options.processorOptions.tempo || 1.0;
        this.pitch = options.processorOptions.pitch || 0;
        this.wasmBytes = options.processorOptions.wasmBytes;
        
        this.initialized = false;
        this.soundtouch = null;
        
        // Output buffer
        this.buffer = [];
        
        // Initialize synchronously with provided WASM bytes
        this.init();
        
        // Listen for parameter changes
        this.port.onmessage = (event) => {
            const { type, value } = event.data;
            
            if (!this.soundtouch) return;
            
            switch (type) {
                case 'setTempo':
                    this.tempo = value;
                    this.soundtouch.setTempo(value);
                    break;
                case 'setPitch':
                    this.pitch = value;
                    this.soundtouch.setPitchSemitones(value);
                    break;
            }
        };
    }
    
    init() {
        try {
            // Initialize WASM module if not already done
            if (!wasmInitialized && this.wasmBytes) {
                initSync({ module: this.wasmBytes });
                wasmInitialized = true;
            }
            
            // Create SoundTouch instance
            this.soundtouch = new SoundTouchWasm(this.sampleRate, 2); // stereo
            this.soundtouch.setTempo(this.tempo);
            this.soundtouch.setPitchSemitones(this.pitch);
            
            this.initialized = true;
            
            this.port.postMessage({ type: 'initialized' });
        } catch (err) {
            console.error('Failed to initialize SoundTouch:', err);
            this.port.postMessage({ 
                type: 'error', 
                message: err.toString() 
            });
        }
    }
    
    process(inputs, outputs, parameters) {
        if (!this.initialized || !this.soundtouch) {
            return true;
        }
        
        const input = inputs[0];
        const output = outputs[0];
        
        // Check if we have valid output
        if (!output || !output[0]) {
            return true;
        }
        
        // Determine actual number of output channels available
        const channels = 2;
        const actualOutputChannels = output.length;
        const frameCount = output[0].length;
        
        // If we don't have stereo output, just pass through silence
        if (actualOutputChannels < 2 || !output[1]) {
            for (let ch = 0; ch < actualOutputChannels; ch++) {
                if (output[ch]) {
                    output[ch].fill(0);
                }
            }
            return true;
        }
        
        try {
            // Feed input to SoundTouch
            if (input && input[0] && input[0].length > 0) {
                const inputInterleaved = new Float32Array(input[0].length * channels);
                for (let frame = 0; frame < input[0].length; frame++) {
                    inputInterleaved[frame * channels] = input[0][frame];
                    inputInterleaved[frame * channels + 1] = input[1] ? input[1][frame] : input[0][frame];
                }
                this.soundtouch.putSamples(inputInterleaved);
            }
            
            // Get processed samples and buffer them
            const tempBuffer = new Float32Array(frameCount * channels * 4);
            const receivedFrames = this.soundtouch.receiveSamples(tempBuffer);
            
            // Add to buffer
            for (let i = 0; i < receivedFrames * channels; i++) {
                this.buffer.push(tempBuffer[i]);
            }
            
            // Output from buffer
            const neededSamples = frameCount * channels;
            
            if (this.buffer.length >= neededSamples) {
                // We have enough samples
                let idx = 0;
                for (let frame = 0; frame < frameCount; frame++) {
                    if (output[0]) output[0][frame] = this.buffer[idx++];
                    if (output[1]) output[1][frame] = this.buffer[idx++];
                }
                // Remove used samples
                this.buffer.splice(0, neededSamples);
            } else {
                // Not enough - pass through input
                if (input && input[0]) {
                    for (let ch = 0; ch < output.length; ch++) {
                        output[ch].set(input[ch] || input[0]);
                    }
                } else {
                    for (let ch = 0; ch < output.length; ch++) {
                        output[ch].fill(0);
                    }
                }
            }
            
        } catch (err) {
            console.error('Processing error:', err);
            for (let ch = 0; ch < output.length; ch++) {
                if (output[ch]) {
                    output[ch].fill(0);
                }
            }
        }
        
        return true;
    }
}

registerProcessor('soundtouch-processor', SoundTouchProcessor);

