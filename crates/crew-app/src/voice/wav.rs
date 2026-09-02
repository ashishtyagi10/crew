//! PCM in, a WAV file out.
//!
//! Whisper is handed a file, not a buffer of samples, and the smallest honest file for 16-bit
//! mono PCM is a 44-byte RIFF header in front of the samples. Written by hand rather than with a
//! crate because that is genuinely all it is, and because a header this test can read back is
//! worth more here than a dependency.
/// The header is 44 bytes: `RIFF` size `WAVE`, a 16-byte PCM `fmt ` chunk, and `data` size.
pub(crate) const HEADER: usize = 44;

/// One channel. crew records a person at a keyboard, not a room.
const CHANNELS: u16 = 1;
/// 16-bit samples — what every speech API takes and what `cpal` gives us.
const BITS: u16 = 16;

/// Wrap `pcm` (16-bit mono, `rate` Hz) as a WAV file.
pub(crate) fn encode(pcm: &[i16], rate: u32) -> Vec<u8> {
    let data_len = (pcm.len() * 2) as u32;
    let byte_rate = rate * u32::from(CHANNELS) * u32::from(BITS / 8);
    let block_align = CHANNELS * (BITS / 8);
    let mut out = Vec::with_capacity(HEADER + pcm.len() * 2);
    out.extend_from_slice(b"RIFF");
    // Everything after this field: 4 (`WAVE`) + 24 (fmt chunk) + 8 (data header) + the samples.
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&16u32.to_le_bytes()); // PCM fmt chunk length
    out.extend_from_slice(&1u16.to_le_bytes()); // 1 = uncompressed PCM
    out.extend_from_slice(&CHANNELS.to_le_bytes());
    out.extend_from_slice(&rate.to_le_bytes());
    out.extend_from_slice(&byte_rate.to_le_bytes());
    out.extend_from_slice(&block_align.to_le_bytes());
    out.extend_from_slice(&BITS.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for s in pcm {
        out.extend_from_slice(&s.to_le_bytes());
    }
    out
}

/// How long `pcm` lasts, in milliseconds — what a "did anybody actually say anything" check and
/// a log line both want.
pub(crate) fn duration_ms(pcm: &[i16], rate: u32) -> u64 {
    match rate {
        0 => 0,
        r => (pcm.len() as u64 * 1000) / u64::from(r),
    }
}

/// Whether `pcm` is loud enough to be worth sending anywhere.
///
/// Silence costs a network round trip and comes back as an empty transcript or, worse, as
/// Whisper's best guess at what silence says — which is a real phenomenon and reads as crew
/// hearing voices. The threshold is deliberately low: it rejects a room, not a quiet person.
pub(crate) fn has_speech(pcm: &[i16]) -> bool {
    const FLOOR: i64 = 500; // ~1.5% of full scale, RMS
    if pcm.is_empty() {
        return false;
    }
    let sum: i64 = pcm.iter().map(|s| i64::from(*s) * i64::from(*s)).sum();
    let rms = ((sum / pcm.len() as i64) as f64).sqrt() as i64;
    rms >= FLOOR
}

#[cfg(test)]
#[path = "wav_tests.rs"]
mod tests;
