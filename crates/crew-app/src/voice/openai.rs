//! Whisper and TTS over HTTP: the two calls that turn a microphone into a channel.
//!
//! `POST /v1/audio/transcriptions` takes a multipart form with a file in it, and
//! `POST /v1/audio/speech` returns audio. crew asks for **`pcm`** back — raw 24 kHz 16-bit mono —
//! rather than mp3, and that is the one decision here worth stating: an mp3 would mean a decoder
//! dependency to play a sentence, and `pcm` is what the speaker wants anyway.
//!
//! The multipart body is built by hand. `reqwest`'s `multipart` feature is not enabled in this
//! workspace, the format is a boundary and two headers, and a body a test can read back beats a
//! feature flag every crate then compiles.
use std::time::Duration;

/// What the request bodies are made of, so the shapes can be checked without a network.
pub(crate) struct Form {
    pub content_type: String,
    pub body: Vec<u8>,
}

/// The multipart body for a transcription: the audio file plus the model.
pub(crate) fn transcription_form(wav: &[u8], model: &str) -> Form {
    // Fixed rather than random: nothing here is adversarial, and a deterministic body is one a
    // test can assert on byte for byte.
    let boundary = "----crew-voice-boundary";
    let mut body = Vec::new();
    let mut field = |name: &str, value: &str| {
        body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
        body.extend_from_slice(
            format!("Content-Disposition: form-data; name=\"{name}\"\r\n\r\n").as_bytes(),
        );
        body.extend_from_slice(value.as_bytes());
        body.extend_from_slice(b"\r\n");
    };
    field("model", model);
    // A response with no JSON to parse: the transcript IS the body.
    field("response_format", "text");
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(
        b"Content-Disposition: form-data; name=\"file\"; filename=\"speech.wav\"\r\n",
    );
    body.extend_from_slice(b"Content-Type: audio/wav\r\n\r\n");
    body.extend_from_slice(wav);
    body.extend_from_slice(b"\r\n");
    body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
    Form {
        content_type: format!("multipart/form-data; boundary={boundary}"),
        body,
    }
}

/// The JSON body for a spoken reply.
pub(crate) fn speech_body(text: &str, model: &str, voice: &str) -> serde_json::Value {
    serde_json::json!({
        "model": model,
        "voice": voice,
        "input": text,
        // Raw samples, so nothing has to decode an mp3 to say one sentence.
        "response_format": "pcm",
    })
}

/// The sample rate `/v1/audio/speech` returns `pcm` at. Not negotiable and not in the response,
/// so it is written down here where the playback path reads it.
pub(crate) const TTS_RATE: u32 = 24_000;

/// What crew records at. Whisper resamples anything, and 16 kHz is the rate speech models are
/// trained on — sending 48 kHz would cost three times the upload for no accuracy.
pub(crate) const MIC_RATE: u32 = 16_000;

/// Bytes of PCM to 16-bit samples, little-endian. A trailing odd byte is dropped rather than
/// misread: half a sample is not a sample.
pub(crate) fn pcm_from_bytes(bytes: &[u8]) -> Vec<i16> {
    bytes
        .chunks_exact(2)
        .map(|b| i16::from_le_bytes([b[0], b[1]]))
        .collect()
}

/// The OpenAI voice pair: Whisper in, TTS out.
pub(crate) struct OpenAiSpeech {
    key: String,
    base: String,
    stt_model: String,
    tts_model: String,
    voice: String,
}

impl OpenAiSpeech {
    /// From the environment, or `None` when there is no key — which is what makes the channel
    /// report itself unready rather than failing at the first word.
    pub(crate) fn from_env() -> Option<Self> {
        let key = std::env::var("OPENAI_API_KEY")
            .ok()
            .map(|k| k.trim().to_string())
            .filter(|k| !k.is_empty())?;
        Some(Self {
            key,
            base: std::env::var("OPENAI_BASE_URL")
                .unwrap_or_else(|_| "https://api.openai.com/v1".into()),
            stt_model: std::env::var("CREW_VOICE_STT").unwrap_or_else(|_| "whisper-1".into()),
            tts_model: std::env::var("CREW_VOICE_TTS").unwrap_or_else(|_| "gpt-4o-mini-tts".into()),
            voice: std::env::var("CREW_VOICE").unwrap_or_else(|_| "alloy".into()),
        })
    }

    fn client(&self) -> Result<reqwest::Client, String> {
        reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .map_err(|e| format!("could not build an HTTP client: {e}"))
    }
}

/// Run one request to completion on this thread.
///
/// Every call here already happens on a thread of the voice channel's own — recording and
/// speaking must never block the daemon's 250ms loop — so a small runtime built and dropped
/// around the request is the whole of the async story, and `reqwest`'s blocking feature (which
/// spawns a runtime of its own on every client) stays out of the workspace.
fn wait<T>(fut: impl std::future::Future<Output = T>) -> Result<T, String> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("could not start an HTTP runtime: {e}"))
        .map(|rt| rt.block_on(fut))
}

impl super::Speech for OpenAiSpeech {
    fn transcribe(&self, wav: &[u8]) -> Result<String, String> {
        let form = transcription_form(wav, &self.stt_model);
        let req = self
            .client()?
            .post(format!("{}/audio/transcriptions", self.base))
            .bearer_auth(&self.key)
            .header("Content-Type", form.content_type)
            .body(form.body);
        wait(async move {
            let res = req
                .send()
                .await
                .map_err(|e| format!("could not reach the transcription service: {e}"))?;
            let status = res.status();
            let text = res.text().await.unwrap_or_default();
            match status.is_success() {
                true => Ok(text.trim().to_string()),
                false => Err(format!("transcription failed ({status}): {}", text.trim())),
            }
        })?
    }

    fn speak(&self, text: &str) -> Result<Vec<i16>, String> {
        let req = self
            .client()?
            .post(format!("{}/audio/speech", self.base))
            .bearer_auth(&self.key)
            .json(&speech_body(text, &self.tts_model, &self.voice));
        wait(async move {
            let res = req
                .send()
                .await
                .map_err(|e| format!("could not reach the speech service: {e}"))?;
            let status = res.status();
            if !status.is_success() {
                let why = res.text().await.unwrap_or_default();
                return Err(format!("speech failed ({status}): {}", why.trim()));
            }
            let bytes = res
                .bytes()
                .await
                .map_err(|e| format!("could not read the spoken reply: {e}"))?;
            Ok(pcm_from_bytes(&bytes))
        })?
    }
}

#[cfg(test)]
#[path = "openai_tests.rs"]
mod tests;
