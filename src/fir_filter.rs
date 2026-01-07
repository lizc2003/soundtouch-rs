//! FIR (Finite Impulse Response) Filter
//!
//! General FIR digital filter routines for audio processing

use crate::types::Sample;
use crate::error::{Result, SoundTouchError};

/// FIR Filter for audio signal processing
pub struct FIRFilter {
    /// Number of FIR filter taps
    length: usize,
    /// Number of FIR filter taps divided by 8
    length_div8: usize,
    /// Result divider factor in 2^k format
    result_div_factor: u32,
    /// Filter coefficients for mono
    filter_coeffs: Vec<Sample>,
    /// Filter coefficients for stereo (duplicated for vectorization)
    filter_coeffs_stereo: Vec<Sample>,
}

impl FIRFilter {
    /// Create a new FIR filter
    pub fn new() -> Self {
        FIRFilter {
            length: 0,
            length_div8: 0,
            result_div_factor: 0,
            filter_coeffs: Vec::new(),
            filter_coeffs_stereo: Vec::new(),
        }
    }

    /// Set filter coefficients and length
    ///
    /// # Arguments
    /// * `coeffs` - Filter coefficients
    /// * `new_length` - Filter length (must be divisible by 8)
    /// * `result_div_factor` - Result divider factor in 2^k format
    pub fn set_coefficients(&mut self, coeffs: &[Sample], new_length: usize, result_div_factor: u32) -> Result<()> {
        if new_length == 0 {
            return Err(SoundTouchError::InvalidParameter("Filter length must be > 0".to_string()));
        }
        
        if new_length % 8 != 0 {
            return Err(SoundTouchError::InvalidParameter("FIR filter length not divisible by 8".to_string()));
        }

        self.length_div8 = new_length / 8;
        self.length = new_length;
        self.result_div_factor = result_div_factor;

        // For floating point samples, scale coefficients by 2^(-result_div_factor)
        let scale = 0.5_f32.powi(result_div_factor as i32);

        // Allocate coefficient arrays
        self.filter_coeffs = Vec::with_capacity(self.length);
        self.filter_coeffs_stereo = Vec::with_capacity(self.length * 2);

        for coeff in coeffs.iter().take(self.length) {
            let scaled_coeff = coeff * scale;
            self.filter_coeffs.push(scaled_coeff);
            
            // Create stereo set of filter coefficients for better vectorization
            self.filter_coeffs_stereo.push(scaled_coeff);
            self.filter_coeffs_stereo.push(scaled_coeff);
        }

        Ok(())
    }

    /// Get filter length
    pub fn get_length(&self) -> usize {
        self.length
    }

    /// Apply filter to samples
    ///
    /// Note: The amount of output samples is by value of 'filter_length'
    /// smaller than the amount of input samples.
    ///
    /// # Returns
    /// Number of samples copied to dest
    pub fn evaluate(&self, dest: &mut [Sample], src: &[Sample], num_samples: usize, num_channels: usize) -> usize {
        if self.length == 0 {
            return 0;
        }

        if num_samples < self.length {
            return 0;
        }

        match num_channels {
            1 => self.evaluate_filter_mono(dest, src, num_samples),
            2 => self.evaluate_filter_stereo(dest, src, num_samples),
            _ => self.evaluate_filter_multi(dest, src, num_samples, num_channels),
        }
    }

    /// Filter evaluation for mono
    fn evaluate_filter_mono(&self, dest: &mut [Sample], src: &[Sample], num_samples: usize) -> usize {
        let ilength = self.length & !7; // Hint for compiler: divisible by 8
        let end = num_samples - ilength;

        for j in 0..end {
            let src_ptr = &src[j..];
            let mut sum: f32 = 0.0;

            for i in 0..ilength {
                sum += src_ptr[i] * self.filter_coeffs[i];
            }

            dest[j] = sum;
        }

        end
    }

    /// Filter evaluation for stereo
    fn evaluate_filter_stereo(&self, dest: &mut [Sample], src: &[Sample], num_samples: usize) -> usize {
        let ilength = self.length & !7; // Hint for compiler: divisible by 8
        let end = 2 * (num_samples - ilength);

        for j in (0..end).step_by(2) {
            let src_ptr = &src[j..];
            let mut sum_l: f32 = 0.0;
            let mut sum_r: f32 = 0.0;

            for i in 0..ilength {
                let idx = 2 * i;
                sum_l += src_ptr[idx] * self.filter_coeffs_stereo[idx];
                sum_r += src_ptr[idx + 1] * self.filter_coeffs_stereo[idx + 1];
            }

            dest[j] = sum_l;
            dest[j + 1] = sum_r;
        }

        num_samples - ilength
    }

    /// Filter evaluation for multi-channel
    fn evaluate_filter_multi(&self, dest: &mut [Sample], src: &[Sample], num_samples: usize, num_channels: usize) -> usize {
        let ilength = self.length & !7; // Hint for compiler: divisible by 8
        let end = num_channels * (num_samples - ilength);

        for j in (0..end).step_by(num_channels) {
            let mut sums = vec![0.0_f32; num_channels];
            let src_ptr = &src[j..];

            for i in 0..ilength {
                let idx = i * num_channels;
                let coef = self.filter_coeffs[i];
                for c in 0..num_channels {
                    sums[c] += src_ptr[idx + c] * coef;
                }
            }

            for c in 0..num_channels {
                dest[j + c] = sums[c];
            }
        }

        num_samples - ilength
    }
}

impl Default for FIRFilter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_filter() {
        let filter = FIRFilter::new();
        assert_eq!(filter.get_length(), 0);
    }

    #[test]
    fn test_set_coefficients() {
        let mut filter = FIRFilter::new();
        let coeffs = vec![1.0; 32];
        assert!(filter.set_coefficients(&coeffs, 32, 14).is_ok());
        assert_eq!(filter.get_length(), 32);
    }

    #[test]
    fn test_invalid_length() {
        let mut filter = FIRFilter::new();
        let coeffs = vec![1.0; 33]; // Not divisible by 8
        assert!(filter.set_coefficients(&coeffs, 33, 14).is_err());
    }

    #[test]
    fn test_evaluate_mono() {
        let mut filter = FIRFilter::new();
        let coeffs = vec![0.125; 32];
        filter.set_coefficients(&coeffs, 32, 0).unwrap();

        let src = vec![1.0; 64];
        let mut dest = vec![0.0; 64];

        let result = filter.evaluate(&mut dest, &src, 64, 1);
        assert!(result > 0);
    }

    #[test]
    fn test_evaluate_stereo() {
        let mut filter = FIRFilter::new();
        let coeffs = vec![0.125; 32];
        filter.set_coefficients(&coeffs, 32, 0).unwrap();

        let src = vec![1.0; 128];
        let mut dest = vec![0.0; 128];

        let result = filter.evaluate(&mut dest, &src, 64, 2);
        assert!(result > 0);
    }
}

