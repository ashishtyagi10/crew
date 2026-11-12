//! Whether a file is one to READ here — the question Far's Enter asks before
//! it decides between crew's viewer and the system's opener.
//!
//! Text in any of its rungs (markdown, code, data, CSV, a diff) and pictures
//! are read in the viewer, in a tab of their own, so reading a file never
//! means leaving crew. What the viewer could only show as a card saying it
//! cannot — a binary, a disk image, an app — still goes to the system, and
//! so does a document that needs converting first (a PDF, a Word file):
//! Enter on those has always meant "open it properly".
use std::io::Read;
use std::path::Path;

use super::detect::{detect, Format, SNIFF_BYTES};
use super::extractor::Probe;

/// One short read of the file's head, on the caller's thread: the same
/// sniff the loader makes, without waiting for a worker to make it.
pub(crate) fn readable(path: &Path) -> bool {
    let mut head = Vec::with_capacity(SNIFF_BYTES);
    let read =
        std::fs::File::open(path).and_then(|f| f.take(SNIFF_BYTES as u64).read_to_end(&mut head));
    if read.is_err() {
        return false;
    }
    // No extractors: a file that needs one is not read here.
    let none = Probe {
        textutil: false,
        pdftotext: false,
    };
    !matches!(
        detect(path, &head, none),
        Format::Opaque { .. } | Format::Extract { .. }
    )
}

#[cfg(test)]
#[path = "readable_tests.rs"]
mod tests;
