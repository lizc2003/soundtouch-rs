# SoundTouch-RS

Rust implementation of the SoundTouch audio processing library for changing tempo, pitch, and playback rate of audio streams or files.

## Features

- **Tempo Control**: Change playback speed while maintaining the original pitch
- **Pitch Control**: Change pitch while maintaining the original tempo  
- **Rate Control**: Change both tempo and pitch together
- **BPM Detection**: Detect beats per minute of audio
- High-quality time-domain audio processing algorithms
- Support for mono and stereo audio
- WAV file processing via command-line tool

## Library Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
soundtouch = { git = "https://github.com/lizc2003/soundtouch-rs.git", tag = "v0.1.0" }
```

### Example

```rust
use soundtouch::SoundTouch;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut st = SoundTouch::new();
    
    // Set audio format
    st.set_sample_rate(44100);
    st.set_channels(2)?;
    
    // Adjust audio parameters
    st.set_tempo_change(10.0);  // 10% faster
    st.set_pitch_semi_tones(-2); // 2 semitones lower
    
    // Process samples
    let input_samples: Vec<f32> = vec![/* your audio data */];
    st.put_samples(&input_samples, input_samples.len() / 2)?;
    
    // Receive processed samples
    let mut output = vec![0.0; 8192];
    let received = st.receive_samples(&mut output, 4096);
    
    Ok(())
}
```

## Command-Line Tool

The `soundstretch` binary provides command-line audio processing:

### Build

```bash
cd soundtouch-rs
cargo build --release
```

## Examples

### Lower pitch by one octave
```bash
soundstretch input.wav output.wav -pitch=-12
```

### Speed up by 50% (without changing pitch)
```bash
soundstretch input.wav output.wav -tempo=50
```

### Create slow-motion effect (slower tempo, lower pitch)
```bash
soundstretch input.wav output.wav -rate=-30
```

### High-quality voice processing
```bash
soundstretch voice.wav output.wav -pitch=-2 -speech
```

### Detect BPM and process
```bash
soundstretch song.wav -bpm
```

## Architecture

The library consists of several key components:

- **SoundTouch**: Main processor class that coordinates the pipeline
- **FIFOSampleBuffer**: FIFO buffer for sample management
- **BPMDetect**: BPM detection using autocorrelation

## Testing

Run the test suite:

```bash
cargo test
```

Run benchmarks:

```bash
cargo bench
```

## Performance

The Rust implementation aims to match or exceed the performance of the original C++ library while providing:

- Memory safety without garbage collection
- Zero-cost abstractions
- Modern error handling

## Comparison with C++ Version
| Feature | C++ SoundTouch | SoundTouch-RS |
|---------|---------------|---------------|
| Tempo change | ✓ | ✓ |
| Pitch change | ✓ | ✓ |
| Rate change | ✓ | ✓ |
| BPM detection | ✓ | ✓ |
| SIMD optimizations | ✓ | Planned |
| Integer samples | ✓ | Planned |
| Float samples | ✓ | ✓ |

## License

LGPL-2.1 (same as the original SoundTouch library)

## Credits

This is a Rust port of SoundTouch by Olli Parviainen.

## See Also

- Original C++ SoundTouch: https://www.surina.net/soundtouch/
- SoundTouch documentation and papers
