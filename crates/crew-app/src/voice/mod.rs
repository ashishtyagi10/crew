//! Voice: the third [`Channel`], and the one that finally makes the trait's first line true —
//! *"a pane, a phone and a microphone are the same kind of thing"*.
//!
//! It is a channel and nothing more. A spoken task reaches the same session a typed one does,
//! through the same gate, into the same ledger, and the answer comes back through `send` like any
//! other. What is different is only the transport: a microphone in, a speaker out, with Whisper
//! and OpenAI's TTS on the two ends of the wire.
//!
//! **Push-to-talk, not a wake word.** A hotkey is a channel; a wake word is a wake word plus an
//! always-on microphone plus a consent story, and shipping them together means shipping neither.
//! `crew daemon listen` is the button: press it, speak, press it again (or stop talking). Nothing
//! opens the microphone until you do.
//!
//! **Barge-in from the start.** Pressing the button while crew is talking stops it mid-sentence,
//! because a spoken answer you cannot interrupt is worse than a printed one.
//!
//! Everything below the [`Ears`], [`Mouth`] and [`Speech`] traits is fakeable, which is how the
//! whole state machine — listening, transcribing, refusing silence, speaking, being interrupted —
//! is tested with no microphone, no key and no network.
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use super::channel::{Channel, Inbound};

pub(crate) mod mic;
pub(crate) mod openai;
pub(crate) mod wav;

/// The one address a microphone has. There is one seat at this keyboard.
pub(crate) const ADDRESS: &str = "voice:local";

/// Longest single utterance. A button somebody forgot to press again must not become an
/// unbounded recording and a very expensive transcription.
pub(crate) const MAX_UTTERANCE_MS: u64 = 60_000;

/// Where the samples come from.
pub(crate) trait Ears: Send + Sync {
    /// Record until `stop` goes true or the recording reaches its limit. 16-bit mono at
    /// [`openai::MIC_RATE`].
    fn record(&self, stop: &AtomicBool) -> Result<Vec<i16>, String>;
    /// Whether a microphone is actually available on this machine.
    fn available(&self) -> bool {
        true
    }
}

/// Where the samples go.
pub(crate) trait Mouth: Send + Sync {
    /// Play `pcm` at `rate`, stopping early if `stop` goes true — which is what barge-in is.
    fn play(&self, pcm: &[i16], rate: u32, stop: &AtomicBool) -> Result<(), String>;
}

/// The two model calls.
pub(crate) trait Speech: Send + Sync {
    fn transcribe(&self, wav: &[u8]) -> Result<String, String>;
    /// 16-bit mono PCM at [`openai::TTS_RATE`].
    fn speak(&self, text: &str) -> Result<Vec<i16>, String>;
}

/// What the channel is doing, for the operator line and for the button's toggle.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) enum State {
    #[default]
    Idle,
    Listening,
    Thinking,
    Speaking,
}

/// Shared between the channel and the threads it spawns.
#[derive(Default)]
struct Shared {
    inbox: Vec<Inbound>,
    notices: Vec<String>,
    state: State,
}

/// A microphone and a speaker, as a channel.
pub(crate) struct Voice {
    shared: Arc<Mutex<Shared>>,
    ears: Arc<dyn Ears>,
    mouth: Arc<dyn Mouth>,
    speech: Option<Arc<dyn Speech>>,
    /// Set to stop the current recording — the second press of the button.
    stop_record: Arc<AtomicBool>,
    /// Set to stop the current playback — barge-in.
    stop_play: Arc<AtomicBool>,
}

impl Voice {
    /// The real thing: this machine's microphone and speaker, OpenAI at both ends. Ready only
    /// with a key AND an audio device — a channel that claims to be usable and then cannot hear
    /// you is worse than one that says it is not configured.
    pub(crate) fn from_env() -> Self {
        Self::new(
            Arc::new(mic::Device::default()),
            Arc::new(mic::Device::default()),
            openai::OpenAiSpeech::from_env().map(|s| Arc::new(s) as Arc<dyn Speech>),
        )
    }

    pub(crate) fn new(
        ears: Arc<dyn Ears>,
        mouth: Arc<dyn Mouth>,
        speech: Option<Arc<dyn Speech>>,
    ) -> Self {
        Self {
            shared: Arc::new(Mutex::new(Shared::default())),
            ears,
            mouth,
            speech,
            stop_record: Arc::new(AtomicBool::new(false)),
            stop_play: Arc::new(AtomicBool::new(false)),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Shared> {
        self.shared.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub(crate) fn state(&self) -> State {
        self.lock().state
    }

    /// What is waiting to be polled, without taking it — the seam a test waits on.
    #[cfg(test)]
    pub(crate) fn poll_ready(&self) -> Vec<Inbound> {
        self.lock().inbox.clone()
    }

    /// The button. Pressed while idle it opens the microphone; pressed while listening it closes
    /// it; pressed while crew is talking it interrupts — one control, three meanings, and each
    /// is what a person pressing it again would expect.
    ///
    /// Returns what the press did, for the terminal that pressed it.
    pub(crate) fn press(&self) -> &'static str {
        match self.state() {
            State::Listening => {
                self.stop_record.store(true, Ordering::SeqCst);
                "listening \u{2014} stopping"
            }
            State::Speaking => {
                // Barge-in. The playback thread checks this between blocks.
                self.stop_play.store(true, Ordering::SeqCst);
                "stopped talking"
            }
            State::Thinking => "still thinking about the last one",
            State::Idle => match self.begin() {
                Ok(()) => "listening \u{2014} speak, then press again",
                Err(e) => {
                    self.note(e.clone());
                    "could not start listening"
                }
            },
        }
    }

    /// Start recording on a thread of its own: the daemon's loop must keep turning while
    /// somebody talks, and a transcription is a network round trip.
    fn begin(&self) -> Result<(), String> {
        let Some(speech) = self.speech.clone() else {
            return Err("voice needs OPENAI_API_KEY".into());
        };
        if !self.ears.available() {
            return Err("no microphone crew can open".into());
        }
        self.stop_record.store(false, Ordering::SeqCst);
        self.lock().state = State::Listening;
        let (ears, shared, stop) = (
            Arc::clone(&self.ears),
            Arc::clone(&self.shared),
            Arc::clone(&self.stop_record),
        );
        std::thread::spawn(move || {
            let heard = ears.record(&stop);
            let mut g = shared.lock().unwrap_or_else(|e| e.into_inner());
            g.state = State::Thinking;
            drop(g);
            let said = heard.and_then(|pcm| transcribe(&*speech, &pcm));
            let mut g = shared.lock().unwrap_or_else(|e| e.into_inner());
            match said {
                Ok(Some(text)) => g.inbox.push(Inbound {
                    from: ADDRESS.to_string(),
                    text,
                }),
                // Silence is not an error and not a task: saying nothing must cost nothing.
                Ok(None) => g.notices.push("voice: heard nothing".into()),
                Err(e) => g.notices.push(format!("voice: {e}")),
            }
            g.state = State::Idle;
        });
        Ok(())
    }

    fn note(&self, text: String) {
        self.lock().notices.push(text);
    }
}

/// Samples to words. `Ok(None)` is silence — nothing was said, so nothing is sent anywhere.
fn transcribe(speech: &dyn Speech, pcm: &[i16]) -> Result<Option<String>, String> {
    if !wav::has_speech(pcm) {
        return Ok(None);
    }
    let text = speech.transcribe(&wav::encode(pcm, openai::MIC_RATE))?;
    let text = text.trim().to_string();
    // Whisper answers silence with a plausible sentence rather than an empty string, and an
    // empty transcript after a loud enough recording is the same non-event.
    Ok((!text.is_empty()).then_some(text))
}

impl Channel for Voice {
    fn kind(&self) -> &str {
        "voice"
    }

    fn poll(&mut self) -> Vec<Inbound> {
        std::mem::take(&mut self.lock().inbox)
    }

    /// Say it out loud, on a thread, interruptibly.
    fn send(&mut self, _to: &str, text: &str) -> Result<(), String> {
        let Some(speech) = self.speech.clone() else {
            return Err("voice needs OPENAI_API_KEY".into());
        };
        let (mouth, shared, stop) = (
            Arc::clone(&self.mouth),
            Arc::clone(&self.shared),
            Arc::clone(&self.stop_play),
        );
        stop.store(false, Ordering::SeqCst);
        self.lock().state = State::Speaking;
        let text = spoken(text);
        std::thread::spawn(move || {
            let played = speech
                .speak(&text)
                .and_then(|pcm| mouth.play(&pcm, openai::TTS_RATE, &stop));
            let mut g = shared.lock().unwrap_or_else(|e| e.into_inner());
            if let Err(e) = played {
                g.notices.push(format!("voice: {e}"));
            }
            g.state = State::Idle;
        });
        Ok(())
    }

    fn notices(&mut self) -> Vec<String> {
        std::mem::take(&mut self.lock().notices)
    }

    fn press(&mut self) -> Option<&'static str> {
        Some(Voice::press(self))
    }

    fn ready(&self) -> bool {
        self.speech.is_some() && self.ears.available()
    }

    /// A microphone has exactly one address, so a standing intent set from it needs no `--to`.
    fn default_address(&self) -> Option<String> {
        self.ready().then(|| ADDRESS.to_string())
    }
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
#[path = "mod_tests.rs"]
mod tests;
