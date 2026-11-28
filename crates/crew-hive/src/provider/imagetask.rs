//! Pictures a user's message names. A pasted screenshot arrives as the PATH of
//! a temp PNG (`crew-app` `paste_image`), which a CLI agent opens itself; this
//! is what lets crew's own API engine see it too.
use std::path::{Path, PathBuf};

use super::image::{omitted, Image, MAX_IMAGES};

/// What a message's image paths came to.
#[derive(Debug, Default)]
pub struct Attached {
    /// The pictures read, in the order named, at most [`MAX_IMAGES`].
    pub images: Vec<Image>,
    /// What to tell the model about each one left out (too big, past the
    /// limit, unreadable). A path that names no file is not here: prose that
    /// mentions `shot.png` is not a request to attach one.
    pub notes: Vec<String>,
}

const EXTENSIONS: [&str; 5] = [".png", ".jpg", ".jpeg", ".gif", ".webp"];

/// The existing image files `ask` names, absolute or relative to `cwd`.
///
/// Scanned in the ask as the user wrote it, never the framed prompt: recalled
/// turns and memory name paths of their own, and a picture the user did not
/// mention this time must not ride along. The path text stays in the message;
/// this only adds the picture beside it.
pub fn from_task(ask: &str, cwd: &Path) -> Attached {
    let mut out = Attached::default();
    let mut seen: Vec<PathBuf> = Vec::new();
    // `\ ` is a space inside a path (a dragged-in file); hold it through the split.
    let held = ask.replace("\\ ", "\u{0}");
    for word in held.split_whitespace() {
        let word = word
            .trim_matches(|c: char| "\"'`()<>[]{},;:!?.".contains(c))
            .replace('\u{0}', " ");
        let lower = word.to_ascii_lowercase();
        if !EXTENSIONS.iter().any(|e| lower.ends_with(e)) {
            continue;
        }
        let path = if Path::new(&word).is_absolute() {
            PathBuf::from(&word)
        } else {
            cwd.join(&word)
        };
        let Ok(real) = path.canonicalize() else {
            continue;
        };
        if !real.is_file() || seen.contains(&real) {
            continue;
        }
        seen.push(real.clone());
        let name = real
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into());
        if out.images.len() >= MAX_IMAGES {
            out.notes
                .push(omitted(&name, "only the first 5 images are attached"));
            continue;
        }
        match Image::read(&real) {
            Ok(img) => out.images.push(img),
            Err(note) => out.notes.push(note),
        }
    }
    out
}

#[cfg(test)]
#[path = "imagetask_tests.rs"]
mod tests;
