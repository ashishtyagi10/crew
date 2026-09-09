//! The microphone and the speaker, and nothing else.
//!
//! This is the only file in `voice/` that cannot be tested without hardware, so it is the only
//! one that does no thinking: open a stream, fill a buffer, stop. Every decision — when to
//! listen, what counts as silence, what to say — is above it, behind traits, and covered.
//!
//! **Where it is built.** `cpal` is a dependency on macOS and Windows only. On Linux it needs
//! ALSA headers at build time, and crew's install is deliberately a single binary with no
//! system dependencies; a Linux build therefore reports no microphone rather than failing to
//! compile, and voice is unavailable there until somebody wants it enough to add the dep.
use std::sync::atomic::{AtomicBool, Ordering};

/// This machine's audio, through whatever backend the platform has.
#[derive(Default)]
pub(crate) struct Device;

/// Nearest-neighbour resample to crew's 16 kHz. Whisper takes any rate, but the upload is
/// three times the size at 48 kHz for no accuracy, and this is the whole of the conversion:
/// speech is band-limited well under 8 kHz, so the aliasing a proper filter would remove is
/// not in the signal to begin with.
fn to_mic_rate(input: &[f32], from: u32, channels: u16) -> Vec<i16> {
    let from = from.max(1);
    let channels = channels.max(1) as usize;
    let frames = input.len() / channels;
    let out_len =
        (frames as u64 * u64::from(super::openai::MIC_RATE) / u64::from(from)).max(0) as usize;
    (0..out_len)
        .map(|i| {
            let src = i as u64 * u64::from(from) / u64::from(super::openai::MIC_RATE).max(1);
            // Mono: the first channel, not a mix. Two channels of one person are the same
            // person, and averaging them adds a comb filter for nothing.
            let s = input
                .get(src as usize * channels)
                .copied()
                .unwrap_or_default();
            (s.clamp(-1.0, 1.0) * f32::from(i16::MAX)) as i16
        })
        .collect()
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
mod real {
    use super::*;
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    use std::sync::{Arc, Mutex};

    impl super::super::Ears for Device {
        fn record(&self, stop: &AtomicBool) -> Result<Vec<i16>, String> {
            let host = cpal::default_host();
            let device = host
                .default_input_device()
                .ok_or("no input device \u{2014} crew has nothing to listen with")?;
            let config = device
                .default_input_config()
                .map_err(|e| format!("no usable microphone format: {e}"))?;
            let (rate, channels) = (config.sample_rate().0, config.channels());
            let buf = Arc::new(Mutex::new(Vec::<f32>::new()));
            let sink = Arc::clone(&buf);
            let stream = device
                .build_input_stream(
                    &config.clone().into(),
                    move |data: &[f32], _: &cpal::InputCallbackInfo| {
                        sink.lock()
                            .unwrap_or_else(|e| e.into_inner())
                            .extend_from_slice(data);
                    },
                    |e| eprintln!("crew: microphone error: {e}"),
                    None,
                )
                .map_err(|e| format!("could not open the microphone: {e}"))?;
            stream
                .play()
                .map_err(|e| format!("could not start the microphone: {e}"))?;
            let start = std::time::Instant::now();
            while !stop.load(Ordering::SeqCst) {
                if start.elapsed().as_millis() as u64 >= super::super::MAX_UTTERANCE_MS {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            drop(stream);
            let raw = std::mem::take(&mut *buf.lock().unwrap_or_else(|e| e.into_inner()));
            Ok(to_mic_rate(&raw, rate, channels))
        }

        fn available(&self) -> bool {
            cpal::default_host().default_input_device().is_some()
        }
    }

    impl super::super::Mouth for Device {
        fn play(&self, pcm: &[i16], rate: u32, stop: &AtomicBool) -> Result<(), String> {
            let host = cpal::default_host();
            let device = host
                .default_output_device()
                .ok_or("no output device \u{2014} crew has nothing to speak through")?;
            let channels = device
                .default_output_config()
                .map_err(|e| format!("no usable speaker format: {e}"))?
                .channels();
            let config = cpal::StreamConfig {
                channels,
                sample_rate: cpal::SampleRate(rate),
                buffer_size: cpal::BufferSize::Default,
            };
            let samples: Arc<Mutex<std::vec::IntoIter<i16>>> =
                Arc::new(Mutex::new(pcm.to_vec().into_iter()));
            let feed = Arc::clone(&samples);
            let done = Arc::new(std::sync::atomic::AtomicBool::new(false));
            let finished = Arc::clone(&done);
            let stream = device
                .build_output_stream(
                    &config,
                    move |out: &mut [f32], _: &cpal::OutputCallbackInfo| {
                        let mut src = feed.lock().unwrap_or_else(|e| e.into_inner());
                        for frame in out.chunks_mut(channels as usize) {
                            // One sample fans out to every channel: the reply is mono and
                            // belongs in the middle of the room, not in one ear.
                            let s = src.next().map(|s| f32::from(s) / f32::from(i16::MAX));
                            let v = s.unwrap_or(0.0);
                            if s.is_none() {
                                finished.store(true, Ordering::SeqCst);
                            }
                            for c in frame.iter_mut() {
                                *c = v;
                            }
                        }
                    },
                    |e| eprintln!("crew: speaker error: {e}"),
                    None,
                )
                .map_err(|e| format!("could not open the speaker: {e}"))?;
            stream
                .play()
                .map_err(|e| format!("could not start the speaker: {e}"))?;
            // Barge-in checks here, between blocks: dropping the stream is what stops the sound.
            while !done.load(Ordering::SeqCst) && !stop.load(Ordering::SeqCst) {
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            Ok(())
        }
    }
}

/// Platforms crew does not build an audio backend for. Voice reports itself unavailable, and
/// everything else about crew is unchanged — which is the same shape every other optional
/// capability here has.
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
mod real {
    use super::*;

    const WHY: &str = "voice is not built on this platform (it needs an audio backend crew does \
                       not depend on here)";

    impl super::super::Ears for Device {
        fn record(&self, _stop: &AtomicBool) -> Result<Vec<i16>, String> {
            Err(WHY.into())
        }
        fn available(&self) -> bool {
            false
        }
    }

    impl super::super::Mouth for Device {
        fn play(&self, _pcm: &[i16], _rate: u32, _stop: &AtomicBool) -> Result<(), String> {
            Err(WHY.into())
        }
    }
}

#[cfg(test)]
#[path = "mic_tests.rs"]
mod tests;
