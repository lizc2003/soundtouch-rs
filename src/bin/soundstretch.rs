//! SoundStretch command-line tool (C++ compatible version)
//!
//! Compatible with original C++ soundstretch parameter format

use hound::{WavReader, WavWriter, WavSpec};
use soundtouch::soundtouch::SoundTouch;
use soundtouch::bpm_detect::BPMDetect;
use std::env;
use std::path::PathBuf;

struct Options {
    input: Option<PathBuf>,
    output: Option<PathBuf>,
    pitch: Option<f64>,
    tempo: Option<f64>,
    rate: Option<f64>,
    detect_bpm: bool,
    goal_bpm: Option<f64>,
    quick: bool,
    no_anti_alias: bool,
    speech: bool,
    show_license: bool,
}

impl Options {
    fn new() -> Self {
        Options {
            input: None,
            output: None,
            pitch: None,
            tempo: None,
            rate: None,
            detect_bpm: false,
            goal_bpm: None,
            quick: false,
            no_anti_alias: false,
            speech: false,
            show_license: false,
        }
    }
}

fn parse_args() -> Result<Options, String> {
    let args: Vec<String> = env::args().collect();
    let mut opts = Options::new();
    
    if args.len() < 3 {
        if args.len() == 2 && args[1].starts_with("-l") {
            opts.show_license = true;
            return Ok(opts);
        }
        return Err("Too few parameters".to_string());
    }
    
    // First argument is always the input file
    opts.input = Some(PathBuf::from(&args[1]));
    
    // Second argument is output file or a switch
    let mut param_start = 2;
    if args.len() > 2 {
        if args[2].starts_with('-') {
            // Output file omitted, switches start at position 2
            opts.output = None;
            param_start = 2;
        } else {
            // Output file specified
            opts.output = Some(PathBuf::from(&args[2]));
            param_start = 3;
        }
    }
    
    // Parse switches
    for i in param_start..args.len() {
        let arg = &args[i];
        
        if !arg.starts_with('-') {
            return Err(format!("Unexpected argument: {}", arg));
        }
        
        // Parse switch parameter
        let switch_lower = arg[1..].to_lowercase();
        let first_char = switch_lower.chars().next().unwrap_or(' ');
        
        match first_char {
            't' => {
                // -tempo=xx
                opts.tempo = Some(parse_switch_value(arg)?);
            }
            'p' => {
                // -pitch=xx
                opts.pitch = Some(parse_switch_value(arg)?);
            }
            'r' => {
                // -rate=xx
                opts.rate = Some(parse_switch_value(arg)?);
            }
            'b' => {
                // -bpm or -bpm=xx
                opts.detect_bpm = true;
                if arg.contains('=') {
                    match parse_switch_value(arg) {
                        Ok(bpm) => opts.goal_bpm = Some(bpm),
                        Err(_) => opts.goal_bpm = None,
                    }
                }
            }
            'q' => {
                // -quick
                opts.quick = true;
            }
            'n' => {
                // -naa (no anti-alias)
                opts.no_anti_alias = true;
            }
            's' => {
                // -speech
                opts.speech = true;
            }
            'l' => {
                // -license
                opts.show_license = true;
            }
            _ => {
                return Err(format!("Unknown switch: {}", arg));
            }
        }
    }
    
    // Check limits
    if let Some(tempo) = opts.tempo {
        if tempo < -95.0 || tempo > 5000.0 {
            return Err("Tempo change out of range (-95..5000)".to_string());
        }
    }
    if let Some(pitch) = opts.pitch {
        if pitch < -60.0 || pitch > 60.0 {
            return Err("Pitch change out of range (-60..60)".to_string());
        }
    }
    if let Some(rate) = opts.rate {
        if rate < -95.0 || rate > 5000.0 {
            return Err("Rate change out of range (-95..5000)".to_string());
        }
    }
    
    Ok(opts)
}

fn parse_switch_value(switch_str: &str) -> Result<f64, String> {
    if let Some(pos) = switch_str.find('=') {
        let value_str = &switch_str[pos + 1..];
        value_str.parse().map_err(|_| format!("Invalid value in switch: {}", switch_str))
    } else {
        Err(format!("Missing '=' in switch: {}", switch_str))
    }
}

const BUFFER_SIZE: usize = 6720;

const LICENSE_TEXT: &str = r#"LICENSE:
========

SoundTouch sound processing library
Copyright (c) Olli Parviainen

This library is free software; you can redistribute it and/or
modify it under the terms of the GNU Lesser General Public
License version 2.1 as published by the Free Software Foundation.

This library is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU
Lesser General Public License for more details.

You should have received a copy of the GNU Lesser General Public
License along with this library; if not, write to the Free Software
Foundation, Inc., 59 Temple Place, Suite 330, Boston, MA  02111-1307  USA

This application is distributed with full source codes; however, if you
didn't receive them, please visit the author's homepage."#;

const USAGE_TEXT: &str = r#"This application processes WAV audio files by modifying the sound tempo,
pitch and playback rate properties independently from each other.

Usage :
    soundstretch infilename outfilename [switches]

To use standard input/output pipes, give 'stdin' and 'stdout' as filenames.

Available switches are:
  -tempo=n : Change sound tempo by n percents  (n=-95..+5000 %)
  -pitch=n : Change sound pitch by n semitones (n=-60..+60 semitones)
  -rate=n  : Change sound rate by n percents   (n=-95..+5000 %)
  -bpm=n   : Detect the BPM rate of sound and adjust tempo to meet 'n' BPMs.
             If '=n' is omitted, just detects the BPM rate.
  -quick   : Use quicker tempo change algorithm (gain speed, lose quality)
  -naa     : Don't use anti-alias filtering (gain speed, lose quality)
  -speech  : Tune algorithm for speech processing (default is for music)
  -license : Display the program license text (LGPL)
"#;

fn print_hello() {
    eprintln!();
    eprintln!("   SoundStretch v{} - Copyright (c) Olli Parviainen", SoundTouch::version());
    eprintln!("=========================================================");
    eprintln!("author e-mail: <oparviai@iki.fi> - WWW: http://www.surina.net/soundtouch");
    eprintln!();
    eprintln!("This program is subject to (L)GPL license. Run \"soundstretch -license\" for");
    eprintln!("more information.");
    eprintln!();
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    print_hello();
    
    let opts = match parse_args() {
        Ok(o) => o,
        Err(e) => {
            if e.contains("Too few") {
                eprintln!("{}", USAGE_TEXT);
            } else {
                eprintln!("ERROR: {}", e);
                eprintln!();
                eprintln!("{}", USAGE_TEXT);
            }
            std::process::exit(1);
        }
    };

    // Show license if requested
    if opts.show_license {
        println!("{}", LICENSE_TEXT);
        return Ok(());
    }

    let input_path = opts.input.as_ref().unwrap();

    // Open input file
    let mut reader = WavReader::open(input_path)?;
    let spec = reader.spec();
    
    eprintln!("Input file: {:?}", input_path);
    eprintln!("  Channels: {}", spec.channels);
    eprintln!("  Sample rate: {} Hz", spec.sample_rate);
    eprintln!("  Bits per sample: {}", spec.bits_per_sample);
    eprintln!("  Duration: {:.2} seconds", reader.duration() as f64 / spec.sample_rate as f64);
    eprintln!();

    // BPM detection mode
    let mut tempo_delta = opts.tempo;
    if opts.detect_bpm {
        let detected_bpm = detect_bpm(&mut reader, &spec)?;
        
        // Adjust tempo if goal BPM is specified
        if let Some(goal_bpm) = opts.goal_bpm {
            if detected_bpm > 0.0 {
                tempo_delta = Some((goal_bpm / detected_bpm as f64 - 1.0) * 100.0);
                eprintln!("The file will be converted to {:.1} BPM", goal_bpm);
                eprintln!();
            }
        }
        
        // If no output file specified, just detect BPM and exit
        if opts.output.is_none() {
            return Ok(());
        }
        
        // Reopen file for processing
        reader = WavReader::open(input_path)?;
    }

    // Check if output file is specified
    let output_path = match &opts.output {
        Some(path) => path,
        None => {
            eprintln!("Warning: output file name missing, won't output anything.");
            eprintln!();
            return Ok(());
        }
    };

    // Create SoundTouch processor
    let mut st = SoundTouch::new();
    st.set_sample_rate(spec.sample_rate);
    st.set_channels(spec.channels as usize)?;

    // Apply parameters
    let pitch_delta = opts.pitch.unwrap_or(0.0);
    let tempo_delta = tempo_delta.unwrap_or(0.0);
    let rate_delta = opts.rate.unwrap_or(0.0);
    
    st.set_pitch_semi_tones_float(pitch_delta);
    st.set_tempo_change(tempo_delta);
    st.set_rate_change(rate_delta);

    // Apply quality settings
    st.set_setting(soundtouch::types::SettingId::UseQuickSeek, if opts.quick { 1 } else { 0 });
    st.set_setting(soundtouch::types::SettingId::UseAAFilter, if opts.no_anti_alias { 0 } else { 1 });

    if opts.speech {
        // Use settings for speech processing
        st.set_setting(soundtouch::types::SettingId::SequenceMs, 40);
        st.set_setting(soundtouch::types::SettingId::SeekWindowMs, 15);
        st.set_setting(soundtouch::types::SettingId::OverlapMs, 8);
        eprintln!("Tune processing parameters for speech processing.");
    }

    // Print processing information
    eprintln!("Uses 32bit floating point sample type in processing.");
    eprintln!();
    
    eprintln!("Processing the file with the following changes:");
    eprintln!("  tempo change = {:+.1} %", tempo_delta);
    eprintln!("  pitch change = {:+.1} semitones", pitch_delta);
    eprintln!("  rate change  = {:+.1} %", rate_delta);
    eprintln!();
    eprintln!("Working...");

    // Process audio
    process_audio(&mut reader, &spec, &mut st, output_path)?;

    eprintln!("Done!");

    Ok(())
}

fn detect_bpm(reader: &mut WavReader<std::io::BufReader<std::fs::File>>, spec: &WavSpec) -> Result<f32, hound::Error> {
    eprint!("Detecting BPM rate...");
    
    let channels = spec.channels as usize;
    let mut bpm_detect = BPMDetect::new(channels, spec.sample_rate);
    let mut buffer = [0.0_f32; BUFFER_SIZE];
    
    // Round read size down to multiple of num.channels
    let read_size = BUFFER_SIZE - BUFFER_SIZE % channels;
    let mut samples_iter = reader.samples::<i16>();

    loop {
        let mut eof = false;
        let mut num_samples = read_size;
        for i in 0..read_size {
            match samples_iter.next() {
                Some(Ok(sample)) => buffer[i] = sample as f32 / 32768.0,
                Some(Err(e)) => return Err(e),
                None => {
                    eof = true;
                    num_samples = i;
                    break;
                }
            }
        }
        
        if num_samples > 0 {
            num_samples /= channels;
            bpm_detect.input_samples(&buffer, num_samples);
        }
        if eof {
            break;
        }
    }

    let bpm = bpm_detect.get_bpm();
    eprintln!("Done!");

    if bpm > 0.0 {
        eprintln!("Detected BPM rate {:.1}", bpm);
        eprintln!();
    } else {
        eprintln!("Couldn't detect BPM rate.");
        eprintln!();
    }
    
    Ok(bpm)
}

fn process_audio(
    reader: &mut WavReader<std::io::BufReader<std::fs::File>>,
    spec: &WavSpec,
    st: &mut SoundTouch,
    output_path: &PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    // Create output writer
    let mut writer = WavWriter::create(output_path, *spec)?;

    // Use buffer size similar to C++ version (6720 is divisible by many channel counts)
    let channels = spec.channels as usize;

    let mut buffer = [0.0_f32; BUFFER_SIZE];
    let buff_size_samples = BUFFER_SIZE / channels;
    
    // Round read size down to multiple of num.channels
    let read_size = BUFFER_SIZE - BUFFER_SIZE % channels;
    let mut samples_iter = reader.samples::<i16>();

    loop {
        let mut eof = false;
        let mut num_samples = read_size;
        for i in 0..read_size {
            match samples_iter.next() {
                Some(Ok(sample)) => buffer[i] = sample as f32 / 32768.0,
                Some(Err(e)) => return Err(Box::new(e)),
                None => {
                    eof = true;
                    num_samples = i;
                    break;
                }
            }
        }
        
        if num_samples > 0 {
            num_samples /= channels;
            st.put_samples(&buffer, num_samples)?;
        
            // Read ready samples from SoundTouch processor & write them to output file
            loop {
                let n_samples = st.receive_samples(&mut buffer, buff_size_samples);
                if n_samples == 0 {
                    break;
                }
                
                // Write to output file
                for i in 0..(n_samples * channels) {
                    let sample = (buffer[i] * 32768.0).clamp(-32768.0, 32767.0) as i16;
                    writer.write_sample(sample)?;
                }
            }
        }
        if eof {
            break;
        }
    }
    
    // Flush few last samples that are hiding in the SoundTouch's internal processing pipeline
    st.flush();
    loop {
        let n_samples = st.receive_samples(&mut buffer, buff_size_samples);
        if n_samples == 0 {
            break;
        }
        
        for i in 0..(n_samples * channels) {
            let sample = (buffer[i] * 32768.0).clamp(-32768.0, 32767.0) as i16;
            writer.write_sample(sample)?;
        }
    }
    
    // Finalize the writer to update WAV headers and close the file
    writer.finalize()?;
    
    Ok(())
}

