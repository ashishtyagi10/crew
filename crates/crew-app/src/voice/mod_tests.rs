//! The whole state machine with no microphone, no key and no network: what a press does, what
//! silence costs, what an interruption stops, and what a reply sounds like.
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
fn loud() -> Vec<i16> {
    (0..16_000)
        .map(|i| ((i as f32 / 8.0).sin() * 6_000.0) as i16)
        .collect()
}

struct Rig {
    voice: Voice,
    said: Arc<Mutex<Vec<String>>>,
    played: Arc<Mutex<Vec<Vec<i16>>>>,
    interrupted: Arc<AtomicBool>,
    stopped: Arc<AtomicBool>,
}

fn rig(pcm: Vec<i16>, heard: &str, fail: bool) -> Rig {
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
fn until(f: impl Fn() -> bool) -> bool {
    for _ in 0..200 {
        if f() {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    false
}

#[test]
fn a_press_listens_and_what_was_said_arrives_as_a_message() {
    // The whole point: a spoken sentence becomes an ordinary inbound message, from an address
    // the daemon can answer.
    let mut r = rig(loud(), "what is on my calendar", false);
    r.voice.press();
    assert!(until(
        || !r.voice.poll_ready().is_empty() || r.voice.state() == State::Idle
    ));
    let msgs = r.voice.poll();
    assert_eq!(msgs.len(), 1, "one utterance, one message");
    assert_eq!(msgs[0].text, "what is on my calendar");
    assert_eq!(msgs[0].from, ADDRESS, "and it can be answered");
}

#[test]
fn a_second_press_stops_the_recording() {
    let mut r = rig(loud(), "done talking", false);
    r.voice.press();
    assert!(until(|| r.voice.state() == State::Listening));
    let what = r.voice.press();
    assert!(what.contains("stopping"), "{what}");
    assert!(
        until(|| r.stopped.load(Ordering::SeqCst)),
        "the microphone was told to stop"
    );
    assert!(until(|| !r.voice.poll_ready().is_empty()));
}

#[test]
fn silence_costs_nothing_and_says_so() {
    // Whisper answers silence with a plausible sentence, which reads as crew hearing voices.
    let mut r = rig(vec![0; 16_000], "a hallucinated sentence", false);
    r.voice.press();
    assert!(until(|| r.voice.state() == State::Idle));
    assert!(
        r.voice.poll().is_empty(),
        "nothing was said, so nothing was sent"
    );
    let notices = r.voice.notices();
    assert!(
        notices.iter().any(|n| n.contains("heard nothing")),
        "{notices:?}"
    );
}

#[test]
fn a_failed_transcription_is_a_notice_rather_than_a_task() {
    let mut r = rig(loud(), "", true);
    r.voice.press();
    assert!(until(|| r.voice.state() == State::Idle));
    assert!(r.voice.poll().is_empty());
    let notices = r.voice.notices();
    assert!(
        notices.iter().any(|n| n.contains("service is down")),
        "{notices:?}"
    );
}

#[test]
fn a_reply_is_spoken_aloud() {
    let mut r = rig(loud(), "hi", false);
    r.voice.send(ADDRESS, "The forecast is fine.").unwrap();
    assert!(until(|| !r.played.lock().unwrap().is_empty()));
    assert_eq!(r.said.lock().unwrap().as_slice(), ["The forecast is fine."]);
}

#[test]
fn pressing_while_crew_is_talking_stops_it_mid_sentence() {
    // Barge-in. A spoken answer you cannot interrupt is worse than a printed one.
    let mut r = rig(loud(), "never mind", false);
    r.voice.send(ADDRESS, "a very long answer").unwrap();
    assert!(until(|| r.voice.state() == State::Speaking));
    let what = r.voice.press();
    assert!(what.contains("stopped talking"), "{what}");
    assert!(
        until(|| r.interrupted.load(Ordering::SeqCst)),
        "playback was cut off"
    );
    assert!(r.played.lock().unwrap().is_empty(), "and it never finished");
}

#[test]
fn a_channel_with_no_key_is_registered_but_not_ready() {
    // The same shape Telegram has: present the moment it is configured, inert until it is.
    let voice = Voice::new(
        Arc::new(FakeEars {
            pcm: vec![],
            stopped: Arc::new(AtomicBool::new(false)),
            available: true,
        }),
        Arc::new(FakeMouth::default()),
        None,
    );
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

#[test]
fn a_reply_written_for_a_screen_is_shortened_for_an_ear() {
    // A transcript is written to be read: code fences, tables, a hundred-line diff. Reading one
    // aloud is unbearable, so the spoken form says what it can and says that it stopped.
    assert_eq!(spoken("one\ntwo   three"), "one two three");
    assert_eq!(
        spoken("here it is:\n```\nfn main() {}\n```\ndone"),
        "here it is: fn main() {} done",
        "the fence markers are not read out"
    );
    let long = "word ".repeat(400);
    let out = spoken(&long);
    assert!(out.chars().count() < 760, "{}", out.chars().count());
    assert!(out.ends_with("the rest is on screen."), "{out}");
}
