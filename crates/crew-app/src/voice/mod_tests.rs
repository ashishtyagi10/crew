//! The channel with no microphone, no key and no network: the fakes every voice test runs on,
//! what readiness means, and what a reply does on the way out. What the button does is in
//! `button_tests`; what a reply sounds like is in `words_tests`.
use super::*;
use crate::channel::Channel;

/// A microphone that hands back whatever the test decided it heard, and records how long it was
/// asked to listen for.
struct FakeEars {
    pcm: Vec<i16>,
    /// Set when `record` was asked to stop rather than running to its own end.
    stopped: Arc<AtomicBool>,
    available: bool,
}

impl Ears for FakeEars {
    fn record(&self, stop: &AtomicBool) -> Result<Vec<i16>, String> {
        // Wait briefly for a stop, so a test can press the button twice.
        for _ in 0..50 {
            if stop.load(Ordering::SeqCst) {
                self.stopped.store(true, Ordering::SeqCst);
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        Ok(self.pcm.clone())
    }
    fn available(&self) -> bool {
        self.available
    }
}

/// A speaker that remembers what it played and whether it was cut off.
#[derive(Default)]
struct FakeMouth {
    played: Arc<Mutex<Vec<Vec<i16>>>>,
    interrupted: Arc<AtomicBool>,
}

impl Mouth for FakeMouth {
    fn play(&self, pcm: &[i16], _rate: u32, stop: &AtomicBool) -> Result<(), String> {
        // Long enough that a test can interrupt it, short enough not to slow the suite.
        for _ in 0..100 {
            if stop.load(Ordering::SeqCst) {
                self.interrupted.store(true, Ordering::SeqCst);
                return Ok(());
            }
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        self.played
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(pcm.to_vec());
        Ok(())
    }
}

/// A speech service that transcribes to a fixed sentence and remembers what it was asked to say.
struct FakeSpeech {
    heard: String,
    said: Arc<Mutex<Vec<String>>>,
    fail: bool,
}

impl Speech for FakeSpeech {
    fn transcribe(&self, wav: &[u8]) -> Result<String, String> {
        if self.fail {
            return Err("the transcription service is down".into());
        }
        assert_eq!(&wav[0..4], b"RIFF", "a WAV file, not raw samples");
        Ok(self.heard.clone())
    }
    fn speak(&self, text: &str) -> Result<Vec<i16>, String> {
        self.said
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(text.to_string());
        Ok(vec![1, 2, 3])
    }
}

/// A second of something loud enough to count as speech.
pub(super) fn loud() -> Vec<i16> {
    (0..16_000)
        .map(|i| ((i as f32 / 8.0).sin() * 6_000.0) as i16)
        .collect()
}

pub(super) struct Rig {
    pub voice: Voice,
    pub said: Arc<Mutex<Vec<String>>>,
    pub played: Arc<Mutex<Vec<Vec<i16>>>>,
    pub interrupted: Arc<AtomicBool>,
    pub stopped: Arc<AtomicBool>,
}

pub(super) fn rig(pcm: Vec<i16>, heard: &str, fail: bool) -> Rig {
    let stopped = Arc::new(AtomicBool::new(false));
    let said = Arc::new(Mutex::new(Vec::new()));
    let played = Arc::new(Mutex::new(Vec::new()));
    let interrupted = Arc::new(AtomicBool::new(false));
    let ears = FakeEars {
        pcm,
        stopped: Arc::clone(&stopped),
        available: true,
    };
    let mouth = FakeMouth {
        played: Arc::clone(&played),
        interrupted: Arc::clone(&interrupted),
    };
    let speech = FakeSpeech {
        heard: heard.to_string(),
        said: Arc::clone(&said),
        fail,
    };
    Rig {
        voice: Voice::new(Arc::new(ears), Arc::new(mouth), Some(Arc::new(speech))),
        said,
        played,
        interrupted,
        stopped,
    }
}

/// Wait for `f` to hold, up to a second. Every path here crosses a thread.
pub(super) fn until(f: impl Fn() -> bool) -> bool {
    for _ in 0..200 {
        if f() {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    false
}

#[test]
fn a_reply_is_spoken_aloud() {
    let mut r = rig(loud(), "hi", false);
    r.voice.send(ADDRESS, "The forecast is fine.").unwrap();
    assert!(until(|| !r.played.lock().unwrap().is_empty()));
    assert_eq!(r.said.lock().unwrap().as_slice(), ["The forecast is fine."]);
}

/// A voice with a microphone but no key: what a machine without `OPENAI_API_KEY` registers.
pub(super) fn keyless() -> Voice {
    Voice::new(
        Arc::new(FakeEars {
            pcm: vec![],
            stopped: Arc::new(AtomicBool::new(false)),
            available: true,
        }),
        Arc::new(FakeMouth::default()),
        None,
    )
}

#[test]
fn a_channel_with_no_key_is_registered_but_not_ready() {
    // The same shape Telegram has: present the moment it is configured, inert until it is.
    let voice = keyless();
    assert_eq!(voice.kind(), "voice");
    assert!(!voice.ready());
    assert_eq!(voice.default_address(), None);
}

#[test]
fn a_machine_with_no_microphone_is_not_ready_either() {
    // A channel that claims to be usable and then cannot hear you is worse than one that says
    // it is not configured.
    let voice = Voice::new(
        Arc::new(FakeEars {
            pcm: vec![],
            stopped: Arc::new(AtomicBool::new(false)),
            available: false,
        }),
        Arc::new(FakeMouth::default()),
        Some(Arc::new(FakeSpeech {
            heard: String::new(),
            said: Arc::new(Mutex::new(Vec::new())),
            fail: false,
        })),
    );
    assert!(!voice.ready());
}

#[test]
fn a_ready_voice_is_the_one_address_a_standing_intent_needs() {
    let r = rig(loud(), "hi", false);
    assert!(r.voice.ready());
    assert_eq!(r.voice.default_address(), Some(ADDRESS.to_string()));
}
