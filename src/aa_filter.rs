//! Anti-Alias Filter
//!
//! FIR low-pass (anti-alias) filter with filter coefficient design routine.
//! Used to prevent folding of high frequencies when transposing the sample rate.

use crate::fir_filter::FIRFilter;
use crate::fifo_buffer::FIFOSampleBuffer;
use crate::types::Sample;
use crate::error::Result;
use std::f64::consts::PI;

const TWOPI: f64 = 2.0 * PI;

/// Anti-alias filter using FIR filtering
pub struct AAFilter {
    /// Internal FIR filter
    fir: FIRFilter,
    /// Low-pass filter cut-off frequency (normalized, nyquist = 0.5)
    cutoff_freq: f64,
    /// Number of filter taps
    length: usize,
}

impl AAFilter {
    /// Create a new anti-alias filter
    ///
    /// # Arguments
    /// * `length` - Number of filter taps (must be divisible by 4)
    pub fn new(length: usize) -> Self {
        let mut filter = AAFilter {
            fir: FIRFilter::new(),
            cutoff_freq: 0.5,
            length,
        };
        filter.calculate_coeffs().ok();
        filter
    }

    /// Set anti-alias filter cut-off edge frequency
    ///
    /// Scaled to sampling frequency (nyquist frequency = 0.5).
    /// The filter will cut off frequencies higher than the given frequency.
    ///
    /// # Arguments
    /// * `new_cutoff_freq` - Cut-off frequency (0.0 to 0.5)
    pub fn set_cutoff_freq(&mut self, new_cutoff_freq: f64) {
        self.cutoff_freq = new_cutoff_freq.clamp(0.0, 0.5);
        self.calculate_coeffs().ok();
    }

    /// Set number of FIR filter taps
    ///
    /// # Arguments
    /// * `new_length` - Filter length (must be divisible by 4)
    pub fn set_length(&mut self, new_length: usize) {
        self.length = new_length;
        self.calculate_coeffs().ok();
    }

    /// Get filter length
    pub fn get_length(&self) -> usize {
        self.fir.get_length()
    }

    /// Calculate FIR coefficients for low-pass filter using Hamming window
    fn calculate_coeffs(&mut self) -> Result<()> {
        assert!(self.length >= 2);
        assert!(self.length % 4 == 0);
        assert!(self.cutoff_freq >= 0.0);
        assert!(self.cutoff_freq <= 0.5);

        let wc = 2.0 * PI * self.cutoff_freq;
        let temp_coeff = TWOPI / self.length as f64;

        let mut work = vec![0.0_f64; self.length];
        let mut sum = 0.0_f64;

        // Calculate filter coefficients using windowed sinc function
        for i in 0..self.length {
            let cnt_temp = i as f64 - (self.length / 2) as f64;
            let temp = cnt_temp * wc;

            // Sinc function: sin(x)/x
            let h = if temp.abs() > 1e-9 {
                temp.sin() / temp
            } else {
                1.0
            };

            // Hamming window: 0.54 + 0.46 * cos(2πn/N)
            let w = 0.54 + 0.46 * (temp_coeff * cnt_temp).cos();

            let coeff = w * h;
            work[i] = coeff;
            sum += coeff;
        }

        // Ensure valid filter design
        assert!(sum > 0.0, "Sum of coefficients must be positive");
        assert!(work[self.length / 2] > 0.0, "Filter center tap must be positive");

        // Calculate scaling coefficient to normalize to 16384
        // This allows using integer division by 2^14 = 16384
        let scale_coeff = 16384.0 / sum;

        let mut coeffs: Vec<Sample> = Vec::with_capacity(self.length);
        for i in 0..self.length {
            let temp = work[i] * scale_coeff;
            // Round to nearest
            let rounded = if temp >= 0.0 {
                temp + 0.5
            } else {
                temp - 0.5
            };
            
            // Ensure no overflows (should fit in i16 range)
            assert!(rounded >= -32768.0 && rounded <= 32767.0);
            coeffs.push(rounded as Sample);
        }

        // Set coefficients with divide factor 14 (divide result by 2^14 = 16384)
        self.fir.set_coefficients(&coeffs, self.length, 14)?;

        Ok(())
    }

    /// Apply filter to samples
    ///
    /// Note: The amount of output samples is by value of 'filter length'
    /// smaller than the amount of input samples.
    ///
    /// # Returns
    /// Number of samples written to dest
    pub fn evaluate(&self, dest: &mut [Sample], src: &[Sample], num_samples: usize, num_channels: usize) -> usize {
        self.fir.evaluate(dest, src, num_samples, num_channels)
    }

    /// Apply filter to FIFO buffers
    ///
    /// Processes samples from src buffer and adds results to dest buffer.
    /// Processed samples are removed from src.
    ///
    /// # Returns
    /// Number of samples processed
    pub fn evaluate_fifo(&self, dest: &mut FIFOSampleBuffer, src: &mut FIFOSampleBuffer) -> usize {
        let num_channels = src.get_channels();
        assert_eq!(num_channels, dest.get_channels());

        let num_src_samples = src.num_samples();
        if num_src_samples == 0 {
            return 0;
        }

        let src_data = src.ptr_begin();
        let dest_data = dest.ptr_end(num_src_samples);
        
        let result = self.fir.evaluate(dest_data, src_data, num_src_samples, num_channels);
        if result > 0 {
            // Remove processed samples from source
            src.receive_samples_no_copy(result);
            dest.put_samples_no_copy(result);
        }

        result
    }
}

impl Default for AAFilter {
    fn default() -> Self {
        Self::new(32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_aa_filter() {
        let filter = AAFilter::new(32);
        assert_eq!(filter.get_length(), 32);
    }

    #[test]
    fn test_set_cutoff_freq() {
        let mut filter = AAFilter::new(32);
        filter.set_cutoff_freq(0.4);
        assert_eq!(filter.cutoff_freq, 0.4);
    }

    #[test]
    fn test_set_length() {
        let mut filter = AAFilter::new(32);
        filter.set_length(64);
        assert_eq!(filter.get_length(), 64);
    }

    #[test]
    fn test_cutoff_freq_clamping() {
        let mut filter = AAFilter::new(32);
        
        // Test upper bound
        filter.set_cutoff_freq(1.0);
        assert_eq!(filter.cutoff_freq, 0.5);
        
        // Test lower bound
        filter.set_cutoff_freq(-0.1);
        assert_eq!(filter.cutoff_freq, 0.0);
    }

    #[test]
    fn test_evaluate() {
        let filter = AAFilter::new(32);
        let src = vec![1.0; 128];
        let mut dest = vec![0.0; 128];

        let result = filter.evaluate(&mut dest, &src, 64, 2);
        assert!(result > 0);
        assert!(result < 64); // Output is smaller due to filter length
    }

    #[test]
    fn test_evaluate_fifo() {
        let filter = AAFilter::new(32);
        let mut src = FIFOSampleBuffer::new(2).unwrap();
        let mut dest = FIFOSampleBuffer::new(2).unwrap();

        let input = vec![1.0; 256];
        src.put_samples(&input, 128);

        let result = filter.evaluate_fifo(&mut dest, &mut src);
        assert!(result > 0);
    }
}

