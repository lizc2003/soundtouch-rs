/**
 * SoundTouch WASM - Node.js Example
 * 
 * This example demonstrates how to use SoundTouch in Node.js
 * to process audio files.
 */

const fs = require('fs');
const path = require('path');

// Import SoundTouch WASM (adjust path based on your build)
const { SoundTouchWasm, getVersion } = require('../pkg-nodejs/soundtouch.js');

/**
 * Generate a simple sine wave for testing
 */
function generateSineWave(sampleRate, frequency, duration, channels) {
    const numSamples = Math.floor(sampleRate * duration);
    const data = new Float32Array(numSamples * channels);
    
    for (let i = 0; i < numSamples; i++) {
        const t = i / sampleRate;
        const sample = Math.sin(2 * Math.PI * frequency * t) * 0.5;
        
        for (let ch = 0; ch < channels; ch++) {
            data[i * channels + ch] = sample;
        }
    }
    
    return { data, sampleRate, channels, numSamples };
}

/**
 * Process audio with SoundTouch
 */
function processAudio(inputAudio, tempo, pitchSemitones) {
    console.log(`\nProcessing audio:`);
    console.log(`  Sample rate: ${inputAudio.sampleRate} Hz`);
    console.log(`  Channels: ${inputAudio.channels}`);
    console.log(`  Duration: ${(inputAudio.numSamples / inputAudio.sampleRate).toFixed(2)}s`);
    console.log(`  Tempo: ${tempo}x`);
    console.log(`  Pitch: ${pitchSemitones > 0 ? '+' : ''}${pitchSemitones} semitones`);
    
    // Create SoundTouch instance
    const st = new SoundTouchWasm(inputAudio.sampleRate, inputAudio.channels);
    
    // Set parameters
    st.setTempo(tempo);
    st.setPitchSemitones(pitchSemitones);
    
    // Process in chunks
    const chunkSize = 8192; // frames
    const outputChunks = [];
    let totalOutputSamples = 0;
    
    console.log('\nProcessing chunks...');
    
    // Process all input
    for (let i = 0; i < inputAudio.data.length; i += chunkSize * inputAudio.channels) {
        const chunk = inputAudio.data.slice(i, i + chunkSize * inputAudio.channels);
        st.putSamples(chunk);
        
        // Receive output
        let outputChunk = new Float32Array(chunkSize * inputAudio.channels * 4);
        const received = st.receiveSamples(outputChunk);
        
        if (received > 0) {
            const actualData = outputChunk.slice(0, received * inputAudio.channels);
            outputChunks.push(actualData);
            totalOutputSamples += received * inputAudio.channels;
        }
    }
    
    // Flush remaining samples
    console.log('Flushing pipeline...');
    st.flush();
    
    let flushChunk = new Float32Array(chunkSize * inputAudio.channels * 4);
    const flushed = st.receiveSamples(flushChunk);
    
    if (flushed > 0) {
        const actualData = flushChunk.slice(0, flushed * inputAudio.channels);
        outputChunks.push(actualData);
        totalOutputSamples += flushed * inputAudio.channels;
    }
    
    // Concatenate all chunks
    const outputData = new Float32Array(totalOutputSamples);
    let offset = 0;
    for (const chunk of outputChunks) {
        outputData.set(chunk, offset);
        offset += chunk.length;
    }
    
    const outputNumSamples = totalOutputSamples / inputAudio.channels;
    const outputDuration = outputNumSamples / inputAudio.sampleRate;
    
    console.log(`\nProcessing complete:`);
    console.log(`  Output samples: ${outputNumSamples} frames`);
    console.log(`  Output duration: ${outputDuration.toFixed(2)}s`);
    console.log(`  Time ratio: ${(outputDuration / (inputAudio.numSamples / inputAudio.sampleRate)).toFixed(2)}x`);
    
    return {
        data: outputData,
        sampleRate: inputAudio.sampleRate,
        channels: inputAudio.channels,
        numSamples: outputNumSamples
    };
}

/**
 * Write WAV file (simple 32-bit float PCM)
 */
function writeWavFile(filename, audio) {
    const numChannels = audio.channels;
    const sampleRate = audio.sampleRate;
    const bitsPerSample = 32;
    const bytesPerSample = bitsPerSample / 8;
    const blockAlign = numChannels * bytesPerSample;
    const dataSize = audio.data.length * bytesPerSample;
    const fileSize = 44 + dataSize;
    
    const buffer = Buffer.alloc(fileSize);
    
    // RIFF header
    buffer.write('RIFF', 0);
    buffer.writeUInt32LE(fileSize - 8, 4);
    buffer.write('WAVE', 8);
    
    // fmt chunk
    buffer.write('fmt ', 12);
    buffer.writeUInt32LE(16, 16); // chunk size
    buffer.writeUInt16LE(3, 20); // format (3 = IEEE float)
    buffer.writeUInt16LE(numChannels, 22);
    buffer.writeUInt32LE(sampleRate, 24);
    buffer.writeUInt32LE(sampleRate * blockAlign, 28); // byte rate
    buffer.writeUInt16LE(blockAlign, 32);
    buffer.writeUInt16LE(bitsPerSample, 34);
    
    // data chunk
    buffer.write('data', 36);
    buffer.writeUInt32LE(dataSize, 40);
    
    // Write audio data
    for (let i = 0; i < audio.data.length; i++) {
        buffer.writeFloatLE(audio.data[i], 44 + i * bytesPerSample);
    }
    
    fs.writeFileSync(filename, buffer);
    console.log(`\nWrote ${filename} (${(fileSize / 1024).toFixed(2)} KB)`);
}

/**
 * Main function
 */
function main() {
    console.log('='.repeat(60));
    console.log('SoundTouch WASM - Node.js Example');
    console.log('='.repeat(60));
    console.log(`SoundTouch version: ${getVersion()}`);
    
    // Example 1: Speed up (faster tempo, same pitch)
    console.log('\n' + '-'.repeat(60));
    console.log('Example 1: Speed up (1.5x faster)');
    console.log('-'.repeat(60));
    
    const audio1 = generateSineWave(44100, 440, 2.0, 2); // 440 Hz (A4), 2 seconds, stereo
    const processed1 = processAudio(audio1, 1.5, 0);
    writeWavFile('output-faster.wav', processed1);
    
    // Example 2: Pitch shift (higher pitch, same tempo)
    console.log('\n' + '-'.repeat(60));
    console.log('Example 2: Pitch shift (+5 semitones)');
    console.log('-'.repeat(60));
    
    const audio2 = generateSineWave(44100, 440, 2.0, 2);
    const processed2 = processAudio(audio2, 1.0, 5);
    writeWavFile('output-pitched.wav', processed2);
    
    // Example 3: Combined (slower + lower pitch)
    console.log('\n' + '-'.repeat(60));
    console.log('Example 3: Slower + lower pitch (0.8x tempo, -3 semitones)');
    console.log('-'.repeat(60));
    
    const audio3 = generateSineWave(44100, 440, 2.0, 2);
    const processed3 = processAudio(audio3, 0.8, -3);
    writeWavFile('output-combined.wav', processed3);
    
    // Example 4: Mono audio
    console.log('\n' + '-'.repeat(60));
    console.log('Example 4: Mono audio (1.2x faster)');
    console.log('-'.repeat(60));
    
    const audio4 = generateSineWave(48000, 880, 1.5, 1); // 880 Hz (A5), 1.5 seconds, mono
    const processed4 = processAudio(audio4, 1.2, 0);
    writeWavFile('output-mono.wav', processed4);
    
    console.log('\n' + '='.repeat(60));
    console.log('All examples completed successfully!');
    console.log('='.repeat(60));
}

// Run if called directly
if (require.main === module) {
    try {
        main();
    } catch (err) {
        console.error('Error:', err);
        process.exit(1);
    }
}

module.exports = {
    processAudio,
    generateSineWave,
    writeWavFile
};

