//! A saved attachment opens only when it is a document or a picture.
use std::path::Path;

use super::opened;
use crate::linkclick::OPENED;

fn opens(name: &str) -> (bool, String) {
    OPENED.with(|o| o.borrow_mut().clear());
    let said = opened(&Path::new("/nowhere/Downloads").join(name));
    (OPENED.with(|o| !o.borrow().is_empty()), said)
}

#[test]
fn a_document_or_a_picture_opens_and_says_so() {
    for name in ["report.pdf", "photo.JPG", "deck.pptx", "notes.txt"] {
        let (went, said) = opens(name);
        assert!(went, "{name}");
        assert!(said.ends_with("opening it"), "{said}");
    }
}

/// A program, or a file that names no kind, is saved and left alone.
#[test]
fn a_program_is_saved_and_never_opened() {
    for name in [
        "invoice.pdf.command",
        "setup.pkg",
        "run.sh",
        "Tool.app",
        "tool.jar",
        "invoice",
    ] {
        let (went, said) = opens(name);
        assert!(!went, "{name} was opened");
        assert!(said.contains("not opened"), "{said}");
    }
}
