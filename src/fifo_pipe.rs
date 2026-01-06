//! FIFO Sample Pipe
//!
//! Abstract base trait for FIFO (first-in-first-out) sample processing.
//! Processing stages can be chained together so that samples fed into
//! the beginning automatically go through all processing stages.
//!
//! This matches the C++ `FIFOSamplePipe` abstract base class.

use crate::types::Sample;

/// Abstract base trait for FIFO (first-in-first-out) sample processing.
/// 
/// Equivalent to C++ `FIFOSamplePipe` abstract base class.
#[allow(dead_code)]
pub trait FIFOSamplePipe {
    /// Add samples to the pipe
    fn put_samples(&mut self, samples: &[Sample], num_samples: usize);
    
    /// Receive samples from the pipe
    fn receive_samples(&mut self, output: &mut [Sample], max_samples: usize) -> usize;
    
    /// Get number of samples currently available
    fn num_samples(&self) -> usize;
    
    /// Check if pipe is empty (has default implementation)
    fn is_empty(&self) -> bool {
        self.num_samples() == 0
    }
    
    /// Clear all samples
    fn clear(&mut self);
    
    /// Allow trimming (downwards) amount of samples in pipeline
    /// Returns adjusted amount of samples
    fn adjust_amount_of_samples(&mut self, num_samples: usize) -> usize;
}

