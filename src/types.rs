//! Common type definitions for SoundTouch

/// Sample type - using f32 for floating point samples
pub type Sample = f32;

/// Maximum number of channels supported
pub const MAX_CHANNELS: usize = 32;

/// Sample format enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleFormat {
    Float32,
    Int16,
}

/// Settings IDs for setSetting/getSetting functions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingId {
    /// Enable/disable anti-alias filter in pitch transposer
    UseAAFilter = 0,
    /// Pitch transposer anti-alias filter length (8..128 taps, default = 32)
    AAFilterLength = 1,
    /// Enable/disable quick seeking algorithm
    UseQuickSeek = 2,
    /// Time-stretch sequence length in milliseconds
    SequenceMs = 3,
    /// Time-stretch seeking window length in milliseconds
    SeekWindowMs = 4,
    /// Time-stretch overlap length in milliseconds
    OverlapMs = 5,
    /// Processing sequence size in samples (read-only)
    NominalInputSequence = 6,
    /// Nominal average output size in samples (read-only)
    NominalOutputSequence = 7,
    /// Initial processing latency in samples (read-only)
    InitialLatency = 8,
}

impl From<usize> for SettingId {
    fn from(value: usize) -> Self {
        match value {
            0 => SettingId::UseAAFilter,
            1 => SettingId::AAFilterLength,
            2 => SettingId::UseQuickSeek,
            3 => SettingId::SequenceMs,
            4 => SettingId::SeekWindowMs,
            5 => SettingId::OverlapMs,
            6 => SettingId::NominalInputSequence,
            7 => SettingId::NominalOutputSequence,
            8 => SettingId::InitialLatency,
            _ => SettingId::UseAAFilter,
        }
    }
}

/// Check if two floats are approximately equal
#[inline]
pub fn float_equal(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-10
}

