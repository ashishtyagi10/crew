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

mod button;
pub(crate) mod mic;
pub(crate) mod openai;
pub(crate) mod wav;
pub(crate) mod words;

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

    fn note(&self, text: String) {
        self.lock().notices.push(text);
    }
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
        let text = words::spoken(text);
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

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
