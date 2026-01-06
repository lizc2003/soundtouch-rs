//! FIFO Sample Buffer
//!
//! A buffer class for temporarily storing sound samples, operates as a
//! first-in-first-out pipe.

use crate::types::{Sample, MAX_CHANNELS};
use crate::error::{Result, SoundTouchError};
use crate::aligned_buffer::AlignedSampleVec;

/// FIFO Sample Buffer for audio processing
pub struct FIFOSampleBuffer {
    /// 16-byte aligned sample buffer for SIMD optimization
    buffer: AlignedSampleVec,
    /// Buffer size in bytes
    size_in_bytes: usize,
    /// How many samples currently in buffer
    samples_in_buffer: usize,
    /// Buffer position pointer
    buffer_pos: usize,
    /// Number of channels (1=mono, 2=stereo)
    channels: usize,
}

impl FIFOSampleBuffer {
    /// Create a new FIFO sample buffer
    pub fn new(num_channels: usize) -> Result<Self> {
        if num_channels == 0 || num_channels > MAX_CHANNELS {
            return Err(SoundTouchError::InvalidChannels(num_channels as u32));
        }

        let mut ret = FIFOSampleBuffer {
            buffer: AlignedSampleVec::new(0),
            size_in_bytes: 0,
            samples_in_buffer: 0,
            buffer_pos: 0,
            channels: num_channels,
        };
        ret.ensure_capacity(32);
        Ok(ret)
    }

    /// Add samples to the buffer
    pub fn put_samples(&mut self, samples: &[Sample], num_samples: usize) {
        // Ensure input array has enough elements
        debug_assert!(samples.len() >= num_samples * self.channels, 
                "Input array too small: got {} elements, need {} (num_samples={}, channels={})",
                samples.len(), num_samples * self.channels, num_samples, self.channels);
        
        self.ensure_capacity(self.samples_in_buffer + num_samples);
        
        let start_idx = (self.buffer_pos + self.samples_in_buffer) * self.channels;
        
        unsafe {
            std::ptr::copy_nonoverlapping(
                samples.as_ptr(),
                self.buffer.as_mut_ptr().add(start_idx),
                num_samples * self.channels,
            );
        }
        
        self.samples_in_buffer += num_samples;
    }

    pub fn put_samples_no_copy(&mut self, num_samples: usize) {
        self.ensure_capacity(self.samples_in_buffer + num_samples);
        self.samples_in_buffer += num_samples;
    }

    /// Receive samples from the buffer
    pub fn receive_samples(&mut self, output: &mut [Sample], max_samples: usize) -> usize {
        let num = max_samples.min(self.samples_in_buffer);
        if num == 0 {
            return 0;
        }

        let start = self.buffer_pos * self.channels;
        
        // Copy samples from buffer to output using ptr::copy (allows unaligned access)
        unsafe {
            std::ptr::copy_nonoverlapping(
                self.buffer.as_ptr().add(start),
                output.as_mut_ptr(),
                num * self.channels,
            );
        }
        
        self.samples_in_buffer -= num;
        self.buffer_pos += num;
        
        num
    }

    /// Remove samples from beginning without copying
    pub fn receive_samples_no_copy(&mut self, max_samples: usize) -> usize {
        let num = max_samples.min(self.samples_in_buffer);
        self.samples_in_buffer -= num;
        self.buffer_pos += num;
        num
    }

    pub fn move_samples(&mut self, src: &mut Self) {
        let num_samples = src.num_samples();
        if num_samples == 0 {
            return;
        }

        self.put_samples(src.ptr_begin(), num_samples);
        src.receive_samples_no_copy(num_samples);
    }

    /// Get number of samples currently in buffer
    pub fn num_samples(&self) -> usize {
        self.samples_in_buffer
    }

    /// Check if buffer is empty
    pub fn is_empty(&self) -> bool {
        self.samples_in_buffer == 0
    }

    /// Clear all samples from buffer
    pub fn clear(&mut self) {
        self.samples_in_buffer = 0;
        self.buffer_pos = 0;
    }

    /// Set number of channels
    pub fn set_channels(&mut self, num_channels: usize) -> Result<()> {
        if num_channels == 0 || num_channels > MAX_CHANNELS {
            return Err(SoundTouchError::InvalidChannels(num_channels as u32));
        }
        if num_channels != self.channels {
            let new_samples_in_buffer = self.samples_in_buffer * self.channels / num_channels;
            self.channels = num_channels;
            self.samples_in_buffer = new_samples_in_buffer;
        }
        Ok(())
    }

    /// Get number of channels
    pub fn get_channels(&self) -> usize {
        self.channels
    }

    /// Get pointer to beginning of samples
    pub fn ptr_begin(&self) -> &[Sample] {
        if self.samples_in_buffer == 0 {
            return &[];
        }
        
        let start = self.buffer_pos * self.channels;
        let count = self.samples_in_buffer * self.channels;
        
        unsafe {
            std::slice::from_raw_parts(self.buffer.as_ptr().add(start), count)
        }
    }

    pub fn ptr_end(&mut self, slack_capacity: usize) -> &mut [Sample] {
        self.ensure_capacity(self.samples_in_buffer + slack_capacity);

        let start = (self.buffer_pos + self.samples_in_buffer) * self.channels;
        let count = slack_capacity * self.channels;
        
        unsafe {
            std::slice::from_raw_parts_mut(self.buffer.as_mut_ptr().add(start), count)
        }
    }

    /// Ensure buffer has capacity for at least this many samples
    /// capacity_samples is the total number of samples we need to be able to hold
    fn ensure_capacity(&mut self, capacity_samples: usize) {
        let current_capacity = self.get_capacity();
        
        if capacity_samples > current_capacity {
            // Need to enlarge the buffer
            let bytes_needed = capacity_samples * self.channels * std::mem::size_of::<Sample>();
            self.size_in_bytes = (bytes_needed + 4095) & !4095;
            
            let new_capacity = self.size_in_bytes / std::mem::size_of::<Sample>();
            
            // Create new buffer with larger capacity
            let mut new_buffer = AlignedSampleVec::new(new_capacity);
            
            // Copy existing samples if any
            if self.samples_in_buffer > 0 {
                let start = self.buffer_pos * self.channels;
                let count = self.samples_in_buffer * self.channels;
                
                unsafe {
                    std::ptr::copy_nonoverlapping(
                        self.buffer.as_ptr().add(start),
                        new_buffer.as_mut_ptr(),
                        count,
                    );
                    new_buffer.set_len(new_capacity);
                }
            } else {
                // Even with no samples, set length to full capacity
                unsafe {
                    new_buffer.set_len(new_capacity);
                }
            }
            
            self.buffer = new_buffer;
            self.buffer_pos = 0;
        } else if self.buffer_pos + capacity_samples > current_capacity {
            // Current capacity is enough, but buffer_pos is non-zero and we don't 
            // have enough contiguous space, so need to rewind
            self.rewind();
        }
        // Otherwise, we have enough contiguous space, do nothing
    }

    /// Rewind buffer by moving data to beginning
    fn rewind(&mut self) {
        if self.buffer_pos == 0 {
            return;
        }
        
        if self.samples_in_buffer == 0 {
            self.buffer_pos = 0;
            return;
        }

        let start = self.buffer_pos * self.channels;
        let count = self.samples_in_buffer * self.channels;
        
        // This should never overflow if ensure_capacity is called correctly
        debug_assert!(count <= self.buffer.capacity(),
                     "Rewind overflow: start={}, count={}, capacity={}", 
                     start, count, self.buffer.capacity());
        
        unsafe {
            std::ptr::copy(
                self.buffer.as_ptr().add(start),
                self.buffer.as_mut_ptr(),
                count,
            );
        }
        
        self.buffer_pos = 0;
    }
    
    /// Get current capacity in samples
    fn get_capacity(&self) -> usize {
        self.size_in_bytes / (self.channels * std::mem::size_of::<Sample>())
    }

    /// Add silent samples
    pub fn add_silent(&mut self, num_samples: usize) {
        self.ensure_capacity(self.samples_in_buffer + num_samples);
        
        let start_idx = (self.buffer_pos + self.samples_in_buffer) * self.channels;
        let count = num_samples * self.channels;
        
        // Fill with zeros
        unsafe {
            std::ptr::write_bytes(self.buffer.as_mut_ptr().add(start_idx), 0, count);
        }
        
        self.samples_in_buffer += num_samples;
    }
    
    /// Adjust amount of samples in buffer (downwards only)
    pub fn adjust_amount_of_samples(&mut self, num_samples: usize) -> usize {
        if num_samples < self.samples_in_buffer {
            self.samples_in_buffer = num_samples;
        }
        self.samples_in_buffer
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_buffer() {
        let buf = FIFOSampleBuffer::new(2).unwrap();
        assert_eq!(buf.num_samples(), 0);
        assert!(buf.is_empty());
    }

    #[test]
    fn test_put_and_receive() {
        let mut buf = FIFOSampleBuffer::new(2).unwrap();
        let input: Vec<Sample> = vec![1.0, 2.0, 3.0, 4.0];
        buf.put_samples(&input, 2);
        
        assert_eq!(buf.num_samples(), 2);
        
        let mut output = vec![0.0; 4];
        let received = buf.receive_samples(&mut output, 2);
        
        assert_eq!(received, 2);
        assert_eq!(output, vec![1.0, 2.0, 3.0, 4.0]);
        assert_eq!(buf.num_samples(), 0);
    }
}

