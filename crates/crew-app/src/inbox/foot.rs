//! The pane's last line: what the keys do where you are, or what just
//! happened, or the search being typed.
use super::draw::Canvas;
use super::{InboxPane, Stage};

impl InboxPane {
    pub(crate) fn foot(&self, cv: &mut Canvas) {
        let t = crew_theme::theme();
        let last = cv.rows.saturating_sub(1);
        let (text, fg) = match (&self.typing, self.note.is_empty()) {
            (Some(q), _) => (format!("search: {q}\u{258f}"), crate::palette::accent()),
            (None, false) => (self.note.clone(), t.bell),
            (None, true) => (hint(self), t.text_muted),
        };
        cv.text(last, 1, cv.cols, &text, fg, false);
    }
}

/// The foot line's key list for where the pane is.
pub(crate) fn hint(p: &InboxPane) -> String {
    match (&p.stage, &p.reading) {
        (Stage::Setup { .. }, _) => "1\u{2013}5 open the console \u{b7} enter sign in \u{b7} esc close".into(),
        (Stage::SigningIn(_), _) => "esc cancel".into(),
        (Stage::Ready(_), Some(_)) => {
            "esc back \u{b7} r reply \u{b7} e archive \u{b7} d delete \u{b7} a ask smith \u{b7} t todo \u{b7} z quotes \u{b7} v text \u{b7} m unread \u{b7} s star \u{b7} o gmail".into()
        }
        (Stage::Ready(_), None) if p.busy => "loading\u{2026}".into(),
        (Stage::Ready(_), None) => {
            "enter read \u{b7} r reply \u{b7} e archive \u{b7} d delete \u{b7} u undo \u{b7} a ask smith \u{b7} / search \u{b7} m read \u{b7} s star \u{b7} t todo \u{b7} R refresh \u{b7} o gmail \u{b7} L lines".into()
        }
    }
}
