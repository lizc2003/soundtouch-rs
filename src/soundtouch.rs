//! Main SoundTouch class
//!
//! Main class for tempo/pitch/rate adjusting routines

use crate::rate_transposer::RateTransposer;
use crate::td_stretch::TDStretch;
use crate::types::{Sample, SettingId, float_equal};
use crate::error::{Result, SoundTouchError};

/// Main SoundTouch processor
pub struct SoundTouch {
    /// Rate transposer
    rate_transposer: RateTransposer,
    /// Time-domain stretcher
    td_stretch: TDStretch,
    /// Virtual rate parameter
    virtual_rate: f64,
    /// Virtual tempo parameter
    virtual_tempo: f64,
    /// Virtual pitch parameter
    virtual_pitch: f64,
    /// Effective rate value
    rate: f64,
    /// Effective tempo value
    tempo: f64,
    /// Number of channels
    channels: usize,
    /// Sample rate set flag
    sample_rate_set: bool,
    /// Expected output samples
    samples_expected_out: f64,
    /// Samples output counter
    samples_output: usize,
    /// Flag indicating if rate <= 1.0 (determines processing order)
    /// true: RateTransposer -> TDStretch, false: TDStretch -> RateTransposer
    rate_down: bool,
}

impl SoundTouch {
    /// Create a new SoundTouch instance
    pub fn new() -> Self {
        let mut st = SoundTouch {
            rate_transposer: RateTransposer::new(),
            td_stretch: TDStretch::new(),
            virtual_rate: 1.0,
            virtual_tempo: 1.0,
            virtual_pitch: 1.0,
            rate: 0.0,
            tempo: 0.0,
            channels: 0,
            sample_rate_set: false,
            samples_expected_out: 0.0,
            samples_output: 0,
            rate_down: true,  // Initial output = pTDStretch in C++
        };
        
        // Calculate initial rate and tempo (mimics C++ constructor)
        st.calc_effective_rate_and_tempo();
        st
    }

    /// Get version string
    pub fn version() -> &'static str {
        crate::VERSION
    }

    /// Get version ID
    pub fn version_id() -> u32 {
        crate::VERSION_ID
    }

    /// Set number of channels
    pub fn set_channels(&mut self, num_channels: usize) -> Result<()> {
        if num_channels == 0 || num_channels > crate::types::MAX_CHANNELS {
            return Err(SoundTouchError::InvalidChannels(num_channels as u32));
        }

        self.channels = num_channels;
        self.rate_transposer.set_channels(num_channels)?;
        self.td_stretch.set_channels(num_channels)?;
        Ok(())
    }

    /// Get number of channels
    pub fn num_channels(&self) -> usize {
        self.channels
    }

    /// Set sample rate
    pub fn set_sample_rate(&mut self, rate: u32) {
        if rate == 0 {
            return;
        }
        self.td_stretch.set_parameters(rate, -1, -1, -1);
        self.sample_rate_set = true;
    }

    /// Set rate (affects both tempo and pitch)
    pub fn set_rate(&mut self, new_rate: f64) {
        self.virtual_rate = new_rate;
        self.calc_effective_rate_and_tempo();
    }

    /// Set rate change in percentage (-50 .. +100 %)
    pub fn set_rate_change(&mut self, new_rate: f64) {
        self.virtual_rate = 1.0 + 0.01 * new_rate;
        self.calc_effective_rate_and_tempo();
    }

    /// Set tempo (affects tempo but not pitch)
    pub fn set_tempo(&mut self, new_tempo: f64) {
        self.virtual_tempo = new_tempo;
        self.calc_effective_rate_and_tempo();
    }

    /// Set tempo change in percentage (-50 .. +100 %)
    pub fn set_tempo_change(&mut self, new_tempo: f64) {
        self.virtual_tempo = 1.0 + 0.01 * new_tempo;
        self.calc_effective_rate_and_tempo();
    }

    /// Set pitch (affects pitch but not tempo)
    pub fn set_pitch(&mut self, new_pitch: f64) {
        self.virtual_pitch = new_pitch;
        self.calc_effective_rate_and_tempo();
    }

    /// Set pitch change in octaves compared to the original pitch (-1.00 .. +1.00)
    pub fn set_pitch_octaves(&mut self, new_pitch: f64) {
        self.virtual_pitch = (0.69314718056 * new_pitch).exp();
        self.calc_effective_rate_and_tempo();
    }

    /// Set pitch change in semi-tones compared to the original pitch (-12 .. +12)
    pub fn set_pitch_semi_tones(&mut self, new_pitch: i32) {
        self.set_pitch_octaves(new_pitch as f64 / 12.0);
    }

    /// Set pitch change in semi-tones (float)
    pub fn set_pitch_semi_tones_float(&mut self, new_pitch: f64) {
        self.set_pitch_octaves(new_pitch / 12.0);
    }

    /// Calculate effective rate and tempo from virtual parameters
    fn calc_effective_rate_and_tempo(&mut self) {
        let old_tempo = self.tempo;
        let old_rate = self.rate;

        self.tempo = self.virtual_tempo / self.virtual_pitch;
        self.rate = self.virtual_pitch * self.virtual_rate;

        if !float_equal(self.rate, old_rate) {
            self.rate_transposer.set_rate(self.rate);
        }
        if !float_equal(self.tempo, old_tempo) {
            self.td_stretch.set_tempo(self.tempo);
        }

        // Determine processing order based on rate
        // This mimics C++ SOUNDTOUCH_PREVENT_CLICK_AT_RATE_CROSSOVER logic
        let new_rate_down = self.rate <= 1.0;
        
        if new_rate_down != self.rate_down {
            // Processing order is changing, need to move samples between buffers
            if new_rate_down {
                // Switching to rate <= 1.0: RateTransposer -> TDStretch
                // Move samples from rate_transposer output to td_stretch output
                let td_output = self.td_stretch.get_output();
                let rate_output = self.rate_transposer.get_output();
                td_output.move_samples(rate_output);
            } else {
                // Switching to rate > 1.0: TDStretch -> RateTransposer  
                // Move samples from td_stretch output to rate_transposer output
                let td_output = self.td_stretch.get_output();
                let rate_output = self.rate_transposer.get_output();
                rate_output.move_samples(td_output);

                // move samples in tempo changer's input to pitch transposer's input
                let td_input = self.td_stretch.get_input();
                let num_samples = td_input.num_samples();
                self.rate_transposer.put_samples(td_input.ptr_begin(), num_samples);
                td_input.receive_samples_no_copy(num_samples);
            }
            self.rate_down = new_rate_down;
        }
    }
    
    /// Put samples into the processing pipeline (with error checking)
    pub fn put_samples(&mut self, samples: &[Sample], num_samples: usize) -> Result<()> {
        if !self.sample_rate_set {
            return Err(SoundTouchError::SampleRateNotSet);
        }
        if self.channels == 0 {
            return Err(SoundTouchError::ChannelsNotSet);
        }

        // Update expected output
        self.samples_expected_out += num_samples as f64 / (self.rate * self.tempo);

        if self.rate_down {
            // rate <= 1.0: RateTransposer -> TDStretch
            self.rate_transposer.put_samples(samples, num_samples);
            let output = self.rate_transposer.get_output();
            let num_samples = output.num_samples();
            self.td_stretch.put_samples(output.ptr_begin(), num_samples);
            output.receive_samples_no_copy(num_samples);
        } else {
            // rate > 1.0: TDStretch -> RateTransposer
            self.td_stretch.put_samples(samples, num_samples);
            let output = self.td_stretch.get_output();
            let num_samples = output.num_samples();
            self.rate_transposer.put_samples(output.ptr_begin(), num_samples);
            output.receive_samples_no_copy(num_samples);
         }        
         Ok(())
    }
    
    /// Internal put_samples implementation
    //fn put_samples_internal(&mut self, samples: &[Sample], num_samples: usize) {
    //}

    /// Get number of unprocessed samples
    pub fn num_unprocessed_samples(&self) -> usize {
        self.td_stretch.get_unmut_input().num_samples()
    }

    /// Flush remaining samples
    pub fn flush(&mut self) {
        let mut num_still_expected_i32 = ((self.samples_expected_out + 0.5) as i64 - self.samples_output as i64) as i32;
        if num_still_expected_i32 < 0 {
            num_still_expected_i32 = 0;
        }
        let num_still_expected = num_still_expected_i32 as usize;

        // Push blank samples through to flush
        let blank: Vec<Sample> = vec![0.0 as Sample; 128 * self.channels];
        for _ in 0..200 {
            if num_still_expected <= self.num_samples() {
                break;
            }
            let _ = self.put_samples(&blank, 128);
        }

        self.adjust_amount_of_samples(num_still_expected);

        self.td_stretch.clear_input();
    }

    /// Set a processing setting
    pub fn set_setting(&mut self, setting_id: SettingId, value: i32) -> bool {
        let (sample_rate, sequence_ms, seek_window_ms, overlap_ms) = 
            self.td_stretch.get_parameters();

        match setting_id {
            SettingId::UseAAFilter => {
                self.rate_transposer.enable_aa_filter(value != 0);
                true
            }
            SettingId::AAFilterLength => {
                let aa_filter = self.rate_transposer.get_aa_filter_mut();
                aa_filter.set_length(value as usize);
                true
            }
            SettingId::UseQuickSeek => {
                self.td_stretch.enable_quick_seek(value != 0);
                true
            }
            SettingId::SequenceMs => {
                self.td_stretch.set_parameters(sample_rate, value, seek_window_ms as i32, overlap_ms as i32);
                true
            }
            SettingId::SeekWindowMs => {
                self.td_stretch.set_parameters(sample_rate, sequence_ms as i32, value, overlap_ms as i32);
                true
            }
            SettingId::OverlapMs => {
                self.td_stretch.set_parameters(sample_rate, sequence_ms as i32, seek_window_ms as i32, value);
                true
            }
            _ => false,
        }
    }

    /// Get a processing setting
    pub fn get_setting(&self, setting_id: SettingId) -> i32 {
        let (_, sequence_ms, seek_window_ms, overlap_ms) = self.td_stretch.get_parameters();

        match setting_id {
            SettingId::UseAAFilter => self.rate_transposer.is_aa_filter_enabled() as i32,
            SettingId::AAFilterLength => {
                self.rate_transposer.get_aa_filter().get_length() as i32
            }
            SettingId::UseQuickSeek => self.td_stretch.is_quick_seek_enabled() as i32,
            SettingId::SequenceMs => sequence_ms as i32,
            SettingId::SeekWindowMs => seek_window_ms as i32,
            SettingId::OverlapMs => overlap_ms as i32,
            SettingId::NominalInputSequence => {
                let size = self.td_stretch.get_input_sample_req();
                if self.rate <= 1.0 {
                    (size as f64 * self.rate + 0.5) as i32
                } else {
                    size as i32
                }
            }
            SettingId::NominalOutputSequence => {
                let size = self.td_stretch.get_output_batch_size();
                if self.rate > 1.0 {
                    (size as f64 / self.rate + 0.5) as i32
                } else {
                    size as i32
                }
            }
            SettingId::InitialLatency => {
                let mut latency = self.td_stretch.get_latency() as f64;
                let latency_tr = self.rate_transposer.get_latency() as f64;
                
                if self.rate <= 1.0 {
                    latency = (latency + latency_tr) * self.rate;
                } else {
                    latency += latency_tr / self.rate;
                }
                (latency + 0.5) as i32
            }
        }
    }

    /// Get input/output sample ratio
    pub fn get_input_output_sample_ratio(&self) -> f64 {
        1.0 / (self.tempo * self.rate)
    }
    
    /// Receive processed samples (public method)
    pub fn receive_samples(&mut self, output: &mut [Sample], max_samples: usize) -> usize {
        let received = if self.rate_down {
            // Output comes from td_stretch when rate <= 1.0
            self.td_stretch.receive_samples(output, max_samples)
        } else {
            // Output comes from rate_transposer when rate > 1.0
            self.rate_transposer.receive_samples(output, max_samples)
        };
        self.samples_output += received;
        received
    }
    
    /// Get number of available output samples (public method)
    pub fn num_samples(&self) -> usize {
        if self.rate_down {
            self.td_stretch.num_samples()
        } else {
            self.rate_transposer.num_samples()
        }
    }
    
    /// Check if output is empty (public method)
    pub fn is_empty(&self) -> bool {
        if self.rate_down {
            self.td_stretch.is_empty()
        } else {
            self.rate_transposer.is_empty()
        }
    }
    
    /// Clear all buffers (public method)
    pub fn clear(&mut self) {
        self.samples_expected_out = 0.0;
        self.samples_output = 0;
        self.rate_transposer.clear();
        self.td_stretch.clear();
    }
    
    /// Adjust amount of samples
    pub fn adjust_amount_of_samples(&mut self, num_samples: usize) -> usize {
        if self.rate_down {
            self.td_stretch.adjust_amount_of_samples(num_samples)
        } else {
            self.rate_transposer.adjust_amount_of_samples(num_samples)
        }
    }
}

/*
// Implement FIFOSamplePipe trait (for generic usage)
impl FIFOSamplePipe for SoundTouch {
    fn put_samples(&mut self, samples: &[Sample], num_samples: usize) {
        // Note: This panics if not initialized (sample_rate or channels not set)
        // For Result-based error handling, use the public put_samples() method instead
        self.put_samples_internal(samples, num_samples);
    }
    
    fn receive_samples(&mut self, output: &mut [Sample], max_samples: usize) -> usize {
        self.receive_samples(output, max_samples)
    }
    
    fn num_samples(&self) -> usize {
        self.num_samples()
    }
    
    fn is_empty(&self) -> bool {
        self.is_empty()
    }
    
    fn clear(&mut self) {
        self.clear();
    }
    
    fn adjust_amount_of_samples(&mut self, num_samples: usize) -> usize {
        self.adjust_amount_of_samples(num_samples)
    }
}
*/

impl Default for SoundTouch {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_soundtouch_creation() {
        let st = SoundTouch::new();
        assert_eq!(st.num_channels(), 0);
    }

    #[test]
    fn test_set_channels() {
        let mut st = SoundTouch::new();
        assert!(st.set_channels(2).is_ok());
        assert_eq!(st.num_channels(), 2);
    }

    #[test]
    fn test_set_parameters() {
        let mut st = SoundTouch::new();
        st.set_sample_rate(44100);
        st.set_tempo_change(10.0);
        st.set_pitch_semi_tones(-3);
        st.set_rate_change(5.0);
    }

    #[test]
    fn test_rate_switching() {
        let mut st = SoundTouch::new();
        st.set_sample_rate(44100);
        let _ = st.set_channels(2);
        
        // Initially rate = 1.0, so rate_down should be true (1.0 <= 1.0)
        assert!(st.rate_down);
        
        // Set rate > 1.0, should switch to rate_down = false
        st.set_rate(1.5);
        assert!(!st.rate_down);
        assert_eq!(st.rate, 1.5);
        
        // Set rate < 1.0, should switch to rate_down = true
        st.set_rate(0.8);
        assert!(st.rate_down);
        assert_eq!(st.rate, 0.8);
        
        // Set rate = 1.0, should be rate_down = true (1.0 <= 1.0)
        st.set_rate(1.0);
        assert!(st.rate_down);
        assert_eq!(st.rate, 1.0);
    }

    #[test]
    fn test_pitch_rate_combination() {
        let mut st = SoundTouch::new();
        st.set_sample_rate(44100);
        let _ = st.set_channels(2);
        
        // Set pitch = 2.0 and virtual_rate = 0.5
        // effective rate = pitch * virtual_rate = 2.0 * 0.5 = 1.0
        st.set_pitch(2.0);
        st.set_rate(0.5);
        assert_eq!(st.rate, 1.0);
        assert!(st.rate_down); // rate = 1.0, so rate_down = true
        
        // Change pitch to 0.5
        // effective rate = 0.5 * 0.5 = 0.25
        st.set_pitch(0.5);
        assert_eq!(st.rate, 0.25);
        assert!(st.rate_down);
        
        // Change virtual_rate to 5.0
        // effective rate = 0.5 * 5.0 = 2.5
        st.set_rate(5.0);
        assert_eq!(st.rate, 2.5);
        assert!(!st.rate_down);
    }
}

