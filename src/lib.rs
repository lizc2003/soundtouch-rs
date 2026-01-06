//! # SoundTouch-RS
//!
//! Rust implementation of the SoundTouch audio processing library.
//! 
//! SoundTouch is an audio processing library that allows changing the sound tempo,
//! pitch and playback rate parameters independently from each other:
//! 
//! - **Tempo** (speed): Change playback speed while maintaining the original pitch
//! - **Pitch**: Change pitch while maintaining the original tempo
//! - **Rate**: Change both tempo and pitch together
//!
//! ## Example
//!
//! ```rust
//! use soundtouch::SoundTouch;
//!
//! let mut st = SoundTouch::new();
//! st.set_sample_rate(44100);
//! st.set_channels(2);
//! st.set_tempo_change(10.0); // 10% faster
//! st.set_pitch_semi_tones(-2); // 2 semitones lower
//!
//! // Process audio samples
//! // st.put_samples(&input_samples);
//! // let output = st.receive_samples(output_buffer);
//! ```

pub mod types;
pub mod fifo_buffer;
pub mod soundtouch;
pub mod bpm_detect;
pub mod error;

mod aligned_buffer;
mod fifo_pipe;
mod fir_filter;
mod aa_filter;
mod interpolate;
mod rate_transposer;
mod td_stretch;

pub use crate::soundtouch::SoundTouch;
pub use crate::types::{Sample, SampleFormat};
pub use crate::error::{SoundTouchError, Result};

/// Library version
pub const VERSION: &str = "2.4.0-rs";
pub const VERSION_ID: u32 = 20400;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_soundtouch() {
        let st = SoundTouch::new();
        assert_eq!(st.num_channels(), 0);
    }

    #[test]
    fn test_set_parameters() {
        let mut st = SoundTouch::new();
        st.set_sample_rate(44100);
        st.set_channels(2).unwrap();
        st.set_tempo_change(10.0);
        st.set_pitch_semi_tones(-3);
    }
}
