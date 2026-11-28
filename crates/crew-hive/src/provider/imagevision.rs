//! Which models are given pictures. crew's catalog (`catalog/data.rs`) records
//! no modality, and the Models API's `capabilities` is not fetched, so this is
//! a table of model-id fragments.
//!
//! UNVERIFIED: written from what the families are documented to take, with no
//! live call behind it. It errs toward omitting: a vision model missing here
//! gets the `[image omitted: ...]` note (the user can see what happened), where
//! a text model wrongly listed would fail the whole request with a 400.
//!
//! Left text-only on purpose: whether qwen3.8 reads images is unconfirmed, and
//! text-only is the safe default until a live call shows otherwise.

/// Fragments of the ids of models that read images (lower case). Claude 4.x/5.x,
/// GPT-4o and later, Gemini, and the `-vl` / vision lines of the open families.
const SEES: [&str; 10] = [
    "claude", "gpt-4o", "gpt-4.1", "gpt-5", "gpt-6", "gemini", "-vl", "vision", "pixtral",
    "llama-4",
];

/// Whether `model` is given images. Text-only: DashScope's `qwen-max`/`plus`/
/// `turbo`, DeepSeek, and anything unlisted.
pub fn sees(model: &str) -> bool {
    let m = model.to_ascii_lowercase();
    // OpenAI's reasoning line is `o3`/`o4-mini`: matched at the id's start, since
    // "o3" inside another name means nothing.
    let id = m.rsplit('/').next().unwrap_or(&m);
    id.starts_with("o3") || id.starts_with("o4") || SEES.iter().any(|f| m.contains(f))
}

/// `images` split for `model`: the ones it is shown, and a note for each one
/// it is not. Decided where the request is rendered, per model attempted: a
/// fallback walk renders once per model, and a text-only fallback must not be
/// handed the picture the first model took.
pub fn split<'a>(model: &str, images: &'a [super::Image]) -> (Vec<&'a super::Image>, Vec<String>) {
    match sees(model) {
        true => (images.iter().collect(), Vec::new()),
        false => (
            Vec::new(),
            images
                .iter()
                .map(|i| super::image::text_only(&i.name))
                .collect(),
        ),
    }
}

/// `text` with `notes` on lines after it.
pub fn with_notes(text: &str, notes: &[String]) -> String {
    notes
        .iter()
        .fold(text.to_string(), |t, n| match t.is_empty() {
            true => n.clone(),
            false => format!("{t}\n\n{n}"),
        })
}

#[cfg(test)]
#[path = "imagevision_tests.rs"]
mod tests;
