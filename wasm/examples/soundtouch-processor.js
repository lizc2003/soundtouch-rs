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

// Import WASM module (you'll need to adjust this based on your build)
// For AudioWorklet, you might need to use importScripts or dynamic import
// This is a placeholder - actual implementation may vary
importScripts('../pkg-web/soundtouch.js');

const { SoundTouchWasm } = wasm_bindgen;

class SoundTouchProcessor extends AudioWorkletProcessor {
    constructor(options) {
        super();
        
        this.sampleRate = options.processorOptions.sampleRate || 48000;
        this.tempo = options.processorOptions.tempo || 1.0;
        this.pitch = options.processorOptions.pitch || 0;
        
        this.initialized = false;
        this.soundtouch = null;
        
        // Buffering
        this.inputBuffer = [];
        this.outputBuffer = [];
        this.bufferSize = 4096;
        
        // Initialize asynchronously
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
    
    async init() {
        try {
            // Initialize WASM
            await wasm_bindgen('../pkg-web/soundtouch_bg.wasm');
            
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
            // Pass through unchanged until initialized
            if (inputs[0] && inputs[0][0]) {
                for (let channel = 0; channel < outputs[0].length; channel++) {
                    outputs[0][channel].set(inputs[0][channel]);
                }
            }
            return true;
        }
        
        const input = inputs[0];
        const output = outputs[0];
        
        if (!input || !input[0]) {
            return true;
        }
        
        const channels = Math.min(input.length, 2);
        const frameCount = input[0].length;
        
        try {
            // Convert to interleaved format
            const interleaved = new Float32Array(frameCount * channels);
            for (let frame = 0; frame < frameCount; frame++) {
                for (let ch = 0; ch < channels; ch++) {
                    const sample = input[ch] ? input[ch][frame] : 0;
                    interleaved[frame * channels + ch] = sample;
                }
            }
            
            // Put samples into SoundTouch
            this.soundtouch.putSamples(interleaved);
            
            // Receive processed samples
            const outputInterleaved = new Float32Array(frameCount * channels * 2);
            const receivedFrames = this.soundtouch.receiveSamples(outputInterleaved);
            
            // Convert back to planar format
            for (let ch = 0; ch < output.length; ch++) {
                const outputChannel = output[ch];
                for (let frame = 0; frame < Math.min(receivedFrames, frameCount); frame++) {
                    outputChannel[frame] = outputInterleaved[frame * channels + ch];
                }
                
                // Zero out remaining frames if any
                for (let frame = receivedFrames; frame < frameCount; frame++) {
                    outputChannel[frame] = 0;
                }
            }
            
        } catch (err) {
            console.error('Processing error:', err);
            // On error, pass through unchanged
            for (let channel = 0; channel < output.length; channel++) {
                output[channel].set(input[channel]);
            }
        }
        
        return true; // Keep processor alive
    }
}

registerProcessor('soundtouch-processor', SoundTouchProcessor);

