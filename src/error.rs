//! Error types for SoundTouch library

use thiserror::Error;

#[derive(Error, Debug)]
pub enum SoundTouchError {
    #[error("Sample rate not set")]
    SampleRateNotSet,
    
    #[error("Number of channels not set")]
    ChannelsNotSet,
    
    #[error("Invalid number of channels: {0}")]
    InvalidChannels(u32),
    
    #[error("Invalid sample rate: {0}")]
    InvalidSampleRate(u32),
    
    #[error("Buffer too small: required {required}, got {actual}")]
    BufferTooSmall { required: usize, actual: usize },
    
    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("WAV error: {0}")]
    Wav(#[from] hound::Error),
}

pub type Result<T> = std::result::Result<T, SoundTouchError>;

