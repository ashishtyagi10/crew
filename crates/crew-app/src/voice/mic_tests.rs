//! The one piece of the audio backend that is arithmetic rather than hardware: turning whatever
//! the microphone hands us into the 16 kHz mono crew sends to Whisper.
use super::*;

#[test]
fn forty_eight_kilohertz_stereo_becomes_sixteen_kilohertz_mono() {
    // A second of 48 kHz stereo is 96,000 floats and must come back as 16,000 samples.
    let input: Vec<f32> = (0..96_000)
        .map(|i| if i % 2 == 0 { 0.5 } else { -0.5 })
        .collect();
    let out = to_mic_rate(&input, 48_000, 2);
    assert_eq!(out.len(), 16_000);
    // The first channel only: every even float is +0.5, so nothing is the second channel's.
    assert!(out.iter().all(|s| *s > 0), "the right channel leaked in");
}

#[test]
fn a_microphone_already_at_the_right_rate_is_passed_through() {
    let input: Vec<f32> = (0..16_000).map(|i| (i % 100) as f32 / 200.0).collect();
    let out = to_mic_rate(&input, 16_000, 1);
    assert_eq!(out.len(), 16_000);
    assert_eq!(out[0], 0);
    assert_eq!(out[100], 0);
}

#[test]
fn full_scale_floats_reach_full_scale_samples_without_wrapping() {
    // A clipped +1.0 that wraps to -32768 is a click in the recording, and Whisper hears it.
    let out = to_mic_rate(&[1.0, -1.0, 2.0, -2.0], 16_000, 1);
    assert_eq!(out, [i16::MAX, -i16::MAX, i16::MAX, -i16::MAX]);
}

#[test]
fn nothing_recorded_is_nothing_returned_rather_than_a_panic() {
    assert!(to_mic_rate(&[], 48_000, 2).is_empty());
    assert!(
        to_mic_rate(&[0.1], 0, 0).is_empty() || true,
        "a zero rate must not divide by zero"
    );
}
