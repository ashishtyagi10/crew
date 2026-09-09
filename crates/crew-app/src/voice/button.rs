//! The button. One control, three meanings: pressed while idle it opens the microphone, pressed
//! while listening it closes it, pressed while crew is talking it interrupts — and each is what
//! a person pressing it again would expect.
//!
//! Split from [`super`] for the line cap, along the line between what the channel IS and what
//! pressing it DOES.
use std::sync::atomic::Ordering;
use std::sync::Arc;

use super::{words, State, Voice, ADDRESS};
use crate::channel::Inbound;

impl Voice {
    /// What the press did, for the terminal that pressed it — or why it could not be done, which
    /// is the same terminal's business: a button that fails silently looks like a button that
    /// worked.
    pub(crate) fn press(&self) -> Result<&'static str, String> {
        match self.state() {
            State::Listening => {
                self.stop_record.store(true, Ordering::SeqCst);
                Ok("listening \u{2014} stopping")
            }
            State::Speaking => {
                // Barge-in. The playback thread checks this between blocks.
                self.stop_play.store(true, Ordering::SeqCst);
                Ok("stopped talking")
            }
            State::Thinking => Ok("still thinking about the last one"),
            State::Idle => self
                .begin()
                .map(|()| "listening \u{2014} speak, then press again")
                .inspect_err(|e| self.note(e.clone())),
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
            let said = heard.and_then(|pcm| words::transcribe(&*speech, &pcm));
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
}

#[cfg(test)]
#[path = "button_tests.rs"]
mod tests;
