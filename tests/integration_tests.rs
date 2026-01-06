//! Integration tests for SoundTouch

use soundtouch::SoundTouch;

#[test]
fn test_basic_processing() {
    let mut st = SoundTouch::new();
    
    st.set_sample_rate(44100);
    assert!(st.set_channels(2).is_ok());
    
    st.set_tempo_change(10.0);
    st.set_pitch_semi_tones(-3);
    
    // Generate test samples
    let input: Vec<f32> = (0..8800).map(|i| (i as f32 * 0.1).sin() * 0.5).collect();
    
    // Process
    assert!(st.put_samples(&input, 4400).is_ok());
    
    // Should have some output
    assert!(st.num_samples() > 0);
}

#[test]
fn test_tempo_change() {
    let mut st = SoundTouch::new();
    st.set_sample_rate(44100);
    st.set_channels(2).unwrap();
    
    // Test various tempo changes
    st.set_tempo_change(50.0);
    st.set_tempo_change(-25.0);
    st.set_tempo(1.5);
    st.set_tempo(0.8);
}

#[test]
fn test_pitch_change() {
    let mut st = SoundTouch::new();
    st.set_sample_rate(44100);
    st.set_channels(2).unwrap();
    
    // Test various pitch changes
    st.set_pitch_semi_tones(5);
    st.set_pitch_semi_tones(-7);
    st.set_pitch_octaves(1.0);
    st.set_pitch_octaves(-0.5);
}

#[test]
fn test_rate_change() {
    let mut st = SoundTouch::new();
    st.set_sample_rate(44100);
    st.set_channels(2).unwrap();
    
    st.set_rate_change(20.0);
    st.set_rate(1.3);
}

#[test]
fn test_clear() {
    let mut st = SoundTouch::new();
    st.set_sample_rate(44100);
    st.set_channels(2).unwrap();
    
    let input: Vec<f32> = vec![0.5; 8800];
    st.put_samples(&input, 4400).unwrap();
    
    assert!(st.num_samples() > 0 || st.num_unprocessed_samples() > 0);
    
    st.clear();
    assert_eq!(st.num_samples(), 0);
}

#[test]
fn test_settings() {
    let mut st = SoundTouch::new();
    st.set_sample_rate(44100);
    st.set_channels(2).unwrap();
    
    // Test sequence setting
    assert!(st.set_setting(soundtouch::types::SettingId::SequenceMs, 100));
    let seq = st.get_setting(soundtouch::types::SettingId::SequenceMs);
    assert_eq!(seq, 100);
    
    // Test quick seek
    assert!(st.set_setting(soundtouch::types::SettingId::UseQuickSeek, 1));
    let quick = st.get_setting(soundtouch::types::SettingId::UseQuickSeek);
    assert_eq!(quick, 1);
}

#[test]
fn test_mono_audio() {
    let mut st = SoundTouch::new();
    st.set_sample_rate(44100);
    assert!(st.set_channels(1).is_ok());
    
    let input: Vec<f32> = (0..4400).map(|i| (i as f32 * 0.1).sin()).collect();
    assert!(st.put_samples(&input, 4400).is_ok());
}

#[test]
fn test_stereo_audio() {
    let mut st = SoundTouch::new();
    st.set_sample_rate(44100);
    assert!(st.set_channels(2).is_ok());
    
    let input: Vec<f32> = (0..8800).map(|i| (i as f32 * 0.1).sin()).collect();
    assert!(st.put_samples(&input, 4400).is_ok());
}

#[test]
fn test_invalid_channels() {
    let mut st = SoundTouch::new();
    assert!(st.set_channels(0).is_err());
    assert!(st.set_channels(100).is_err());
}

#[test]
fn test_process_without_setup() {
    let mut st = SoundTouch::new();
    let input: Vec<f32> = vec![0.5; 100];
    
    // Should fail - sample rate not set
    assert!(st.put_samples(&input, 50).is_err());
    
    st.set_sample_rate(44100);
    // Should fail - channels not set
    assert!(st.put_samples(&input, 50).is_err());
}

#[test]
fn test_flush() {
    let mut st = SoundTouch::new();
    st.set_sample_rate(44100);
    st.set_channels(2).unwrap();
    
    let input: Vec<f32> = vec![0.5; 8800];
    st.put_samples(&input, 4400).unwrap();
    
    st.flush();
    
    // After flush, should have output samples ready
    let mut output = vec![0.0; 8800];
    let _received = st.receive_samples(&mut output, 4400);
}

#[test]
fn test_input_output_ratio() {
    let mut st = SoundTouch::new();
    st.set_sample_rate(44100);
    st.set_channels(2).unwrap();
    
    st.set_tempo_change(50.0); // 50% faster
    let ratio = st.get_input_output_sample_ratio();
    
    // Output should be shorter (faster tempo)
    assert!(ratio < 1.0);
}

#[test]
fn test_version() {
    let version = SoundTouch::version();
    assert!(!version.is_empty());
    
    let version_id = SoundTouch::version_id();
    assert!(version_id > 0);
}

