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
    /// Optimized with 4 independent accumulators to reduce data dependencies
    #[cfg(feature = "unsafe-optimizations")]
    #[inline(always)]
    fn evaluate_filter_mono(&self, dest: &mut [Sample], src: &[Sample], num_samples: usize) -> usize {
        let ilength = self.length & !7; // Hint for compiler: divisible by 8
        let end = num_samples - ilength;

        // SAFETY precondition checks
        debug_assert!(src.len() >= end + ilength, "src buffer too small");
        debug_assert!(dest.len() >= end, "dest buffer too small");
        debug_assert!(self.filter_coeffs.len() >= ilength, "coeffs buffer too small");

        unsafe {
            let src_base = src.as_ptr();
            let coef_ptr = self.filter_coeffs.as_ptr();
            let dest_base = dest.as_mut_ptr();

            for j in 0..end {
                let src_ptr = src_base.add(j);
                
                // Use 4 independent accumulators to reduce data dependencies
                let mut sum0: f32 = 0.0;
                let mut sum1: f32 = 0.0;
                let mut sum2: f32 = 0.0;
                let mut sum3: f32 = 0.0;

                // Unroll by 8, distributing to 4 accumulators
                let mut i = 0;
                while i + 8 <= ilength {
                    // SAFETY: We've verified bounds above, all accesses are within range
                    let s0 = *src_ptr.add(i);
                    let s1 = *src_ptr.add(i + 1);
                    let s2 = *src_ptr.add(i + 2);
                    let s3 = *src_ptr.add(i + 3);
                    let s4 = *src_ptr.add(i + 4);
                    let s5 = *src_ptr.add(i + 5);
                    let s6 = *src_ptr.add(i + 6);
                    let s7 = *src_ptr.add(i + 7);
                    
                    let c0 = *coef_ptr.add(i);
                    let c1 = *coef_ptr.add(i + 1);
                    let c2 = *coef_ptr.add(i + 2);
                    let c3 = *coef_ptr.add(i + 3);
                    let c4 = *coef_ptr.add(i + 4);
                    let c5 = *coef_ptr.add(i + 5);
                    let c6 = *coef_ptr.add(i + 6);
                    let c7 = *coef_ptr.add(i + 7);
                    
                    sum0 += s0 * c0 + s4 * c4;
                    sum1 += s1 * c1 + s5 * c5;
                    sum2 += s2 * c2 + s6 * c6;
                    sum3 += s3 * c3 + s7 * c7;
                    
                    i += 8;
                }

                // Combine accumulators
                *dest_base.add(j) = (sum0 + sum1) + (sum2 + sum3);
            }
        }

        end
    }

    /// Filter evaluation for mono (safe version)
    #[cfg(not(feature = "unsafe-optimizations"))]
    #[inline(always)]
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
    /// Optimized with multiple independent accumulators for each channel
    #[cfg(feature = "unsafe-optimizations")]
    #[inline(always)]
    fn evaluate_filter_stereo(&self, dest: &mut [Sample], src: &[Sample], num_samples: usize) -> usize {
        let ilength = self.length & !7; // Hint for compiler: divisible by 8
        let end = 2 * (num_samples - ilength);

        // SAFETY precondition checks
        debug_assert!(src.len() >= end + 2 * ilength, "src buffer too small");
        debug_assert!(dest.len() >= end, "dest buffer too small");
        debug_assert!(self.filter_coeffs_stereo.len() >= 2 * ilength, "stereo coeffs buffer too small");

        unsafe {
            let src_base = src.as_ptr();
            let coef_ptr = self.filter_coeffs_stereo.as_ptr();
            let dest_base = dest.as_mut_ptr();

            let mut j = 0;
            while j < end {
                let src_ptr = src_base.add(j);
                
                // Use 2 independent accumulators per channel to reduce dependencies
                let mut sum_l0: f32 = 0.0;
                let mut sum_l1: f32 = 0.0;
                let mut sum_r0: f32 = 0.0;
                let mut sum_r1: f32 = 0.0;

                // Unroll by 4 iterations for stereo (8 samples total per unroll)
                let mut i = 0;
                while i + 4 <= ilength {
                    let idx0 = 2 * i;
                    let idx1 = 2 * (i + 1);
                    let idx2 = 2 * (i + 2);
                    let idx3 = 2 * (i + 3);

                    // SAFETY: All indices verified within bounds
                    // Load source samples
                    let sl0 = *src_ptr.add(idx0);
                    let sr0 = *src_ptr.add(idx0 + 1);
                    let sl1 = *src_ptr.add(idx1);
                    let sr1 = *src_ptr.add(idx1 + 1);
                    let sl2 = *src_ptr.add(idx2);
                    let sr2 = *src_ptr.add(idx2 + 1);
                    let sl3 = *src_ptr.add(idx3);
                    let sr3 = *src_ptr.add(idx3 + 1);
                    
                    // Load coefficients
                    let cl0 = *coef_ptr.add(idx0);
                    let cr0 = *coef_ptr.add(idx0 + 1);
                    let cl1 = *coef_ptr.add(idx1);
                    let cr1 = *coef_ptr.add(idx1 + 1);
                    let cl2 = *coef_ptr.add(idx2);
                    let cr2 = *coef_ptr.add(idx2 + 1);
                    let cl3 = *coef_ptr.add(idx3);
                    let cr3 = *coef_ptr.add(idx3 + 1);
                    
                    // Distribute to independent accumulators
                    sum_l0 += sl0 * cl0 + sl2 * cl2;
                    sum_l1 += sl1 * cl1 + sl3 * cl3;
                    sum_r0 += sr0 * cr0 + sr2 * cr2;
                    sum_r1 += sr1 * cr1 + sr3 * cr3;
                    
                    i += 4;
                }

                // Handle remaining iterations
                while i < ilength {
                    let idx = 2 * i;
                    sum_l0 += *src_ptr.add(idx) * *coef_ptr.add(idx);
                    sum_r0 += *src_ptr.add(idx + 1) * *coef_ptr.add(idx + 1);
                    i += 1;
                }

                // Combine accumulators
                *dest_base.add(j) = sum_l0 + sum_l1;
                *dest_base.add(j + 1) = sum_r0 + sum_r1;
                j += 2;
            }
        }

        num_samples - ilength
    }

    /// Filter evaluation for stereo (safe version)
    #[cfg(not(feature = "unsafe-optimizations"))]
    #[inline(always)]
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
    #[cfg(feature = "unsafe-optimizations")]
    fn evaluate_filter_multi(&self, dest: &mut [Sample], src: &[Sample], num_samples: usize, num_channels: usize) -> usize {
        let ilength = self.length & !7; // Hint for compiler: divisible by 8
        let end = num_channels * (num_samples - ilength);

        // SAFETY precondition checks
        debug_assert!(src.len() >= end + ilength * num_channels, "src buffer too small");
        debug_assert!(dest.len() >= end, "dest buffer too small");
        debug_assert!(self.filter_coeffs.len() >= ilength, "coeffs buffer too small");

        // Use stack-allocated array for small channel counts to avoid allocation
        let mut sums_stack = [0.0_f32; 8];
        
        unsafe {
            let src_base = src.as_ptr();
            let coef_ptr = self.filter_coeffs.as_ptr();
            let dest_base = dest.as_mut_ptr();

            let mut j = 0;
            while j < end {
                let src_ptr = src_base.add(j);
                
                // Use stack array for common cases, heap for unusual channel counts
                let sums: &mut [f32] = if num_channels <= 8 {
                    sums_stack[..num_channels].fill(0.0);
                    &mut sums_stack[..num_channels]
                } else {
                    // Rare case: more than 8 channels
                    let mut heap_sums = vec![0.0_f32; num_channels];
                    // Process directly to avoid borrowing issues
                    for i in 0..ilength {
                        let idx = i * num_channels;
                        let coef = *coef_ptr.add(i);
                        for c in 0..num_channels {
                            heap_sums[c] += *src_ptr.add(idx + c) * coef;
                        }
                    }
                    for c in 0..num_channels {
                        *dest_base.add(j + c) = heap_sums[c];
                    }
                    j += num_channels;
                    continue;
                };

                // Process filter taps
                for i in 0..ilength {
                    let idx = i * num_channels;
                    let coef = *coef_ptr.add(i);
                    for c in 0..num_channels {
                        sums[c] += *src_ptr.add(idx + c) * coef;
                    }
                }

                // Write output
                for c in 0..num_channels {
                    *dest_base.add(j + c) = sums[c];
                }
                j += num_channels;
            }
        }

        num_samples - ilength
    }

    /// Filter evaluation for multi-channel (safe version)
    #[cfg(not(feature = "unsafe-optimizations"))]
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

