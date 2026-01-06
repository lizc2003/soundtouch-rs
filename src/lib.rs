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
//! use soundtouch::soundtouch::SoundTouch;
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
pub mod error;
pub mod soundtouch;
pub mod bpm_detect;

mod fifo_buffer;
mod aligned_buffer;
mod fifo_pipe;
mod fir_filter;
mod aa_filter;
mod interpolate;
mod rate_transposer;
mod td_stretch;
