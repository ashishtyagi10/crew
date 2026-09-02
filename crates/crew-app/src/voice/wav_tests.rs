//! A malformed header is a 400 from Whisper with no clue in it, so the bytes are checked here.
use super::*;

fn read_u32(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

#[test]
fn the_header_says_what_the_file_is() {
    let wav = encode(&[1, -1, 2, -2], 16_000);
    assert_eq!(&wav[0..4], b"RIFF");
    assert_eq!(&wav[8..12], b"WAVE");
    assert_eq!(&wav[12..16], b"fmt ");
    assert_eq!(read_u32(&wav, 16), 16, "PCM fmt chunk length");
    assert_eq!(u16::from_le_bytes([wav[20], wav[21]]), 1, "uncompressed");
    assert_eq!(u16::from_le_bytes([wav[22], wav[23]]), 1, "mono");
    assert_eq!(read_u32(&wav, 24), 16_000, "sample rate");
    assert_eq!(read_u32(&wav, 28), 32_000, "byte rate = rate × 2 bytes");
    assert_eq!(u16::from_le_bytes([wav[34], wav[35]]), 16, "bits");
    assert_eq!(&wav[36..40], b"data");
}

#[test]
fn every_length_field_agrees_with_the_file_that_arrives() {
    // The two sizes in a RIFF header are the two things a reader trusts; one wrong is a file
    // that plays half a second of noise.
    let pcm: Vec<i16> = (0..1000).map(|i| i as i16).collect();
    let wav = encode(&pcm, 16_000);
    assert_eq!(wav.len(), HEADER + pcm.len() * 2);
    assert_eq!(read_u32(&wav, 4) as usize, wav.len() - 8, "RIFF size");
    assert_eq!(read_u32(&wav, 40) as usize, pcm.len() * 2, "data size");
}

#[test]
fn the_samples_survive_the_trip_little_endian() {
    let wav = encode(&[0x0102, -2], 16_000);
    assert_eq!(&wav[HEADER..HEADER + 2], &[0x02, 0x01]);
    assert_eq!(&wav[HEADER + 2..HEADER + 4], &(-2i16).to_le_bytes());
}

#[test]
fn an_empty_recording_is_still_a_valid_file() {
    let wav = encode(&[], 16_000);
    assert_eq!(wav.len(), HEADER);
    assert_eq!(read_u32(&wav, 40), 0);
}

#[test]
fn duration_is_the_length_a_person_would_say_it_was() {
    assert_eq!(duration_ms(&vec![0; 16_000], 16_000), 1_000);
    assert_eq!(duration_ms(&vec![0; 8_000], 16_000), 500);
    assert_eq!(duration_ms(&[], 16_000), 0);
    assert_eq!(duration_ms(&[1, 2], 0), 0, "no rate, no division by zero");
}

#[test]
fn silence_is_not_speech_and_a_voice_is() {
    // Sending silence costs a round trip and comes back as Whisper's best guess at what silence
    // says — which is a real phenomenon and reads as crew hearing voices.
    assert!(!has_speech(&[]));
    assert!(!has_speech(&vec![0; 16_000]));
    assert!(!has_speech(&vec![80; 16_000]), "a quiet room is not speech");
    let tone: Vec<i16> = (0..16_000)
        .map(|i| ((i as f32 / 8.0).sin() * 6_000.0) as i16)
        .collect();
    assert!(has_speech(&tone));
}
