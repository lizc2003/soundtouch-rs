//! Rate Transposer
//!
//! Transposes sample rate by interpolation to change both tempo and pitch

use crate::fifo_buffer::FIFOSampleBuffer;
use crate::aa_filter::AAFilter;
use crate::interpolate::{Interpolator, InterpolationAlgorithm, create_interpolator};
use crate::error::{Result, SoundTouchError};
use crate::types::{Sample, MAX_CHANNELS};

/// Rate transposer for changing playback rate
pub struct RateTransposer {
    /// Input buffer
    input_buffer: FIFOSampleBuffer,
    /// Output buffer
    output_buffer: FIFOSampleBuffer,
    /// Intermediate buffer (between AA filter and interpolator)
    mid_buffer: FIFOSampleBuffer,
    /// Anti-alias filter
    aa_filter: AAFilter,
    /// Interpolator
    interpolator: Box<dyn Interpolator>,
    /// Number of channels
    channels: usize,
    /// Whether AA filter is enabled
    aa_filter_enabled: bool,
}

#[allow(dead_code)]
impl RateTransposer {
    /// Create new rate transposer with default cubic interpolation
    pub fn new() -> Self {
        Self::with_algorithm(InterpolationAlgorithm::Cubic)
    }

    /// Create new rate transposer with specified interpolation algorithm
    pub fn with_algorithm(algorithm: InterpolationAlgorithm) -> Self {
        RateTransposer {
            input_buffer: FIFOSampleBuffer::new(2).unwrap(),
            output_buffer: FIFOSampleBuffer::new(2).unwrap(),
            mid_buffer: FIFOSampleBuffer::new(2).unwrap(),
            aa_filter: AAFilter::new(64),  // Match C++ default
            interpolator: create_interpolator(algorithm),
            channels: 2,
            aa_filter_enabled: true,
        }
    }
    
    /// Set interpolation algorithm
    pub fn set_algorithm(&mut self, algorithm: InterpolationAlgorithm) {
        let rate = self.interpolator.get_rate();
        self.interpolator = create_interpolator(algorithm);
        self.interpolator.set_rate(rate);
        self.interpolator.set_channels(self.channels);
    }
    
    /// Enable or disable anti-alias filter
    pub fn enable_aa_filter(&mut self, enable: bool) {
        if enable == self.aa_filter_enabled {
            return;
        }
        self.aa_filter_enabled = enable;
        self.clear();
    }
    
    /// Check if AA filter is enabled
    pub fn is_aa_filter_enabled(&self) -> bool {
        self.aa_filter_enabled
    }
    
    /// Get mutable reference to AA filter
    pub fn get_aa_filter_mut(&mut self) -> &mut AAFilter {
        &mut self.aa_filter
    }
    
    /// Get reference to AA filter
    pub fn get_aa_filter(&self) -> &AAFilter {
        &self.aa_filter
    }

    /// Set number of channels
    pub fn set_channels(&mut self, num_channels: usize) -> Result<()> {
        if num_channels == 0 || num_channels > MAX_CHANNELS {
            return Err(SoundTouchError::InvalidChannels(num_channels as u32));
        }

        if self.channels == num_channels {
            return Ok(());
        }
        
        self.channels = num_channels;
        self.input_buffer.set_channels(num_channels)?;
        self.output_buffer.set_channels(num_channels)?;
        self.mid_buffer.set_channels(num_channels)?;
        self.interpolator.set_channels(num_channels);
        
        Ok(())
    }

    /// Set rate value
    pub fn set_rate(&mut self, new_rate: f64) {
        self.interpolator.set_rate(new_rate);

        // Set cutoff to prevent aliasing
        // When downsampling (rate > 1), limit to prevent aliasing
        // When upsampling (rate < 1), use original frequency
        let cutoff = if new_rate > 1.0 {
            0.5 / new_rate
        } else {
            0.5 * new_rate
        };
        self.aa_filter.set_cutoff_freq(cutoff);
    }
    
    /// Get rate value
    pub fn get_rate(&self) -> f64 {
        self.interpolator.get_rate()
    }

    /// Put samples to input (public method)
    #[inline]
    pub fn put_samples(&mut self, samples: &[Sample], num_samples: usize) {
        self.input_buffer.put_samples(samples, num_samples);
        if self.input_buffer.num_samples() == 0 {
            return;
        }

        // If AA filter is disabled, transpose directly
        if !self.aa_filter_enabled {
            self.interpolator.transpose(&mut self.output_buffer, &mut self.input_buffer);
            return;
        }

        let rate = self.interpolator.get_rate();
        // Transpose with anti-alias filter
        if rate < 1.0 {
            // Upsampling: first transpose, then apply AA filter
            self.interpolator.transpose(&mut self.mid_buffer, &mut self.input_buffer);
            self.aa_filter.evaluate_fifo(&mut self.output_buffer, &mut self.mid_buffer);
        } else {
            // Downsampling: first apply AA filter, then transpose
            self.aa_filter.evaluate_fifo(&mut self.mid_buffer, &mut self.input_buffer);
            self.interpolator.transpose(&mut self.output_buffer, &mut self.mid_buffer);
        }
    }
    
    /// Receive samples from output (public method)
    pub fn receive_samples(&mut self, output: &mut [Sample], max_samples: usize) -> usize {
        self.output_buffer.receive_samples(output, max_samples)
    }
    
    /// Get number of available output samples (public method)
    pub fn num_samples(&self) -> usize {
        self.output_buffer.num_samples()
    }

    /// Get input buffer
    pub fn get_unmut_input(&self) -> &FIFOSampleBuffer {
        &self.input_buffer
    }

    /// Get input buffer
    pub fn get_input(&mut self) -> &mut FIFOSampleBuffer {
        &mut self.input_buffer
    }

    /// Get output buffer
    pub fn get_output(&mut self) -> &mut FIFOSampleBuffer {
        &mut self.output_buffer
    }
    
    /// Check if buffers are empty
    pub fn is_empty(&self) -> bool {
        self.output_buffer.is_empty() && self.input_buffer.is_empty()
    }
    
    /// Clear buffers (public method)
    pub fn clear(&mut self) {
        self.output_buffer.clear();
        self.mid_buffer.clear();
        self.input_buffer.clear();
        self.interpolator.reset();
        
        // Prefill buffer to avoid losing first samples at beginning of stream
        let prefill = self.get_latency();
        self.input_buffer.add_silent(prefill);
    }
    
    /// Adjust amount of samples in output buffer
    pub fn adjust_amount_of_samples(&mut self, num_samples: usize) -> usize {
        self.output_buffer.adjust_amount_of_samples(num_samples)
    }

    /// Get latency in samples
    pub fn get_latency(&self) -> usize {
        let mut latency = self.interpolator.get_latency();
        if self.aa_filter_enabled {
            latency += self.aa_filter.get_length() / 2;
        }
        latency
    }    
}

/*
// Implement FIFOSamplePipe trait (for generic usage)
impl FIFOSamplePipe for RateTransposer {
    fn put_samples(&mut self, samples: &[Sample], num_samples: usize) {
        self.put_samples(samples, num_samples);
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

impl Default for RateTransposer {
    fn default() -> Self {
        Self::new()
    }
}

