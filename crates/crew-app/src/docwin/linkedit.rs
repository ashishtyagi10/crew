//! Editing the one thing a render deliberately hides: a link's URL.
//!
//! Rendering a link means showing its text and NOT its target, so there is nothing on screen to
//! put a caret in. The frame already names the URL the caret is inside (v0.19.93); this makes
//! that line the field you type it in. Cmd+K opens it, Enter applies it as one splice over the
//! URL's bytes, Esc leaves the file exactly as it was.
//!
//! With a selection and no link, the same chord makes one: the selection becomes `[text]()` and
//! the field opens on the empty URL. Cancelling that takes the scaffold back out — an editor
//! that leaves `[half a thought]()` behind because you changed your mind is worse than one with
//! no shortcut at all.
//!
//! Everything below acts on a [`ViewPane`], not on a window, so the whole feature is testable
//! without a display: the window is where the keys and the frame come from, and nothing else.
use winit::keyboard::{Key, NamedKey};

use crate::viewpane::ViewPane;

/// A URL being typed into the frame.
#[derive(Debug)]
pub(crate) struct UrlEdit {
    /// The source bytes the typed URL replaces.
    pub from: u32,
    pub to: u32,
    /// What has been typed so far.
    pub buf: String,
    /// How many changes opening this field wrote into the document, and therefore how many a
    /// cancel has to take back. Zero when the caret was already in a link.
    pub undos: usize,
}

/// What a key does to an open field.
#[derive(Debug, PartialEq)]
pub(crate) enum FieldKey {
    Type(String),
    Backspace,
    Apply,
    Cancel,
}

/// Read a key as an edit to the URL field. `None` for anything the field does not answer, which
/// is left alone rather than swallowed — an open field must not eat the window's own keys.
pub(crate) fn field_key(key: &Key, pressed: bool) -> Option<FieldKey> {
    if !pressed {
        return None;
    }
    match key {
        Key::Named(NamedKey::Enter) => Some(FieldKey::Apply),
        Key::Named(NamedKey::Escape) => Some(FieldKey::Cancel),
        Key::Named(NamedKey::Backspace) => Some(FieldKey::Backspace),
        Key::Named(NamedKey::Space) => Some(FieldKey::Type(" ".into())),
        Key::Character(s) => Some(FieldKey::Type(s.to_string())),
        _ => None,
    }
}

impl UrlEdit {
    /// Open a field over the link the caret is in, or make one out of the selection. `Err`
    /// carries the line to show when there was neither.
    pub(crate) fn open(view: &mut ViewPane, cols: u16, rows: u16) -> Result<Self, &'static str> {
        if let Some(span) = view.caret_link_span() {
            let url = view
                .source_str()
                .map(|s| s[span.url.0 as usize..span.url.1 as usize].to_string())
                .unwrap_or_default();
            return Ok(Self {
                from: span.url.0,
                to: span.url.1,
                buf: url,
                undos: 0,
            });
        }
        let text = view
            .selected_text()
            .filter(|t| !t.trim().is_empty())
            .ok_or("no link here \u{2014} select some words and press Cmd+K to make one")?;
        // The scaffold is a normal edit: recorded, undoable, and a save right now would write
        // exactly what is on screen.
        view.insert(&format!("[{text}]()"), cols, rows);
        let at = view.caret_at.ok_or("could not place the link")?;
        // The caret lands after the `)`, so the empty URL is the byte before it.
        let empty = at.saturating_sub(1);
        Ok(Self {
            from: empty,
            to: empty,
            buf: String::new(),
            // TWO changes, not one: `insert` over a selection deletes it and then splices, and
            // the two do not coalesce into a run (one removes text, the other adds it). A
            // single undo here left the selected words deleted and the link gone — the file
            // then reads `Read  today.` and nobody typed that.
            undos: 2,
        })
    }

    /// Enter: write what was typed over the URL's bytes, as one undoable splice.
    pub(crate) fn apply(self, view: &mut ViewPane, cols: u16, rows: u16) {
        view.replace_range(self.from, self.to, &self.buf, cols, rows);
    }

    /// Esc: leave the document exactly as it was — including taking back a scaffold this field
    /// put there when it opened.
    pub(crate) fn cancel(self, view: &mut ViewPane, cols: u16, rows: u16) {
        for _ in 0..self.undos {
            view.undo(cols, rows);
        }
    }

    /// Take one key. `Some(true)` applied, `Some(false)` cancelled, `None` still typing.
    pub(crate) fn take(&mut self, k: FieldKey) -> Option<bool> {
        match k {
            FieldKey::Type(s) => self.buf.push_str(&s),
            FieldKey::Backspace => {
                self.buf.pop();
            }
            FieldKey::Apply => return Some(true),
            FieldKey::Cancel => return Some(false),
        }
        None
    }

    /// What the frame says while this is open: the URL as typed, with a caret after it.
    pub(crate) fn legend(&self) -> String {
        format!("\u{2192} {}\u{258f}", self.buf)
    }
}

impl super::DocWindow {
    /// Cmd+K. Says so on the frame when there was nothing to link.
    pub(crate) fn open_link_field(&mut self, cols: u16, rows: u16) {
        match UrlEdit::open(&mut self.view, cols, rows) {
            Ok(f) => self.link = Some(f),
            Err(why) => self.hint = Some(why),
        }
    }

    /// Answer one key while the field is open. `true` when the key was the field's.
    pub(crate) fn link_field_key(
        &mut self,
        key: &Key,
        pressed: bool,
        cols: u16,
        rows: u16,
    ) -> bool {
        let Some(f) = &mut self.link else {
            return false;
        };
        let Some(k) = field_key(key, pressed) else {
            return false;
        };
        match f.take(k) {
            Some(true) => {
                let f = self.link.take().expect("just borrowed");
                f.apply(&mut self.view, cols, rows);
                self.warned = false;
            }
            Some(false) => {
                let f = self.link.take().expect("just borrowed");
                f.cancel(&mut self.view, cols, rows);
            }
            None => {}
        }
        self.window.request_redraw();
        true
    }

    /// The frame's line while the field is open.
    pub(crate) fn link_field_legend(&self) -> Option<String> {
        Some(self.link.as_ref()?.legend())
    }
}

#[cfg(test)]
#[path = "linkedit_tests.rs"]
mod tests;
