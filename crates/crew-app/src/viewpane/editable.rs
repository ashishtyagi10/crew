//! Which files a viewer can be typed into, and why not when it cannot.
//!
//! Every text file can: rendered markdown carries its bytes from the parser (`crate::md::source`),
//! and the code and plain rungs stamp theirs as they cut lines into rows ([`super::srcstamp`]).
//! What is left out is what has no bytes to type into — a picture, a binary file's card, the text
//! a PDF was turned into — and every view that is not the file: a rendered table, a diff.
//!
//! **A save writes the whole buffer over the file**, so a buffer that is not the whole file must
//! never be edited: one cut at the read cap, one whose render stopped at the line cap, or one read
//! from bytes that were not UTF-8 (each bad byte became U+FFFD, and saving would write that).
use super::detect::Format;
use super::{LoadState, ViewPane};

/// Whether the rung `format` draws (with `raw`, the source view `s` toggles) carries the bytes a
/// caret is made of.
pub(crate) fn typed_in(format: Format, raw: bool) -> bool {
    match format {
        Format::Markdown | Format::Code { .. } | Format::Data { .. } | Format::Text => true,
        Format::Csv { .. } => raw,
        _ => false,
    }
}

/// A pane's frame legend: a viewer's says whether it is being edited or can be
/// ([`ViewPane::legend`]); a name the user gave it is kept as it is.
pub(crate) fn pane_legend(p: &crate::pane::Pane, focused: bool) -> String {
    match &p.content {
        crate::pane::PaneContent::View(v) if p.name.is_none() => v.legend(focused),
        _ => p.title_text(),
    }
}

impl ViewPane {
    /// Whether this pane's document can be edited here.
    pub(crate) fn editable(&self) -> bool {
        self.edit_refusal().is_none()
    }

    /// Why this pane cannot be edited, or `None` when it can.
    pub(crate) fn edit_refusal(&self) -> Option<&'static str> {
        let LoadState::Ready { format, loaded } = &self.state else {
            return Some(match self.state {
                LoadState::Loading { .. } => "still reading the file",
                _ => "the file could not be read",
            });
        };
        if !typed_in(*format, self.raw) {
            return Some(match format {
                Format::Csv { .. } => "s shows the table's text, which can be edited",
                _ => "this kind of file has no text to edit here — e opens $EDITOR",
            });
        }
        if loaded.truncated.is_some()
            || super::rendercap::cap_render_lines(&loaded.text).1.is_some()
        {
            return Some("only part of this file is shown — e opens $EDITOR");
        }
        if loaded.text.contains('\u{fffd}') {
            return Some("this file is not plain UTF-8 — e opens $EDITOR");
        }
        None
    }

    /// The pane's legend: its [`Self::title`], and — on the focused pane — whether it is being
    /// edited or can be. Editing is asked for (a reader's letters are its commands), so the frame
    /// says how: nothing on screen did, and a reader could not find the way in.
    pub(crate) fn legend(&self, focused: bool) -> String {
        let title = self.title();
        // Per frame, so only the O(1) half of `edit_refusal`: the rare file whose lines or bytes
        // rule it out still wears the cue, and `i` says why.
        let likely = matches!(&self.state, LoadState::Ready { format, loaded }
            if typed_in(*format, self.raw) && loaded.truncated.is_none());
        match (focused, self.caret.is_some(), likely) {
            (true, true, _) => format!("{title} \u{b7} editing"),
            (true, false, true) => format!("{title} \u{b7} i to edit"),
            _ => title,
        }
    }

    /// [`Self::poll`], and — on the tick the text lands in a pane opened to edit — the caret, so
    /// the file is ready to type in as it appears, as in any editor. `true` when anything changed.
    pub(crate) fn poll_landing(&mut self, cols: u16) -> bool {
        let landed = self.poll();
        if landed && std::mem::take(&mut self.edit_on_load) && self.editable() {
            self.start_editing(cols);
        }
        landed
    }

    /// Whether the document is drawn as rendered markdown — where Enter, Tab and Cmd+B speak
    /// markdown — rather than as its lines.
    pub(crate) fn renders_markdown(&self) -> bool {
        matches!(
            &self.state,
            LoadState::Ready {
                format: Format::Markdown,
                ..
            }
        ) && !self.raw
    }
}

#[cfg(test)]
#[path = "editable_tests.rs"]
mod tests;
