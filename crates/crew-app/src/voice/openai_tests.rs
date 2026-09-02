//! The two request bodies, checked without a network. A multipart body built by hand is exactly
//! the thing that fails silently — a missing CRLF is a 400 with no clue in it.
use super::*;

fn as_text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[test]
fn the_transcription_form_carries_the_model_and_the_file() {
    let form = transcription_form(b"RIFFfake", "whisper-1");
    let text = as_text(&form.body);
    assert!(form
        .content_type
        .starts_with("multipart/form-data; boundary="));
    let boundary = form.content_type.rsplit('=').next().unwrap();
    assert!(text.contains(&format!("--{boundary}\r\n")), "{text}");
    assert!(
        text.contains("name=\"model\"\r\n\r\nwhisper-1\r\n"),
        "{text}"
    );
    assert!(
        text.contains("name=\"file\"; filename=\"speech.wav\""),
        "{text}"
    );
    assert!(text.contains("Content-Type: audio/wav"), "{text}");
    assert!(
        text.ends_with(&format!("--{boundary}--\r\n")),
        "closing boundary: {text}"
    );
}

#[test]
fn the_audio_crosses_the_wire_byte_for_byte() {
    // A file mangled on the way out is a transcript of nothing.
    let wav = crate::voice::wav::encode(&[1, -1, 300, -300], 16_000);
    let form = transcription_form(&wav, "whisper-1");
    let start = form
        .body
        .windows(4)
        .position(|w| w == b"RIFF")
        .expect("the file is in the body");
    assert_eq!(&form.body[start..start + wav.len()], wav.as_slice());
}

#[test]
fn a_plain_text_transcript_is_asked_for_rather_than_json() {
    // The response IS the transcript, so nothing has to parse a shape that may change.
    let text = as_text(&transcription_form(b"x", "whisper-1").body);
    assert!(
        text.contains("name=\"response_format\"\r\n\r\ntext\r\n"),
        "{text}"
    );
}

#[test]
fn the_speech_body_asks_for_raw_samples() {
    // `pcm` rather than mp3 is the reason crew needs no audio decoder to say a sentence.
    let body = speech_body("hello", "gpt-4o-mini-tts", "alloy");
    assert_eq!(body["response_format"], "pcm");
    assert_eq!(body["model"], "gpt-4o-mini-tts");
    assert_eq!(body["voice"], "alloy");
    assert_eq!(body["input"], "hello");
}

#[test]
fn pcm_bytes_become_samples_and_half_a_sample_is_dropped() {
    assert_eq!(pcm_from_bytes(&[0x01, 0x02, 0xff, 0xff]), [0x0201, -1]);
    assert_eq!(
        pcm_from_bytes(&[0x01, 0x02, 0x03]),
        [0x0201],
        "a trailing odd byte is not half a sample"
    );
    assert!(pcm_from_bytes(&[]).is_empty());
}
