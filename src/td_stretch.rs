//! Time-Domain Stretcher
//!
//! Stretches audio tempo without changing pitch using time-domain algorithm

use crate::fifo_buffer::FIFOSampleBuffer;
use crate::fifo_pipe::FIFOSamplePipe;
use crate::error::{Result, SoundTouchError};
use crate::types::{Sample, MAX_CHANNELS};
use crate::aligned_buffer::AlignedSampleVec;

/// Default sequence length in milliseconds (auto mode)
const USE_AUTO_SEQUENCE_LEN: i32 = 0;
/// Default seek window length in milliseconds (auto mode)
const USE_AUTO_SEEKWINDOW_LEN: i32 = 0;

/// Time-domain stretch processor
pub struct TDStretch {
    /// Input buffer
    input_buffer: FIFOSampleBuffer,
    /// Output buffer
    output_buffer: FIFOSampleBuffer,
    
    /// Number of channels
    channels: usize,
    /// Sample rate
    sample_rate: u32,
    
    /// Tempo value
    tempo: f64,
    /// Nominal skip value (tempo-based)
    nominal_skip: f64,
    /// Skip fraction part (for error management)
    skip_fract: f64,
    
    /// Sequence length in milliseconds
    sequence_ms: i32,
    /// Seek window length in milliseconds
    seek_window_ms: i32,
    /// Overlap length in milliseconds
    overlap_ms: i32,
    
    /// Overlap length in samples
    overlap_length: usize,
    /// Seek length in samples
    seek_length: usize,
    /// Seek window length in samples
    seek_window_length: usize,
    /// Input sample requirement
    sample_req: usize,
    
    /// Quick seek enabled
    quick_seek: bool,
    /// Auto sequence setting enabled
    auto_seq_setting: bool,
    /// Auto seek window setting enabled
    auto_seek_setting: bool,
    /// Is at beginning of track
    is_beginning: bool,
    
    /// Mid buffer for storing end of previous sequence (16-byte aligned)
    mid_buffer: AlignedSampleVec,
}

impl TDStretch {
    /// Create new TD stretch processor
    pub fn new() -> Self {
        let mut td = TDStretch {
            input_buffer: FIFOSampleBuffer::new(2).unwrap(),
            output_buffer: FIFOSampleBuffer::new(2).unwrap(),
            
            channels: 2,
            sample_rate: 44100,
            
            tempo: 1.0,
            nominal_skip: 0.0,
            skip_fract: 0.0,
            
            sequence_ms: 82,  // Default from C++
            seek_window_ms: 28,
            overlap_ms: 12,
            
            overlap_length: 0,
            seek_length: 0,
            seek_window_length: 0,
            sample_req: 0,
            
            quick_seek: false,
            auto_seq_setting: true,
            auto_seek_setting: true,
            is_beginning: true,
            
            mid_buffer: AlignedSampleVec::new(0),
        };
        
        // Initialize with default parameters (auto mode)
        td.set_parameters(44100, 0, 0, 12);  // 0 means auto
        td.set_tempo(1.0);
        td.clear();
        
        td
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
        
        self.overlap_length = 0;
        self.set_parameters(self.sample_rate, -1, -1, -1);
        
        Ok(())
    }

    /// Set tempo
    pub fn set_tempo(&mut self, new_tempo: f64) {
        self.tempo = new_tempo;
        self.calc_seq_parameters();
        
        // Calculate nominal skip and sample requirement
        self.nominal_skip = self.tempo * (self.seek_window_length - self.overlap_length) as f64;
        let int_skip = (self.nominal_skip + 0.5) as i32;

        self.sample_req = (int_skip + self.overlap_length as i32).max(self.seek_window_length as i32) as usize + self.seek_length;
    }

    /// Set parameters
    pub fn set_parameters(&mut self, sample_rate: u32, sequence_ms: i32, seek_window_ms: i32, overlap_ms: i32) {
        if sample_rate > 0 {
            if sample_rate > 192000 {
                panic!("Error: Excessive samplerate");
            }
            self.sample_rate = sample_rate;
        }
        
        if overlap_ms > 0 {
            self.overlap_ms = overlap_ms;
        }
        
        if sequence_ms > 0 {
            self.sequence_ms = sequence_ms;
            self.auto_seq_setting = false;
        } else if sequence_ms == 0 {
            // Use automatic setting
            self.auto_seq_setting = true;
        }
        
        if seek_window_ms > 0 {
            self.seek_window_ms = seek_window_ms;
            self.auto_seek_setting = false;
        } else if seek_window_ms == 0 {
            // Use automatic setting
            self.auto_seek_setting = true;
        }
        
        self.calc_seq_parameters();
        self.calculate_overlap_length(self.overlap_ms);
        self.set_tempo(self.tempo);
    }

    /// Get parameters
    pub fn get_parameters(&self) -> (u32, usize, usize, usize) {
        let seq_ms = if self.auto_seq_setting { 
            USE_AUTO_SEQUENCE_LEN as usize
        } else { 
            self.sequence_ms as usize
        };
        
        let seek_ms = if self.auto_seek_setting { 
            USE_AUTO_SEEKWINDOW_LEN as usize
        } else { 
            self.seek_window_ms as usize
        };
        
        (self.sample_rate, seq_ms, seek_ms, self.overlap_ms as usize)
    }

    /// Enable/disable quick seek
    pub fn enable_quick_seek(&mut self, enable: bool) {
        self.quick_seek = enable;
    }

    /// Check if quick seek is enabled
    pub fn is_quick_seek_enabled(&self) -> bool {
        self.quick_seek
    }

    /// Calculate sequence parameters according to tempo setting
    fn calc_seq_parameters(&mut self) {
        // Auto setting ranges and formulas
        const AUTOSEQ_TEMPO_LOW: f64 = 0.5;    // -50%
        const AUTOSEQ_TEMPO_TOP: f64 = 2.0;    // +100%
        const AUTOSEQ_AT_MIN: f64 = 90.0;
        const AUTOSEQ_AT_MAX: f64 = 40.0;
        const AUTOSEEK_AT_MIN: f64 = 20.0;
        const AUTOSEEK_AT_MAX: f64 = 15.0;
        
        const AUTOSEQ_K: f64 = (AUTOSEQ_AT_MAX - AUTOSEQ_AT_MIN) / (AUTOSEQ_TEMPO_TOP - AUTOSEQ_TEMPO_LOW);
        const AUTOSEQ_C: f64 = AUTOSEQ_AT_MIN - AUTOSEQ_K * AUTOSEQ_TEMPO_LOW;
        
        const AUTOSEEK_K: f64 = (AUTOSEEK_AT_MAX - AUTOSEEK_AT_MIN) / (AUTOSEQ_TEMPO_TOP - AUTOSEQ_TEMPO_LOW);
        const AUTOSEEK_C: f64 = AUTOSEEK_AT_MIN - AUTOSEEK_K * AUTOSEQ_TEMPO_LOW;
        
        if self.auto_seq_setting {
            let mut seq = AUTOSEQ_C + AUTOSEQ_K * self.tempo;
            seq = seq.clamp(AUTOSEQ_AT_MAX, AUTOSEQ_AT_MIN);
            self.sequence_ms = (seq + 0.5) as i32;
        }
        
        if self.auto_seek_setting {
            let mut seek = AUTOSEEK_C + AUTOSEEK_K * self.tempo;
            seek = seek.clamp(AUTOSEEK_AT_MAX, AUTOSEEK_AT_MIN);
            self.seek_window_ms = (seek + 0.5) as i32;
        }
        
        // Update seek window lengths
        self.seek_window_length = (self.sample_rate as usize * self.sequence_ms as usize) / 1000;
        if self.seek_window_length < 2 * self.overlap_length {
            self.seek_window_length = 2 * self.overlap_length;
        }
        self.seek_length = (self.sample_rate as usize * self.seek_window_ms as usize) / 1000;
    }
    
    /// Calculate overlap length
    fn calculate_overlap_length(&mut self, overlap_ms: i32) {
        if overlap_ms < 0 {
            return; // Ignore negative values
        }

        let mut new_ovl = (self.sample_rate as usize * overlap_ms as usize) / 1000;
        if new_ovl < 16 {
            new_ovl = 16;
        } else {
            new_ovl -= new_ovl % 8;
        }
        // Call accept_new_overlap_length to update and reallocate if needed
        self.accept_new_overlap_length(new_ovl);
    }
    
    /// Accept new overlap length and reallocate mid buffer if necessary
    fn accept_new_overlap_length(&mut self, new_overlap_length: usize) {
        let prev_ovl = self.overlap_length;
        self.overlap_length = new_overlap_length;
        
        if new_overlap_length > prev_ovl {
            // Reallocate mid buffer
            let mid_buffer_size = new_overlap_length * self.channels;
            self.mid_buffer = AlignedSampleVec::new(mid_buffer_size);
            unsafe {
                self.mid_buffer.set_len(mid_buffer_size);
            }
            self.clear_mid_buffer();
        }
    }

    /// Clear mid buffer
    fn clear_mid_buffer(&mut self) {
        if self.mid_buffer.len() > 0 {
            unsafe {
                std::ptr::write_bytes(
                    self.mid_buffer.as_mut_ptr(),
                    0,
                    self.mid_buffer.len()
                );
            }
        }
    }

    /// Process samples
    pub fn process(&mut self) {
        let mut offset = 0;
        // Process samples as long as there are enough samples in inputBuffer
        while self.input_buffer.num_samples() >= self.sample_req {
            if !self.is_beginning {
                // Normal processing: find best overlap position
                offset = self.seek_best_overlap_position();
                
                // Mix with the end of the previous sequence in midBuffer
                self.overlap(offset);
                self.output_buffer.put_samples_no_copy(self.overlap_length);
                offset += self.overlap_length;
            } else {
                // Beginning of track: adjust processing offset
                self.is_beginning = false;
                let skip = (self.tempo * self.overlap_length as f64 
                          + 0.5 * self.seek_length as f64 + 0.5) as i32;
                
                self.skip_fract -= skip as f64;
                if self.skip_fract <= -self.nominal_skip {
                    self.skip_fract = -self.nominal_skip;
                }
            }
            
            if self.input_buffer.num_samples() < offset + self.seek_window_length - self.overlap_length {
                continue; // just in case, shouldn't really happen
            }
            
            // Copy sequence samples from inputBuffer to output
            let temp = self.seek_window_length - 2 * self.overlap_length;
            let input_ptr = self.input_buffer.ptr_begin();
            let src_offset = offset * self.channels;
            self.output_buffer.put_samples(&input_ptr[src_offset..], temp);
            
            // Copy the end of the current sequence to midBuffer for next overlap
            let mid_src_offset = (offset + temp) * self.channels;
            let mid_len = self.overlap_length * self.channels;
            unsafe {
                std::ptr::copy_nonoverlapping(
                    input_ptr[mid_src_offset..].as_ptr(),
                    self.mid_buffer.as_mut_ptr(),
                    mid_len,
                );
            }
            
            // Remove the processed samples from input buffer
            // Update skipFract to manage accumulated error
            self.skip_fract += self.nominal_skip;
            let ovl_skip = self.skip_fract as i32;
            self.skip_fract -= ovl_skip as f64;
            self.input_buffer.receive_samples_no_copy(ovl_skip as usize);
        }
    }
    
    /// Seek for the optimal overlap-mixing position
    fn seek_best_overlap_position(&self) -> usize {
        if self.quick_seek {
            self.seek_best_overlap_position_quick()
        } else {
            self.seek_best_overlap_position_full()
        }
    }

    /// Find best overlap position (full search)
    fn seek_best_overlap_position_full(&self) -> usize {
        let mut best_offs = 0;
        let input_ptr = self.input_buffer.ptr_begin();
        let mid_buffer = self.mid_buffer.as_slice();
        
        // Test position at the beginning
        let mut norm = 0.0;
        let mut best_corr = self.calc_cross_corr(input_ptr, mid_buffer, &mut norm);
        best_corr = (best_corr + 0.1) * 0.75;
        
        // Scan for the best correlation value
        // Use accumulative version for better performance (incrementally updates norm)
        for i in 1..self.seek_length {
            let corr = self.calc_cross_corr_accumulate(input_ptr, i, mid_buffer, &mut norm);
            
            // Heuristic rule to slightly favour values close to mid of the range
            let tmp = (2 * i as i64 - self.seek_length as i64) as f64 / self.seek_length as f64;
            let corr = (corr + 0.1) * (1.0 - 0.25 * tmp * tmp);
            
            if corr > best_corr {
                best_corr = corr;
                best_offs = i;
            }
        }

        // clear cross correlation routine state if necessary (is so e.g. in MMX routines).
        // clearCrossCorrState();
        
        best_offs
    }

    /// Find best overlap position (quick search)
    fn seek_best_overlap_position_quick(&self) -> usize {
        const SCANSTEP: usize = 16;
        const SCANWIND: usize = 8;
        
        let mut best_corr = f32::MIN;
        let mut best_corr2 = f32::MIN;
        let mut best_offs = SCANWIND;
        let mut best_offs2 = SCANWIND;
        
        let input_ptr = self.input_buffer.ptr_begin();
        let mid_buffer = self.mid_buffer.as_slice();
        
        // First pass: scan with SCANSTEP
        let mut i = SCANSTEP;
        let end = self.seek_length.saturating_sub(SCANWIND+1);
        while i < end {
            let offset = i * self.channels;
            let mut norm = 0.0;
            let mut corr = self.calc_cross_corr(&input_ptr[offset..], mid_buffer, &mut norm) as f32;
            
            // Heuristic rule
            let tmp = (2 * i as i64 - self.seek_length as i64 - 1) as f32 / self.seek_length as f32;
            corr = (corr + 0.1) * (1.0 - 0.25 * tmp * tmp);
            
            if corr > best_corr {
                best_corr2 = best_corr;
                best_offs2 = best_offs;
                best_corr = corr;
                best_offs = i;
            } else if corr > best_corr2 {
                best_corr2 = corr;
                best_offs2 = i;
            }
            
            i += SCANSTEP;
        }
        
        // Second pass: refine around best match
        let start = best_offs.saturating_sub(SCANWIND);
        let end = (best_offs + SCANWIND + 1).min(self.seek_length);
        
        for i in start..end {
            if i == best_offs { continue; }
            
            let offset = i * self.channels;
            let mut norm = 0.0;
            let mut corr = self.calc_cross_corr(&input_ptr[offset..], mid_buffer, &mut norm) as f32;
            
            let tmp = (2 * i as i64 - self.seek_length as i64 - 1) as f32 / self.seek_length as f32;
            corr = (corr + 0.1) * (1.0 - 0.25 * tmp * tmp);
            
            if corr > best_corr {
                best_corr = corr;
                best_offs = i;
            }
        }
        
        // Third pass: refine around 2nd best match
        let start = best_offs2.saturating_sub(SCANWIND);
        let end = (best_offs2 + SCANWIND + 1).min(self.seek_length);
        
        for i in start..end {
            if i == best_offs2 { continue; }
            
            let offset = i * self.channels;
            let mut norm = 0.0;
            let mut corr = self.calc_cross_corr(&input_ptr[offset..], mid_buffer, &mut norm) as f32;
            
            let tmp = (2 * i as i64 - self.seek_length as i64 - 1) as f32 / self.seek_length as f32;
            corr = (corr + 0.1) * (1.0 - 0.25 * tmp * tmp);
            
            if corr > best_corr {
                best_corr = corr;
                best_offs = i;
            }
        }
        
        // clear cross correlation routine state if necessary (is so e.g. in MMX routines).
        // clearCrossCorrState();

        best_offs
    }

    /// Overlap samples in midBuffer with the samples in input at position ovl_pos
    fn overlap(&mut self, ovl_pos: usize) {
        let input_ptr = self.input_buffer.ptr_begin();
        let mid_buffer = self.mid_buffer.as_slice();
        let overlap_length = self.overlap_length;
        let overlap_length_s = overlap_length as Sample;
        let channels = self.channels;
        
        // Get output position
        let output_ptr = self.output_buffer.ptr_end(overlap_length);
        
        // Perform overlap based on channel count
        if channels == 1 {
            let mut m1 = 0.0 as Sample;
            let mut m2 = overlap_length as Sample;
            for i in 0..overlap_length {
                output_ptr[i] = (input_ptr[ovl_pos + i] * m1 + mid_buffer[i] * m2) / overlap_length_s;
                m1 += 1.0 as Sample;
                m2 -= 1.0 as Sample;
            }
        } else if channels == 2 {
            let input_start = ovl_pos * 2;
            let mut m1 = 0.0 as Sample;
            let mut m2 = 1.0 as Sample;
            let fscale = 1.0 / overlap_length_s;
            for i in (0..2*overlap_length).step_by(2) {
                output_ptr[i] = input_ptr[input_start + i] * m1 + mid_buffer[i] * m2;
                output_ptr[i + 1] = input_ptr[input_start + i + 1] * m1 + mid_buffer[i + 1] * m2;
                m1 += fscale;
                m2 -= fscale;
            }
        } else {
            let input_start = ovl_pos * channels;
            let mut m1 = 0.0 as Sample;
            let mut m2 = 1.0 as Sample;
            let fscale = 1.0 / overlap_length_s;
            let mut idx = 0;
            for _i in 0..overlap_length {
                for _ch in 0..channels {
                    output_ptr[idx] = input_ptr[input_start + idx] * m1 + mid_buffer[idx] * m2;
                    idx += 1;
                }
                m1 += fscale;
                m2 -= fscale;
            }
        }
    }

    /// Calculate cross-correlation between mixing position and compare buffer
    fn calc_cross_corr(&self, mixing_pos: &[Sample], compare: &[Sample], anorm: &mut f64) -> f64 {
        let mut corr = 0.0_f32;
        let mut norm = 0.0_f32;
        
        // Hint compiler autovectorization that loop length is divisible by 8
        let ilength = (self.channels * self.overlap_length) & !7;
        
        // Same routine for stereo and mono
        for i in 0..ilength {
            corr += mixing_pos[i] * compare[i];
            norm += mixing_pos[i] * mixing_pos[i];
        }
        
        *anorm = norm as f64;
        
        // Normalize result by dividing by sqrt(norm)
        (corr as f64) / ((if norm < 1e-9 { 1.0 } else { norm as f64 }).sqrt())
    }

    /// Calculate cross-correlation with accumulative norm update
    /// This is an optimized version that incrementally updates the norm value
    /// as the search window slides, instead of recalculating it completely.
    /// 
    /// `offset` is the current position in the input buffer (in samples, not including channels)
    fn calc_cross_corr_accumulate(&self, input_buffer: &[Sample], offset: usize, compare: &[Sample], norm: &mut f64) -> f64 {
        // Remove the samples that slid out of the window (from previous position)
        // These are at positions [(offset-1)*channels .. offset*channels]
        // In C++: mixingPos[-channels] .. mixingPos[-1]
        let prev_offset = (offset - 1) * self.channels;
        for i in 0..self.channels {
            let sample = input_buffer[prev_offset + i];
            *norm -= (sample * sample) as f64;
        }
        
        let mut corr = 0.0_f32;
        // Hint compiler autovectorization that loop length is divisible by 8
        let ilength = (self.channels * self.overlap_length) & !7;
        let mixing_offset = offset * self.channels;
        // Calculate correlation (same as calc_cross_corr)
        // mixingPos[0..ilength]
        for i in 0..ilength {
            corr += input_buffer[mixing_offset + i] * compare[i];
        }
        
        // Update normalizer with last samples of this round
        // Add the new samples that slid into the window at the end
        // These are at positions [offset*channels + ilength - channels .. offset*channels + ilength]
        // In C++: mixingPos[ilength-channels] .. mixingPos[ilength-1]
        let new_sample_start = mixing_offset + ilength - self.channels;
        for i in 0..self.channels {
            let sample = input_buffer[new_sample_start + i];
            *norm += (sample * sample) as f64;
        }
        
        // Normalize result by dividing by sqrt(norm)
        (corr as f64) / ((if *norm < 1e-9 { 1.0 } else { *norm }).sqrt())
    }

    /// Clear buffers (public method)
    pub fn clear(&mut self) {
        self.output_buffer.clear();
        self.clear_input();
    }  

    /// Clear input buffer and reset state
    pub fn clear_input(&mut self) {
        self.input_buffer.clear();
        self.clear_mid_buffer();
        self.is_beginning = true;
        self.skip_fract = 0.0;
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

    /// Get input sample requirement
    pub fn get_input_sample_req(&self) -> usize {
        (self.nominal_skip + 0.5) as usize
    }

    /// Get output batch size
    pub fn get_output_batch_size(&self) -> usize {
        self.seek_window_length - self.overlap_length
    }

    /// Get latency
    pub fn get_latency(&self) -> usize {
        self.sample_req
    }
    
    /// Put samples to input (public method)
    pub fn put_samples(&mut self, samples: &[Sample], num_samples: usize) {
        self.input_buffer.put_samples(samples, num_samples);
        self.process();
    }
    
    /// Receive samples from output (public method)
    pub fn receive_samples(&mut self, output: &mut [Sample], max_samples: usize) -> usize {
        self.output_buffer.receive_samples(output, max_samples)
    }
    
    /// Get number of available output samples (public method)
    pub fn num_samples(&self) -> usize {
        self.output_buffer.num_samples()
    }
    
    /// Check if output is empty (public method)
    pub fn is_empty(&self) -> bool {
        self.output_buffer.is_empty()
    }
    
    /// Adjust amount of samples
    pub fn adjust_amount_of_samples(&mut self, num_samples: usize) -> usize {
        self.output_buffer.adjust_amount_of_samples(num_samples)
    }
}

// Implement FIFOSamplePipe trait (for generic usage)
impl FIFOSamplePipe for TDStretch {
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

impl Default for TDStretch {
    fn default() -> Self {
        Self::new()
    }
}

