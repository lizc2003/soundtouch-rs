//! BPM (Beats Per Minute) Detection
//!
//! The beat detection algorithm works as follows:
//! - Use function `input_samples` to input chunks of samples to the class for
//!   analysis. It's a good idea to enter a large sound file or stream in smallish
//!   chunks of around few kilosamples in order not to extinguish too much RAM memory.
//! - Input sound data is decimated to approx 1000 Hz to reduce calculation burden,
//!   which is basically ok as low (bass) frequencies mostly determine the beat rate.
//!   Simple averaging is used for anti-alias filtering because the resulting signal
//!   quality isn't of that high importance.
//! - Decimated sound data is enveloped, i.e. the amplitude shape is detected by
//!   taking absolute value that's smoothed by sliding average. Signal levels that
//!   are below a couple of times the general RMS amplitude level are cut away to
//!   leave only notable peaks there.
//! - Repeating sound patterns (e.g. beats) are detected by calculating short-term
//!   autocorrelation function of the enveloped signal.
//! - After whole sound data file has been analyzed as above, the bpm level is
//!   detected by function `get_bpm` that finds the highest peak of the autocorrelation
//!   function, calculates it's precise location and converts this reading to bpm's.

use crate::fifo_buffer::FIFOSampleBuffer;
use crate::types::Sample;
use std::f32;

/// Minimum allowed BPM rate. Used to restrict accepted result above a reasonable limit.
const MIN_BPM: i32 = 45;

/// Maximum allowed BPM rate range. Used for calculating algorithm parameters
const MAX_BPM_RANGE: i32 = 200;

/// Maximum allowed BPM rate. Used to restrict accepted result below a reasonable limit.
const MAX_BPM_VALID: i32 = 190;

/// Algorithm input sample block size
const INPUT_BLOCK_SIZE: usize = 2048;

/// Decimated sample block size
const DECIMATED_BLOCK_SIZE: usize = 256;

/// Target sample rate after decimation
const TARGET_SRATE: i32 = 1000;

/// XCorr update sequence size, update in about 200msec chunks
const XCORR_UPDATE_SEQUENCE: usize = (TARGET_SRATE / 5) as usize;

/// Moving average N size
const MOVING_AVERAGE_N: usize = 15;

/// XCorr decay time constant, decay to half in 30 seconds
const XCORR_DECAY_TIME_CONSTANT: f64 = 30.0;

/// Data overlap factor for beat detection algorithm
const OVERLAP_FACTOR: usize = 4;

const TWOPI: f64 = 2.0 * std::f64::consts::PI;

/// Beat position and strength
#[derive(Debug, Clone, Copy)]
pub struct Beat {
    /// Position in seconds
    pub pos: f32,
    /// Detection strength
    pub strength: f32,
}

/// 2nd order IIR filter
struct IIR2Filter {
    coeffs: [f64; 5],
    prev: [f64; 5],
}

impl IIR2Filter {
    fn new(coeffs: &[f64; 5]) -> Self {
        IIR2Filter {
            coeffs: *coeffs,
            prev: [0.0; 5],
        }
    }

    fn update(&mut self, xx: f32) -> f32 {
        let x = xx as f64;
        self.prev[0] = x;
        let mut y = x * self.coeffs[0];

        for i in (1..=4).rev() {
            y += self.coeffs[i] * self.prev[i];
            self.prev[i] = self.prev[i - 1];
        }

        self.prev[3] = y;
        y as f32
    }
}

/// Peak finder for BPM detection
struct PeakFinder {
    min_pos: usize,
    max_pos: usize,
}

impl PeakFinder {
    fn new() -> Self {
        PeakFinder {
            min_pos: 0,
            max_pos: 0,
        }
    }

    /// Finds real 'top' of a peak hump from neighbourhood of the given peakpos
    fn find_top(&self, data: &[f32], peakpos: usize) -> Option<usize> {
        let mut refvalue = data[peakpos];
        let mut result = peakpos;

        // seek within ±10 points
        let start = peakpos.saturating_sub(10).max(self.min_pos);
        let end = (peakpos + 10).min(self.max_pos);

        for i in start..=end {
            if data[i] > refvalue {
                result = i;
                refvalue = data[i];
            }
        }

        // failure if max value is at edges of seek range => it's not peak, it's at slope
        if result == start || result == end {
            None
        } else {
            Some(result)
        }
    }

    /// Finds 'ground level' of a peak hump
    fn find_ground(&self, data: &[f32], peakpos: usize, direction: isize) -> usize {
        let mut climb_count = 0;
        let mut refvalue = data[peakpos];
        let mut lowpos = peakpos;
        let mut pos = peakpos as isize;

        while pos > self.min_pos as isize + 1 && pos < self.max_pos as isize - 1 {
            let prevpos = pos;
            pos += direction;

            // calculate derivative
            let delta = data[pos as usize] - data[prevpos as usize];
            if delta <= 0.0 {
                // going downhill, ok
                if climb_count > 0 {
                    climb_count -= 1;
                }

                // check if new minimum found
                if data[pos as usize] < refvalue {
                    lowpos = pos as usize;
                    refvalue = data[pos as usize];
                }
            } else {
                // going uphill, increase climbing counter
                climb_count += 1;
                if climb_count > 5 {
                    break; // we've been climbing too long => it's next uphill => quit
                }
            }
        }
        lowpos
    }

    /// Find offset where the value crosses the given level
    fn find_crossing_level(
        &self,
        data: &[f32],
        level: f32,
        peakpos: usize,
        direction: isize,
    ) -> Option<usize> {
        let mut pos = peakpos as isize;
        while pos >= self.min_pos as isize && (pos + direction) < self.max_pos as isize {
            if data[(pos + direction) as usize] < level {
                return Some(pos as usize);
            }
            pos += direction;
        }
        None
    }

    /// Calculates the center of mass location
    fn calc_mass_center(&self, data: &[f32], first_pos: usize, last_pos: usize) -> f64 {
        let mut sum = 0.0;
        let mut wsum = 0.0;

        for i in first_pos..=last_pos {
            sum += i as f32 * data[i];
            wsum += data[i];
        }

        if wsum < 1e-6 {
            0.0
        } else {
            (sum / wsum) as f64
        }
    }

    /// Get exact center of peak near given position by calculating local mass of center
    fn get_peak_center(&self, data: &[f32], peakpos: usize) -> f64 {
        // find ground positions
        let gp1 = self.find_ground(data, peakpos, -1);
        let gp2 = self.find_ground(data, peakpos, 1);

        let peak_level = data[peakpos];

        let cut_level = if gp1 == gp2 {
            peak_level
        } else {
            let ground_level = 0.5 * (data[gp1] + data[gp2]);
            0.70 * peak_level + 0.30 * ground_level
        };

        // find mid-level crossings
        let crosspos1 = self.find_crossing_level(data, cut_level, peakpos, -1);
        let crosspos2 = self.find_crossing_level(data, cut_level, peakpos, 1);

        match (crosspos1, crosspos2) {
            (Some(cp1), Some(cp2)) => self.calc_mass_center(data, cp1, cp2),
            _ => 0.0,
        }
    }

    /// Detect exact peak position
    fn detect_peak(&mut self, data: &[f32], min_pos: usize, max_pos: usize) -> f64 {
        self.min_pos = min_pos;
        self.max_pos = max_pos;

        // find absolute peak
        let mut peakpos = min_pos;
        let mut peak_val = data[min_pos];
        for i in (min_pos + 1)..max_pos {
            if data[i] > peak_val {
                peak_val = data[i];
                peakpos = i;
            }
        }

        // Calculate exact location of the highest peak mass center
        let high_peak = self.get_peak_center(data, peakpos);
        let mut peak = high_peak;

        // Check if the highest peak were in fact harmonic of the true base beat peak
        for i in 1..3 {
            let harmonic = 2.0_f64.powi(i);
            let harmonic_peakpos = (high_peak / harmonic + 0.5) as usize;
            if harmonic_peakpos < min_pos {
                break;
            }

            if let Some(top_pos) = self.find_top(data, harmonic_peakpos) {
                let peaktmp = self.get_peak_center(data, top_pos);

                // accept harmonic peak if it meets criteria
                let diff = harmonic * peaktmp / high_peak;
                if diff >= 0.96 && diff <= 1.04 {
                    let i1 = (high_peak + 0.5) as usize;
                    let i2 = (peaktmp + 0.5) as usize;
                    if i2 < data.len() && i1 < data.len() && data[i2] >= 0.4 * data[i1] {
                        peak = peaktmp;
                    }
                }
            }
        }

        peak
    }
}

/// BPM Detector
pub struct BPMDetect {
    /// Sample rate
    sample_rate: i32,
    /// Number of channels
    channels: usize,
    /// Auto-correlation accumulator bins
    xcorr: Vec<f32>,
    /// Sample average counter
    decimate_count: usize,
    /// Sample average accumulator for FIFO-like decimation
    decimate_sum: f32,
    /// Decimate sound by this coefficient to reach approx 1000 Hz
    decimate_by: i32,
    /// Auto-correlation window length
    window_len: usize,
    /// Beginning of auto-correlation window
    window_start: usize,
    /// Hamming windows for data preconditioning
    hamw: Vec<f32>,
    hamw2: Vec<f32>,
    /// Beat detection variables
    pos: usize,
    peak_pos: usize,
    beatcorr_ringbuffpos: usize,
    init_scaler: i32,
    peak_val: f32,
    beatcorr_ringbuff: Vec<f32>,
    /// FIFO buffer for decimated processing samples
    buffer: FIFOSampleBuffer,
    /// Collection of detected beat positions
    beats: Vec<Beat>,
    /// 2nd order low-pass filter
    beat_lpf: IIR2Filter,
}

impl BPMDetect {
    /// Create new BPM detector
    pub fn new(num_channels: usize, sample_rate: u32) -> Self {
        // IIR low-pass filter coefficients, calculated with matlab/octave cheby2(2,40,0.05)
        let lpf_coeffs: [f64; 5] = [
            0.00996655391939,
            -0.01944529148401,
            0.00996655391939,
            1.96867605796247,
            -0.96916387431724,
        ];

        let sample_rate = sample_rate as i32;

        // choose decimation factor so that result is approx. 1000 Hz
        let decimate_by = sample_rate / TARGET_SRATE;
        assert!(
            decimate_by > 0 && decimate_by * DECIMATED_BLOCK_SIZE as i32 >= INPUT_BLOCK_SIZE as i32,
            "Sample rate too small"
        );

        // Calculate window length & starting item according to desired min & max bpms
        let window_len = (60 * sample_rate / (decimate_by * MIN_BPM)) as usize;
        let window_start = (60 * sample_rate / (decimate_by * MAX_BPM_RANGE)) as usize;

        assert!(window_len > window_start);

        // Calculate hamming windows
        let hamw = hamming_window(XCORR_UPDATE_SEQUENCE);
        let hamw2 = hamming_window(XCORR_UPDATE_SEQUENCE / 2);

        BPMDetect {
            sample_rate,
            channels: num_channels,
            xcorr: vec![0.0; window_len],
            decimate_count: 0,
            decimate_sum: 0.0,
            decimate_by,
            window_len,
            window_start,
            hamw,
            hamw2,
            pos: 0,
            peak_pos: 0,
            beatcorr_ringbuffpos: 0,
            init_scaler: 1,
            peak_val: 0.0,
            beatcorr_ringbuff: vec![0.0; window_len],
            buffer: FIFOSampleBuffer::new(1).expect("Failed to create FIFO buffer"), // mono processing
            beats: Vec::with_capacity(250),
            beat_lpf: IIR2Filter::new(&lpf_coeffs),
        }
    }

    /// Convert to mono, low-pass filter & decimate to about 1000 Hz
    fn decimate(&mut self, dest: &mut [Sample], src: &[Sample], num_samples: usize) -> usize {
        let mut outcount = 0;
        let mut src_idx = 0;

        for _ in 0..num_samples {
            // convert to mono and accumulate
            for _ in 0..self.channels {
                self.decimate_sum += src[src_idx];
                src_idx += 1;
            }

            self.decimate_count += 1;
            if self.decimate_count >= self.decimate_by as usize {
                // Store every Nth sample only
                let out = self.decimate_sum / (self.decimate_by as f32 * self.channels as f32);
                self.decimate_sum = 0.0;
                self.decimate_count = 0;
                dest[outcount] = out as Sample;
                outcount += 1;
            }
        }
        outcount
    }

    /// Calculates autocorrelation function of the sample history buffer
    fn update_xcorr(&mut self, process_samples: usize) {
        assert!(self.buffer.num_samples() >= process_samples + self.window_len);
        assert!(process_samples == XCORR_UPDATE_SEQUENCE);

        let p_buffer = self.buffer.ptr_begin();

        // calculate decay factor for xcorr filtering
        let xcorr_decay = 0.5_f64.powf(
            process_samples as f64 / (XCORR_DECAY_TIME_CONSTANT * TARGET_SRATE as f64),
        ) as f32;

        // prescale pbuffer
        let mut tmp = [0.0; XCORR_UPDATE_SEQUENCE];
        for i in 0..process_samples {
            tmp[i] = self.hamw[i] * self.hamw[i] * p_buffer[i];
        }

        for offs in self.window_start..self.window_len {
            let mut sum = 0.0;
            for i in 0..process_samples {
                sum += tmp[i] * p_buffer[i + offs];
            }
            self.xcorr[offs] *= xcorr_decay;
            self.xcorr[offs] += sum.abs();
        }
    }

    /// Detect individual beat positions
    fn update_beat_pos(&mut self, process_samples: usize) {
        assert!(self.buffer.num_samples() >= process_samples + self.window_len);
        assert!(process_samples == XCORR_UPDATE_SEQUENCE / 2);

        let p_buffer = self.buffer.ptr_begin();
        let pos_scale = self.decimate_by as f64 / self.sample_rate as f64;
        let reset_dur = (0.12 / pos_scale + 0.5) as usize;

        // prescale pbuffer
        let mut tmp = [0.0; XCORR_UPDATE_SEQUENCE / 2];
        for i in 0..process_samples {
            tmp[i] = self.hamw2[i] * self.hamw2[i] * p_buffer[i];
        }

        for offs in self.window_start..self.window_len {
            let mut sum = 0.0;
            for i in 0..process_samples {
                sum += tmp[i] * p_buffer[offs + i];
            }
            if sum > 0.0 {
                let idx = (self.beatcorr_ringbuffpos + offs) % self.window_len;
                self.beatcorr_ringbuff[idx] += sum;
            }
        }

        let skipstep = XCORR_UPDATE_SEQUENCE / OVERLAP_FACTOR;

        // compensate empty buffer at beginning by scaling coefficient
        let mut scale = self.window_len as f32 / (skipstep as f32 * self.init_scaler as f32);
        if scale > 1.0 {
            self.init_scaler += 1;
        } else {
            scale = 1.0;
        }

        // detect beats
        for _ in 0..skipstep {
            let mut sum = self.beatcorr_ringbuff[self.beatcorr_ringbuffpos];
            sum -= self.beat_lpf.update(sum);

            if sum > self.peak_val {
                // found new local largest value
                self.peak_val = sum;
                self.peak_pos = self.pos;
            }
            if self.pos > self.peak_pos + reset_dur {
                // largest value not updated for ~120msec => accept as beat
                self.peak_pos += skipstep;
                if self.peak_val > 0.0 {
                    // add detected beat to end of "beats" vector
                    self.beats.push(Beat {
                        pos: (self.peak_pos as f64 * pos_scale) as f32,
                        strength: self.peak_val * scale,
                    });
                }

                self.peak_val = 0.0;
                self.peak_pos = self.pos;
            }

            self.beatcorr_ringbuff[self.beatcorr_ringbuffpos] = 0.0;
            self.pos += 1;
            self.beatcorr_ringbuffpos = (self.beatcorr_ringbuffpos + 1) % self.window_len;
        }
    }

    /// Input samples for analysis
    pub fn input_samples(&mut self, samples: &[Sample], mut num_samples: usize) {
        let mut sample_idx = 0;
        let mut decimated = [0.0; DECIMATED_BLOCK_SIZE];

        // iterate so that max INPUT_BLOCK_SIZE processed per iteration
        while num_samples > 0 {
            let block = num_samples.min(INPUT_BLOCK_SIZE);

            // decimate - note that converts to mono at the same time
            let dec_samples =
                self.decimate(&mut decimated, &samples[sample_idx..], block);
            sample_idx += block * self.channels;
            num_samples -= block;

            self.buffer.put_samples(&decimated, dec_samples);
        }

        // when the buffer has enough samples for processing...
        let req = self
            .window_len
            .max(2 * XCORR_UPDATE_SEQUENCE)
            .max(self.window_len + XCORR_UPDATE_SEQUENCE);
        while self.buffer.num_samples() >= req {
            // ... update autocorrelations...
            self.update_xcorr(XCORR_UPDATE_SEQUENCE);
            // ...update beat position calculation...
            self.update_beat_pos(XCORR_UPDATE_SEQUENCE / 2);
            // ... and remove processed samples from the buffer
            let n = XCORR_UPDATE_SEQUENCE / OVERLAP_FACTOR;
            self.buffer.receive_samples_no_copy(n);
        }
    }

    /// Remove linear bias from xcorr data
    fn remove_bias(&mut self) {
        // Calculate mean of 'xcorr' and 'i'
        let mut mean_x = 0.0;
        for i in self.window_start..self.window_len {
            mean_x += self.xcorr[i] as f64;
        }
        mean_x /= (self.window_len - self.window_start) as f64;
        let mean_i = 0.5 * (self.window_len - 1 + self.window_start) as f64;

        // Calculate linear regression coefficient
        let mut b = 0.0;
        let mut div = 0.0;
        for i in self.window_start..self.window_len {
            let xt = self.xcorr[i] as f64 - mean_x;
            let xi = i as f64 - mean_i;
            b += xt * xi;
            div += xi * xi;
        }
        b /= div;

        // Subtract linear regression and resolve min. value bias
        let mut minval = f32::MAX;
        for i in self.window_start..self.window_len {
            self.xcorr[i] -= (b * i as f64) as f32;
            if self.xcorr[i] < minval {
                minval = self.xcorr[i];
            }
        }

        // Subtract min.value
        for i in self.window_start..self.window_len {
            self.xcorr[i] -= minval;
        }
    }

    /// Analyzes the results and returns the BPM rate
    pub fn get_bpm(&mut self) -> f32 {
        // remove bias from xcorr data
        self.remove_bias();

        let coeff = 60.0 * (self.sample_rate as f64 / self.decimate_by as f64);

        // Smoothen by N-point moving-average
        let data = ma_filter(&self.xcorr, self.window_start, self.window_len, MOVING_AVERAGE_N);

        // find peak position
        let mut peak_finder = PeakFinder::new();
        let peak_pos = peak_finder.detect_peak(&data, self.window_start, self.window_len);

        if peak_pos < 1e-9 {
            return 0.0; // detection failed
        }

        // calculate BPM
        let bpm = (coeff / peak_pos) as f32;
        if bpm >= MIN_BPM as f32 && bpm <= MAX_BPM_VALID as f32 {
            bpm
        } else {
            0.0
        }
    }

    /// Get beat position arrays
    pub fn get_beats(&self) -> &[Beat] {
        &self.beats
    }
}

/// Calculate Hamming window
fn hamming_window(n: usize) -> Vec<f32> {
    let mut w = vec![0.0; n];
    for i in 0..n {
        w[i] = (0.54 - 0.46 * (TWOPI * i as f64 / (n - 1) as f64).cos()) as f32;
    }
    w
}

/// Calculate N-point moving average
fn ma_filter(source: &[f32], start: usize, end: usize, n: usize) -> Vec<f32> {
    let mut dest = vec![0.0; end];

    for i in start..end {
        let i1 = i.saturating_sub(n / 2).max(start);
        let i2 = (i + n / 2 + 1).min(end);

        let mut sum = 0.0;
        for j in i1..i2 {
            sum += source[j] as f64;
        }
        dest[i] = (sum / (i2 - i1) as f64) as f32;
    }

    dest
}

