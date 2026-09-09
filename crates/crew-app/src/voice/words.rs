//! The two ends of the wire where sound becomes words and words become sound: what a recording
//! has to be before it is worth transcribing, and what a reply has to become before it is worth
//! hearing.
//!
//! Split from [`super`] for the line cap, along the line between the channel's state machine
//! and the two pure conversions it runs at its edges.
use super::{openai, wav, Speech};

/// Samples to words. `Ok(None)` is silence — nothing was said, so nothing is sent anywhere.
pub(super) fn transcribe(speech: &dyn Speech, pcm: &[i16]) -> Result<Option<String>, String> {
    if !wav::has_speech(pcm) {
        return Ok(None);
    }
    let text = speech.transcribe(&wav::encode(pcm, openai::MIC_RATE))?;
    let text = text.trim().to_string();
    // Whisper answers silence with a plausible sentence rather than an empty string, and an
    // empty transcript after a loud enough recording is the same non-event.
    Ok((!text.is_empty()).then_some(text))
}

/// What a reply sounds like. A transcript is written for a screen — code fences, tables, a
/// hundred-line diff — and reading one aloud is unbearable, so the spoken form is clipped to
/// something a person would actually listen to and says when it clipped.
pub(crate) fn spoken(text: &str) -> String {
    /// Long enough for a real answer, short enough to interrupt.
    const CAP: usize = 700;
    let flat: String = text
        .lines()
        .filter(|l| !l.trim_start().starts_with("```"))
        .collect::<Vec<_>>()
        .join(" ");
    let flat = flat.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() <= CAP {
        return flat;
    }
    let cut: String = flat.chars().take(CAP).collect();
    let cut = match cut.rsplit_once(' ') {
        Some((head, _)) => head.to_string(),
        None => cut,
    };
    format!("{cut}\u{2026} the rest is on screen.")
}

#[cfg(test)]
#[path = "words_tests.rs"]
mod tests;
