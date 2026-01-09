/**
 * SoundTouch AudioWorklet Processor
 */

let wasmInitialized = false;
let SoundTouchWasm = null;
let initSync = null;

class SoundTouchProcessor extends AudioWorkletProcessor {
    constructor(options) {
        super();
        
        this.sampleRate = options.processorOptions.sampleRate || 48000;
        this.tempo = options.processorOptions.tempo || 1.0;
        this.pitch = options.processorOptions.pitch || 0;
        this.wasmBytes = options.processorOptions.wasmBytes;
        this.soundtouchCode = options.processorOptions.soundtouchCode;
        
        this.initialized = false;
        this.soundtouch = null;
        this.buffer = [];
        
        this.init();
        
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
            if (!this.wasmBytes) {
                throw new Error('WASM bytes not provided');
            }
            if (!this.soundtouchCode) {
                throw new Error('soundtouchCode not provided');
            }
            
            if (!wasmInitialized) {
                // Execute soundtouch code to populate globalThis
                const fn = new Function(this.soundtouchCode);
                fn();
                
                SoundTouchWasm = globalThis.SoundTouchWasm;
                initSync = globalThis.initSync;
                
                if (!initSync || !SoundTouchWasm) {
                    throw new Error('SoundTouch exports not found in globalThis');
                }
                
                initSync({ module: this.wasmBytes });
                wasmInitialized = true;
            }
            
            this.soundtouch = new SoundTouchWasm(this.sampleRate, 2, true, false);
            this.soundtouch.setTempo(this.tempo);
            this.soundtouch.setPitchSemitones(this.pitch);
            
            this.initialized = true;
            this.port.postMessage({ type: 'initialized' });
        } catch (err) {
            console.error('Failed to initialize SoundTouch:', err);
            this.port.postMessage({ type: 'error', message: err.toString() });
        }
    }
    
    process(inputs, outputs, parameters) {
        const input = inputs[0];
        const output = outputs[0];
        
        if (!output || !output[0]) return true;
        
        if (!this.initialized || !this.soundtouch) {
            if (input && input[0]) {
                for (let ch = 0; ch < Math.min(input.length, output.length); ch++) {
                    if (input[ch] && output[ch]) output[ch].set(input[ch]);
                }
            }
            return true;
        }
        
        const channels = 2;
        const frameCount = output[0].length;
        
        try {
            if (input && input[0] && input[0].length > 0) {
                const inputInterleaved = new Float32Array(input[0].length * channels);
                for (let frame = 0; frame < input[0].length; frame++) {
                    inputInterleaved[frame * channels] = input[0][frame];
                    inputInterleaved[frame * channels + 1] = input[1] ? input[1][frame] : input[0][frame];
                }
                this.soundtouch.putSamples(inputInterleaved);
            }
            
            const tempBuffer = new Float32Array(frameCount * channels * 4);
            const receivedFrames = this.soundtouch.receiveSamples(tempBuffer);
            
            for (let i = 0; i < receivedFrames * channels; i++) {
                this.buffer.push(tempBuffer[i]);
            }
            
            const neededSamples = frameCount * channels;
            
            if (this.buffer.length >= neededSamples) {
                let idx = 0;
                for (let frame = 0; frame < frameCount; frame++) {
                    if (output[0]) output[0][frame] = this.buffer[idx++];
                    if (output[1]) output[1][frame] = this.buffer[idx++];
                }
                this.buffer.splice(0, neededSamples);
            } else {
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
                if (output[ch]) output[ch].fill(0);
            }
        }
        
        return true;
    }
}

registerProcessor('soundtouch-processor', SoundTouchProcessor);
