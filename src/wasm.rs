//! WASM bindings for SoundTouch
//!
//! This module provides JavaScript-compatible bindings for the SoundTouch library
//! when compiled to WebAssembly.

use wasm_bindgen::prelude::*;
use crate::soundtouch::SoundTouch;

/// Initialize panic hook for better error messages in the browser console
#[wasm_bindgen(start)]
pub fn init() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

/// JavaScript-facing SoundTouch wrapper
/// 
/// This wrapper provides a simplified interface for JavaScript,
/// matching the C++ WASM wrapper API.
#[wasm_bindgen]
pub struct SoundTouchWasm {
    inner: SoundTouch,
    channels: usize,
    sample_rate: u32,
}

#[wasm_bindgen]
impl SoundTouchWasm {
    /// Create a new SoundTouch instance
    /// 
    /// # Arguments
    /// * `sample_rate` - Sample rate in Hz (e.g., 44100, 48000)
    /// * `channels` - Number of channels (1 for mono, 2 for stereo)
    #[wasm_bindgen(constructor)]
    pub fn new(sample_rate: u32, channels: usize) -> Result<SoundTouchWasm, JsValue> {
        let mut inner = SoundTouch::new();
        
        inner.set_sample_rate(sample_rate);
        inner.set_channels(channels)
            .map_err(|e| JsValue::from_str(&format!("Failed to set channels: {}", e)))?;
        
        // Set default settings to match C++ wrapper
        inner.set_tempo(1.0);
        inner.set_pitch(1.0);
        inner.set_setting(crate::types::SettingId::UseQuickSeek, 1);
        inner.set_setting(crate::types::SettingId::UseAAFilter, 0);
        
        Ok(SoundTouchWasm {
            inner,
            channels,
            sample_rate,
        })
    }
    
    /// Set tempo (speed) adjustment
    /// 
    /// # Arguments
    /// * `tempo` - Tempo multiplier (1.0 = original speed, 0.5 = half speed, 2.0 = double speed)
    ///             Valid range: 0.25 - 4.0
    #[wasm_bindgen(js_name = setTempo)]
    pub fn set_tempo(&mut self, mut tempo: f64) {
        // Clamp to valid range
        if tempo < 0.25 {
            tempo = 0.25;
        }
        if tempo > 4.0 {
            tempo = 4.0;
        }
        self.inner.set_tempo(tempo);
    }
    
    /// Set pitch adjustment in semitones
    /// 
    /// # Arguments
    /// * `semitones` - Pitch shift in semitones (0 = no change, +12 = one octave up, -12 = one octave down)
    ///                Valid range: typically -12 to +12
    #[wasm_bindgen(js_name = setPitchSemitones)]
    pub fn set_pitch_semitones(&mut self, semitones: f64) {
        self.inner.set_pitch_semi_tones_float(semitones);
    }
    
    /// Set rate adjustment (affects both tempo and pitch)
    /// 
    /// # Arguments
    /// * `rate` - Rate multiplier (1.0 = original, 0.5 = slower and lower pitch, 2.0 = faster and higher pitch)
    #[wasm_bindgen(js_name = setRate)]
    pub fn set_rate(&mut self, rate: f64) {
        self.inner.set_rate(rate);
    }
    
    /// Put samples into the processing pipeline
    /// 
    /// # Arguments
    /// * `samples` - Float32Array of interleaved audio samples
    ///              For stereo: [L, R, L, R, ...]
    ///              For mono: [sample1, sample2, ...]
    /// 
    /// # Returns
    /// Number of frames processed (samples.length / channels)
    #[wasm_bindgen(js_name = putSamples)]
    pub fn put_samples(&mut self, samples: &[f32]) -> Result<usize, JsValue> {
        let num_frames = samples.len() / self.channels;
        
        if samples.len() % self.channels != 0 {
            return Err(JsValue::from_str("Sample buffer length must be a multiple of channel count"));
        }
        
        self.inner.put_samples(samples, num_frames)
            .map_err(|e| JsValue::from_str(&format!("Failed to put samples: {}", e)))?;
        
        Ok(num_frames)
    }
    
    /// Receive processed samples
    /// 
    /// # Arguments
    /// * `output` - Float32Array to receive processed samples (must be pre-allocated)
    /// 
    /// # Returns
    /// Number of frames written (output.length / channels)
    #[wasm_bindgen(js_name = receiveSamples)]
    pub fn receive_samples(&mut self, output: &mut [f32]) -> usize {
        let max_frames = output.len() / self.channels;
        let received_frames = self.inner.receive_samples(output, max_frames);
        received_frames
    }
    
    /// Get number of available output samples (in frames)
    #[wasm_bindgen(js_name = numSamples)]
    pub fn num_samples(&self) -> usize {
        self.inner.num_samples()
    }
    
    /// Check if there are unprocessed samples in the input buffer
    #[wasm_bindgen(js_name = numUnprocessedSamples)]
    pub fn num_unprocessed_samples(&self) -> usize {
        self.inner.num_unprocessed_samples()
    }
    
    /// Flush the processing pipeline
    /// Call this at the end of processing to get all remaining samples
    #[wasm_bindgen(js_name = flush)]
    pub fn flush(&mut self) {
        self.inner.flush();
    }
    
    /// Clear all internal buffers
    #[wasm_bindgen(js_name = clear)]
    pub fn clear(&mut self) {
        self.inner.clear();
    }
    
    /// Check if output buffer is empty
    #[wasm_bindgen(js_name = isEmpty)]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
    
    /// Get the number of channels
    #[wasm_bindgen(js_name = getChannels)]
    pub fn get_channels(&self) -> usize {
        self.channels
    }
    
    /// Get the sample rate
    #[wasm_bindgen(js_name = getSampleRate)]
    pub fn get_sample_rate(&self) -> u32 {
        self.sample_rate
    }
    
    /// Process interleaved audio in one call (convenience method)
    /// 
    /// This combines putSamples + receiveSamples in one operation.
    /// Note: The output buffer size should be large enough to hold the processed data.
    /// For tempo < 1.0, output will be larger than input.
    /// For tempo > 1.0, output will be smaller than input.
    /// 
    /// # Arguments
    /// * `input` - Float32Array of interleaved input samples
    /// * `output` - Float32Array to receive processed samples (must be pre-allocated)
    /// 
    /// # Returns
    /// Number of output frames written
    #[wasm_bindgen(js_name = processInterleaved)]
    pub fn process_interleaved(&mut self, input: &[f32], output: &mut [f32]) -> Result<usize, JsValue> {
        // Put input samples
        self.put_samples(input)?;
        
        // Receive output samples
        let output_frames = self.receive_samples(output);
        
        Ok(output_frames)
    }
}

/// Get SoundTouch version string
#[wasm_bindgen(js_name = getVersion)]
pub fn get_version() -> String {
    SoundTouch::version().to_string()
}

/// Get SoundTouch version ID
#[wasm_bindgen(js_name = getVersionId)]
pub fn get_version_id() -> u32 {
    SoundTouch::version_id()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_wasm_wrapper_creation() {
        let st = SoundTouchWasm::new(44100, 2);
        assert!(st.is_ok());
        let st = st.unwrap();
        assert_eq!(st.get_channels(), 2);
        assert_eq!(st.get_sample_rate(), 44100);
    }
    
    #[test]
    fn test_wasm_wrapper_settings() {
        let mut st = SoundTouchWasm::new(44100, 2).unwrap();
        st.set_tempo(1.5);
        st.set_pitch_semitones(2.0);
        st.set_rate(1.0);
    }
    
    #[test]
    fn test_wasm_wrapper_processing() {
        let mut st = SoundTouchWasm::new(44100, 2).unwrap();
        
        // Generate 1 second of silence
        let input: Vec<f32> = vec![0.0; 44100 * 2];
        let mut output: Vec<f32> = vec![0.0; 44100 * 2];
        
        st.set_tempo(1.0);
        let result = st.put_samples(&input);
        assert!(result.is_ok());
        
        let received = st.receive_samples(&mut output);
        assert!(received > 0);
    }
}

