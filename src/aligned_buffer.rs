//! Aligned Memory Buffer
//!
//! Provides 16-byte aligned memory allocation for optimal SIMD performance.

use crate::types::Sample;
use std::alloc::{alloc, dealloc, Layout};
use std::ptr::NonNull;
use std::slice;

pub const ALIGN_BYTES: usize = 16;

/// 16-byte aligned sample buffer for SIMD optimization
pub struct AlignedSampleVec {
    /// Aligned allocation pointer (16-byte aligned)
    buffer: NonNull<Sample>,
    /// Layout of the allocation
    layout: Layout,
    /// Current capacity in number of samples
    capacity: usize,
    /// Current length in number of samples
    length: usize,
}

impl AlignedSampleVec {
    /// Create a new aligned sample vector with given capacity
    pub fn new(capacity: usize) -> Self {
        if capacity == 0 {
            // Handle zero capacity case
            return AlignedSampleVec {
                buffer: NonNull::dangling(),
                layout: Layout::from_size_align(0, 1).unwrap(),
                capacity: 0,
                length: 0,
            };
        }

        let bytes_needed = capacity * std::mem::size_of::<Sample>();
        
        // Create layout with 16-byte alignment - allocator will return aligned memory
        let layout = Layout::from_size_align(bytes_needed, ALIGN_BYTES)
            .expect("Invalid layout for aligned buffer");

        unsafe {
            let ptr = alloc(layout) as *mut Sample;
            if ptr.is_null() {
                panic!("Failed to allocate memory for aligned buffer");
            }

            // The allocator guarantees the pointer is aligned to ALIGN_BYTES
            let buffer = NonNull::new_unchecked(ptr);

            AlignedSampleVec {
                buffer,
                layout,
                capacity,
                length: 0,
            }
        }
    }

    /// Get the capacity of the buffer
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Get the current length
    pub fn len(&self) -> usize {
        self.length
    }

    /// Check if the buffer is empty
    pub fn is_empty(&self) -> bool {
        self.length == 0
    }

    /// Set the length (unsafe - must ensure data is initialized)
    pub unsafe fn set_len(&mut self, new_len: usize) {
        assert!(new_len <= self.capacity);
        self.length = new_len;
    }

    /// Get a slice of the samples
    pub fn as_slice(&self) -> &[Sample] {
        if self.length == 0 {
            return &[];
        }
        unsafe { slice::from_raw_parts(self.buffer.as_ptr(), self.length) }
    }

    /// Get a mutable slice of the samples
    pub fn as_mut_slice(&mut self) -> &mut [Sample] {
        if self.length == 0 {
            return &mut [];
        }
        unsafe { slice::from_raw_parts_mut(self.buffer.as_ptr(), self.length) }
    }

    /// Get a raw pointer to the beginning
    pub fn as_ptr(&self) -> *const Sample {
        self.buffer.as_ptr()
    }

    /// Get a mutable raw pointer to the beginning
    pub fn as_mut_ptr(&mut self) -> *mut Sample {
        self.buffer.as_ptr()
    }

    /// Reserve capacity for at least additional samples
    pub fn reserve(&mut self, additional: usize) {
        let required = self.length + additional;
        if required <= self.capacity {
            return;
        }

        // Grow to at least the required size, rounded up to 4KB boundary
        let new_capacity = ((required * std::mem::size_of::<Sample>() + 4095) & !4095)
            / std::mem::size_of::<Sample>();

        let mut new_buffer = AlignedSampleVec::new(new_capacity);

        // Copy existing data
        if self.length > 0 {
            unsafe {
                std::ptr::copy_nonoverlapping(
                    self.buffer.as_ptr(),
                    new_buffer.buffer.as_ptr(),
                    self.length,
                );
                new_buffer.set_len(self.length);
            }
        }

        // Replace self with the new buffer
        std::mem::swap(self, &mut new_buffer);
        // new_buffer will be dropped here, freeing the old allocation
    }

    /// Resize the buffer to new_len, filling with default value if growing
    pub fn resize(&mut self, new_len: usize, value: Sample) {
        if new_len > self.capacity {
            self.reserve(new_len - self.length);
        }

        if new_len > self.length {
            // Fill new elements with value
            unsafe {
                let ptr = self.buffer.as_ptr().add(self.length);
                for i in 0..(new_len - self.length) {
                    std::ptr::write(ptr.add(i), value);
                }
            }
        }

        self.length = new_len;
    }

    /// Push a sample to the end
    pub fn push(&mut self, value: Sample) {
        if self.length >= self.capacity {
            self.reserve(self.capacity.max(32));
        }

        unsafe {
            std::ptr::write(self.buffer.as_ptr().add(self.length), value);
            self.length += 1;
        }
    }

    /// Clear the buffer (set length to 0)
    pub fn clear(&mut self) {
        self.length = 0;
    }

    /// Copy data from a slice starting at offset
    pub fn copy_from_slice_at(&mut self, offset: usize, src: &[Sample]) {
        let required_len = offset + src.len();
        if required_len > self.capacity {
            self.reserve(required_len - self.length);
        }

        unsafe {
            std::ptr::copy_nonoverlapping(
                src.as_ptr(),
                self.buffer.as_ptr().add(offset),
                src.len(),
            );

            if required_len > self.length {
                self.length = required_len;
            }
        }
    }

    /// Copy within the buffer (like Vec::copy_within)
    pub fn copy_within(&mut self, src_range: std::ops::Range<usize>, dest: usize) {
        let src_start = src_range.start;
        let src_end = src_range.end;
        assert!(src_end <= self.length);
        assert!(dest + (src_end - src_start) <= self.capacity);

        unsafe {
            std::ptr::copy(
                self.buffer.as_ptr().add(src_start),
                self.buffer.as_ptr().add(dest),
                src_end - src_start,
            );
        }
    }
}

impl Drop for AlignedSampleVec {
    fn drop(&mut self) {
        if self.capacity > 0 {
            unsafe {
                dealloc(self.buffer.as_ptr() as *mut u8, self.layout);
            }
        }
    }
}

// Safety: AlignedSampleVec owns its data and Sample is Send
unsafe impl Send for AlignedSampleVec {}
unsafe impl Sync for AlignedSampleVec {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alignment() {
        let buf = AlignedSampleVec::new(100);
        let ptr = buf.as_ptr() as usize;
        assert_eq!(ptr % ALIGN_BYTES, 0, "Buffer should be 16-byte aligned");
    }

    #[test]
    fn test_push_and_access() {
        let mut buf = AlignedSampleVec::new(10);
        buf.push(1.0);
        buf.push(2.0);
        buf.push(3.0);

        assert_eq!(buf.len(), 3);
        assert_eq!(buf.as_slice(), &[1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_resize() {
        let mut buf = AlignedSampleVec::new(10);
        buf.resize(5, 1.5);

        assert_eq!(buf.len(), 5);
        assert_eq!(buf.as_slice(), &[1.5, 1.5, 1.5, 1.5, 1.5]);
    }

    #[test]
    fn test_copy_from_slice() {
        let mut buf = AlignedSampleVec::new(10);
        let data = vec![1.0, 2.0, 3.0, 4.0];
        buf.copy_from_slice_at(0, &data);

        assert_eq!(buf.len(), 4);
        assert_eq!(buf.as_slice(), &[1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn test_reserve_and_grow() {
        let mut buf = AlignedSampleVec::new(2);
        buf.push(1.0);
        buf.push(2.0);
        buf.push(3.0); // This should trigger a resize

        assert!(buf.capacity() >= 3);
        assert_eq!(buf.len(), 3);

        // Check alignment is maintained after resize
        let ptr = buf.as_ptr() as usize;
        assert_eq!(ptr % ALIGN_BYTES, 0, "Buffer should remain 16-byte aligned after resize");
    }

    #[test]
    fn test_copy_within() {
        let mut buf = AlignedSampleVec::new(10);
        buf.resize(10, 0.0);
        buf.as_mut_slice()[5..8].copy_from_slice(&[1.0, 2.0, 3.0]);

        buf.copy_within(5..8, 0);

        assert_eq!(buf.as_slice()[0..3], [1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_zero_capacity() {
        let buf = AlignedSampleVec::new(0);
        assert_eq!(buf.capacity(), 0);
        assert_eq!(buf.len(), 0);
        assert!(buf.is_empty());
    }
}

