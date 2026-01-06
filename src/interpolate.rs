//! Audio Interpolation Algorithms
//!
//! Provides different interpolation methods for sample rate transposition.

use crate::types::Sample;
use crate::fifo_buffer::FIFOSampleBuffer;

/// Interpolation algorithm types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum InterpolationAlgorithm {
    /// Linear interpolation (fast, lower quality)
    Linear,
    /// Cubic interpolation (medium speed, good quality)
    Cubic,
}

/// Trait for sample interpolation algorithms
pub trait Interpolator: Send {
    /// Transpose mono audio samples
    fn transpose_mono(
        &mut self,
        dest: &mut [Sample],
        src: &[Sample],
        src_samples: &mut usize,
    ) -> usize;

    /// Transpose stereo audio samples
    fn transpose_stereo(
        &mut self,
        dest: &mut [Sample],
        src: &[Sample],
        src_samples: &mut usize,
    ) -> usize;

    /// Transpose multi-channel audio samples
    fn transpose_multi(
        &mut self,
        dest: &mut [Sample],
        src: &[Sample],
        src_samples: &mut usize,
        num_channels: usize,
    ) -> usize;

    /// Set the transposition rate
    fn set_rate(&mut self, rate: f64);

    /// Get the transposition rate
    fn get_rate(&self) -> f64;

    /// Get number of channels
    fn get_channels(&self) -> usize;

    /// Set number of channels
    fn set_channels(&mut self, channels: usize);

    /// Reset internal state
    fn reset(&mut self);

    /// Get latency in samples
    fn get_latency(&self) -> usize;

    /// Main transpose function using FIFO buffers (default implementation)
    fn transpose(
        &mut self,
        dest: &mut FIFOSampleBuffer,
        src: &mut FIFOSampleBuffer,
    ) -> usize {
        let num_src_samples = src.num_samples();
        if num_src_samples == 0 {
            return 0;
        }

        let size_demand = ((num_src_samples as f64) / self.get_rate()) as usize + 8;
        let src_data = src.ptr_begin();
        let dest_data = dest.ptr_end(size_demand);

        let channels = self.get_channels();
        let mut consumed = num_src_samples;
        
        let num_output = if channels == 1 {
            self.transpose_mono(dest_data, src_data, &mut consumed)
        } else if channels == 2 {
            self.transpose_stereo(dest_data, src_data, &mut consumed)
        } else {
            self.transpose_multi(dest_data, src_data, &mut consumed, channels)
        };

        dest.put_samples_no_copy(num_output);
        src.receive_samples_no_copy(consumed);

        num_output
    }
}

/// Linear interpolation
pub struct LinearInterpolator {
    fract_pos: f64,
    rate: f64,
    channels: usize,
}

impl LinearInterpolator {
    pub fn new() -> Self {
        LinearInterpolator {
            fract_pos: 0.0,
            rate: 1.0,
            channels: 2,
        }
    }
}

impl Default for LinearInterpolator {
    fn default() -> Self {
        Self::new()
    }
}

impl Interpolator for LinearInterpolator {
    fn transpose_mono(
        &mut self,
        dest: &mut [Sample],
        src: &[Sample],
        src_samples: &mut usize,
    ) -> usize {
        let src_sample_end = src_samples.saturating_sub(1);
        let mut src_count = 0;
        let mut i = 0;

        while src_count < src_sample_end {
            debug_assert!(self.fract_pos < 1.0);

            // Linear interpolation: out = (1.0 - fract) * src[0] + fract * src[1]
            let out = (1.0 - self.fract_pos) * src[src_count] as f64 
                    + self.fract_pos * src[src_count + 1] as f64;
            dest[i] = out as Sample;
            i += 1;

            // Update position fraction
            self.fract_pos += self.rate;
            
            // Update whole positions
            let whole = self.fract_pos as usize;
            self.fract_pos -= whole as f64;
            src_count += whole;
        }

        *src_samples = src_count;
        i
    }

    fn transpose_stereo(
        &mut self,
        dest: &mut [Sample],
        src: &[Sample],
        src_samples: &mut usize,
    ) -> usize {
        let src_sample_end = src_samples.saturating_sub(1);
        let mut src_count = 0;
        let mut i = 0;

        while src_count < src_sample_end {
            debug_assert!(self.fract_pos < 1.0);

            // Linear interpolation for stereo
            let idx = src_count * 2;
            let out0 = (1.0 - self.fract_pos) * src[idx] as f64
                     + self.fract_pos * src[idx+2] as f64;
            let out1 = (1.0 - self.fract_pos) * src[idx + 1] as f64
                     + self.fract_pos * src[idx + 3] as f64;
            
            dest[i * 2] = out0 as Sample;
            dest[i * 2 + 1] = out1 as Sample;
            i += 1;

            // Update position fraction
            self.fract_pos += self.rate;
            
            // Update whole positions
            let whole = self.fract_pos as usize;
            self.fract_pos -= whole as f64;
            src_count += whole;
        }

        *src_samples = src_count;
        i
    }

    fn transpose_multi(
        &mut self,
        dest: &mut [Sample],
        src: &[Sample],
        src_samples: &mut usize,
        num_channels: usize,
    ) -> usize {
        let src_sample_end = src_samples.saturating_sub(1);
        let mut src_count = 0;
        let mut dest_idx = 0;
        let mut i = 0;

        while src_count < src_sample_end {
            debug_assert!(self.fract_pos < 1.0);

            let vol1 = (1.0 - self.fract_pos) as f32;
            let fract_float = self.fract_pos as f32;
            
            for ch in 0..num_channels {
                let idx = src_count * num_channels + ch;
                let temp = vol1 * src[idx]
                         + fract_float * src[idx + num_channels];
                dest[dest_idx] = temp as Sample;
                dest_idx += 1;
            }
            i += 1;

            // Update position fraction
            self.fract_pos += self.rate;
            
            // Update whole positions
            let whole = self.fract_pos as usize;
            self.fract_pos -= whole as f64;
            src_count += whole;
        }

        *src_samples = src_count;
        i
    }

    fn set_rate(&mut self, rate: f64) {
        self.rate = rate;
    }

    fn get_rate(&self) -> f64 {
        self.rate
    }

    fn get_channels(&self) -> usize {
        self.channels
    }

    fn set_channels(&mut self, channels: usize) {
        self.channels = channels;
        self.reset();
    }

    fn reset(&mut self) {
        self.fract_pos = 0.0;
    }

    fn get_latency(&self) -> usize {
        0
    }
    
    // transpose() uses the default implementation from trait
}

/// Cubic interpolation using Hermite polynomials
pub struct CubicInterpolator {
    fract_pos: f64,
    rate: f64,
    channels: usize,
}

// Cubic interpolation coefficients matrix
// These coefficients define the Hermite cubic polynomial basis functions
const CUBIC_COEFFS: [f32; 16] = [
    -0.5,  1.0, -0.5, 0.0,   // y0 coefficients: -0.5*x^3 + 1.0*x^2 - 0.5*x + 0.0
     1.5, -2.5,  0.0, 1.0,   // y1 coefficients:  1.5*x^3 - 2.5*x^2 + 0.0*x + 1.0
    -1.5,  2.0,  0.5, 0.0,   // y2 coefficients: -1.5*x^3 + 2.0*x^2 + 0.5*x + 0.0
     0.5, -0.5,  0.0, 0.0,   // y3 coefficients:  0.5*x^3 - 0.5*x^2 + 0.0*x + 0.0
];

impl CubicInterpolator {
    pub fn new() -> Self {
        CubicInterpolator {
            rate: 1.0,
            fract_pos: 0.0,
            channels: 2,
        }
    }

    /// Calculate cubic interpolation weights
    #[inline]
    fn calc_cubic_weights(fract: f32) -> [f32; 4] {
        let x3 = 1.0;
        let x2 = fract;         // x
        let x1 = x2 * x2;       // x^2
        let x0 = x1 * x2;       // x^3

        let y0 = CUBIC_COEFFS[0] * x0 + CUBIC_COEFFS[1] * x1 + CUBIC_COEFFS[2] * x2 + CUBIC_COEFFS[3] * x3;
        let y1 = CUBIC_COEFFS[4] * x0 + CUBIC_COEFFS[5] * x1 + CUBIC_COEFFS[6] * x2 + CUBIC_COEFFS[7] * x3;
        let y2 = CUBIC_COEFFS[8] * x0 + CUBIC_COEFFS[9] * x1 + CUBIC_COEFFS[10] * x2 + CUBIC_COEFFS[11] * x3;
        let y3 = CUBIC_COEFFS[12] * x0 + CUBIC_COEFFS[13] * x1 + CUBIC_COEFFS[14] * x2 + CUBIC_COEFFS[15] * x3;

        [y0, y1, y2, y3]
    }
}

impl Default for CubicInterpolator {
    fn default() -> Self {
        Self::new()
    }
}

impl Interpolator for CubicInterpolator {
    fn transpose_mono(
        &mut self,
        dest: &mut [Sample],
        src: &[Sample],
        src_samples: &mut usize,
    ) -> usize {
        let src_sample_end = src_samples.saturating_sub(4);
        let mut src_count = 0;
        let mut i = 0;

        while src_count < src_sample_end {
            debug_assert!(self.fract_pos < 1.0);

            let weights = Self::calc_cubic_weights(self.fract_pos as f32);
            let out = weights[0] * src[src_count] 
                    + weights[1] * src[src_count + 1]
                    + weights[2] * src[src_count + 2]
                    + weights[3] * src[src_count + 3];

            dest[i] = out;
            i += 1;

            // Update position fraction
            self.fract_pos += self.rate;
            
            // Update whole positions
            let whole = self.fract_pos as usize;
            self.fract_pos -= whole as f64;
            src_count += whole;
        }

        *src_samples = src_count;
        i
    }

    fn transpose_stereo(
        &mut self,
        dest: &mut [Sample],
        src: &[Sample],
        src_samples: &mut usize,
    ) -> usize {
        let src_sample_end = src_samples.saturating_sub(4);
        let mut src_count = 0;
        let mut i = 0;

        while src_count < src_sample_end {
            debug_assert!(self.fract_pos < 1.0);

            let weights = Self::calc_cubic_weights(self.fract_pos as f32);
            let idx = src_count * 2;
            let out0 = weights[0] * src[idx]
                     + weights[1] * src[idx+2]
                     + weights[2] * src[idx+4]
                     + weights[3] * src[idx+6];

            let out1 = weights[0] * src[idx + 1]
                     + weights[1] * src[idx+3]
                     + weights[2] * src[idx+5]
                     + weights[3] * src[idx+7];

            dest[i * 2] = out0;
            dest[i * 2 + 1] = out1;
            i += 1;

            // Update position fraction
            self.fract_pos += self.rate;
            
            // Update whole positions
            let whole = self.fract_pos as usize;
            self.fract_pos -= whole as f64;
            src_count += whole;
        }

        *src_samples = src_count;
        i
    }

    fn transpose_multi(
        &mut self,
        dest: &mut [Sample],
        src: &[Sample],
        src_samples: &mut usize,
        num_channels: usize,
    ) -> usize {
        let src_sample_end = src_samples.saturating_sub(4);
        let mut src_count = 0;
        let mut dest_idx = 0;
        let mut i = 0;

        while src_count < src_sample_end {
            debug_assert!(self.fract_pos < 1.0);

            let weights = Self::calc_cubic_weights(self.fract_pos as f32);

            for ch in 0..num_channels {
                let idx = src_count * num_channels + ch;
                let out = weights[0] * src[idx]
                        + weights[1] * src[idx + num_channels]
                        + weights[2] * src[idx + 2 * num_channels]
                        + weights[3] * src[idx + 3 * num_channels];

                dest[dest_idx] = out;
                dest_idx += 1;
            }
            i += 1;

            // Update position fraction
            self.fract_pos += self.rate;
            
            // Update whole positions
            let whole = self.fract_pos as usize;
            self.fract_pos -= whole as f64;
            src_count += whole;
        }

        *src_samples = src_count;
        i
    }

    fn set_rate(&mut self, rate: f64) {
        self.rate = rate;
    }

    fn get_rate(&self) -> f64 {
        self.rate
    }

    fn get_channels(&self) -> usize {
        self.channels
    }

    fn set_channels(&mut self, channels: usize) {
        self.channels = channels;
        self.reset();
    }

    fn reset(&mut self) {
        self.fract_pos = 0.0;
    }

    fn get_latency(&self) -> usize {
        1  // Cubic needs 4 points, so latency is 2 samples
    }
    
    // transpose() uses the default implementation from trait
}

/// Create an interpolator based on the algorithm type
pub fn create_interpolator(algorithm: InterpolationAlgorithm) -> Box<dyn Interpolator> {
    match algorithm {
        InterpolationAlgorithm::Linear => Box::new(LinearInterpolator::new()),
        InterpolationAlgorithm::Cubic => Box::new(CubicInterpolator::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cubic_weights() {
        // Test at fract = 0.0 (should give weight 1.0 to second sample)
        let weights = CubicInterpolator::calc_cubic_weights(0.0);
        assert!((weights[1] - 1.0).abs() < 0.01);
        assert!(weights[0].abs() < 0.01);
        assert!(weights[2].abs() < 0.01);
        assert!(weights[3].abs() < 0.01);

        // Test weights sum to approximately 1.0
        let weights = CubicInterpolator::calc_cubic_weights(0.5);
        let sum: f32 = weights.iter().sum();
        assert!((sum - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_linear_interpolator() {
        let mut interp = LinearInterpolator::new();
        interp.set_rate(0.5); // Slower rate

        let src = vec![0.0, 1.0, 2.0, 3.0];
        let mut dest = vec![0.0; 8];
        let mut src_samples = 4;

        let output = interp.transpose_mono(&mut dest, &src, &mut src_samples);
        assert!(output > 0);
    }

    #[test]
    fn test_cubic_interpolator() {
        let mut interp = CubicInterpolator::new();
        interp.set_rate(0.5);

        let src = vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0];
        let mut dest = vec![0.0; 12];
        let mut src_samples = 6;

        let output = interp.transpose_mono(&mut dest, &src, &mut src_samples);
        assert!(output > 0);
    }
}

