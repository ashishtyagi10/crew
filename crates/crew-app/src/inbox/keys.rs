//! The inbox's keys. One letter each, as a terminal mail reader has them: the
//! arrows (or j/k) move, Enter reads, `/` searches with Gmail's own syntax,
//! `R` re-lists, `o` opens Gmail in the browser, Esc steps back.
use winit::event::KeyEvent;
use winit::keyboard::{Key, NamedKey};

use super::worker::Ask;
use super::{InboxPane, Reading, Stage};

pub enum InboxAction {
    Close,
    Redraw,
}

impl InboxPane {
    pub fn on_key(&mut self, event: &KeyEvent, cols: u16, rows: u16) -> Option<InboxAction> {
        match event.state.is_pressed() {
            true => self.press(&event.logical_key, cols, rows),
            false => None,
        }
    }

    /// One key, pressed: the pane's whole keyboard, testable without winit.
    pub(crate) fn press(&mut self, key: &Key, cols: u16, rows: u16) -> Option<InboxAction> {
        let ch = match key {
            Key::Character(c) => c.as_str().to_string(),
            Key::Named(NamedKey::Space) => " ".into(),
            _ => String::new(),
        };
        if let Some(q) = self.typing.as_mut() {
            match key {
                Key::Named(NamedKey::Escape) => self.typing = None,
                Key::Named(NamedKey::Backspace) => drop(q.pop()),
                Key::Named(NamedKey::Enter) => {
                    self.query = self.typing.take().unwrap_or_default().trim().to_string();
                    self.relist();
                }
                _ if !ch.is_empty() => q.push_str(&ch),
                _ => return None,
            }
            return Some(InboxAction::Redraw);
        }
        let named = |n: NamedKey| matches!(key, Key::Named(k) if *k == n);
        if matches!(self.stage, Stage::Ready(_)) && (self.triage(&ch) || self.handoff(&ch)) {
            return Some(InboxAction::Redraw);
        }
        match &mut self.stage {
            Stage::Setup { found } => match (ch.as_str(), found.clone()) {
                (n @ ("1" | "2" | "3" | "4" | "5"), _) => {
                    let i = n.parse::<usize>().unwrap_or(1) - 1;
                    let _ = open::that_detached(super::setup::STEPS[i].1);
                    None
                }
                (_, Some((_, client))) if named(NamedKey::Enter) => {
                    match super::signin::spawn(client) {
                        Some(rx) => self.stage = Stage::SigningIn(rx),
                        None => self.note = "could not open a port for the sign-in".into(),
                    }
                    Some(InboxAction::Redraw)
                }
                _ if named(NamedKey::Escape) => Some(InboxAction::Close),
                _ => None,
            },
            Stage::SigningIn(_) if named(NamedKey::Escape) => {
                self.stage = Stage::Setup {
                    found: super::setup::look(),
                };
                Some(InboxAction::Redraw)
            }
            Stage::SigningIn(_) => None,
            Stage::Ready(_) if self.reading.is_some() => self.reading_key(key, &ch, cols, rows),
            Stage::Ready(_) => self.list_key(key, &ch),
        }
    }

    fn list_key(&mut self, key: &Key, ch: &str) -> Option<InboxAction> {
        let named = |n: NamedKey| matches!(key, Key::Named(k) if *k == n);
        if named(NamedKey::ArrowUp) || ch == "k" {
            self.sel = self.sel.saturating_sub(1);
        } else if named(NamedKey::ArrowDown) || ch == "j" {
            self.sel = (self.sel + 1).min(self.rows.len().saturating_sub(1));
            if self.sel + 1 >= self.rows.len() && self.more && !self.busy {
                self.ask(Ask::More);
            }
        } else if named(NamedKey::Enter) {
            let thread = self.rows.get(self.sel)?.clone();
            self.ask(Ask::Open(thread.id.clone()));
            self.opened(&thread.id);
            self.reading = Some(Reading::new(thread));
        } else if ch == "/" {
            self.typing = Some(self.query.clone());
        } else if ch == "R" {
            self.relist();
        } else if ch == "o" {
            self.browse(None);
        } else if named(NamedKey::Escape) && !self.query.is_empty() {
            self.query.clear();
            self.relist();
        } else if named(NamedKey::Escape) {
            return Some(InboxAction::Close);
        } else {
            return None;
        }
        Some(InboxAction::Redraw)
    }

    fn reading_key(&mut self, key: &Key, ch: &str, cols: u16, rows: u16) -> Option<InboxAction> {
        let named = |n: NamedKey| matches!(key, Key::Named(k) if *k == n);
        let page = rows.saturating_sub(2).max(1) as usize;
        let r = self.reading.as_mut()?;
        let last = super::reader::height(r, cols).saturating_sub(page);
        if named(NamedKey::Escape) || named(NamedKey::Backspace) || named(NamedKey::ArrowLeft) {
            self.reading = None;
        } else if named(NamedKey::ArrowUp) || ch == "k" {
            r.scroll = r.scroll.saturating_sub(1);
        } else if named(NamedKey::ArrowDown) || ch == "j" {
            r.scroll = (r.scroll + 1).min(last);
        } else if named(NamedKey::PageUp) {
            r.scroll = r.scroll.saturating_sub(page);
        } else if named(NamedKey::PageDown) || ch == " " {
            r.scroll = (r.scroll + page).min(last);
        } else if ch == "o" {
            let id = r.thread.id.clone();
            self.browse(Some(&id));
        } else if ch == "v" || ch == "z" {
            self.note = r.toggle(ch == "v").into();
        } else {
            return None;
        }
        Some(InboxAction::Redraw)
    }

    /// Open on `q`, a search in Gmail's own syntax.
    pub(crate) fn search(&mut self, q: &str) {
        self.query = q.trim().to_string();
        self.relist();
    }

    /// The first page of the current search, from the top.
    fn relist(&mut self) {
        self.rows.clear();
        self.sel = 0;
        self.ask(Ask::List(self.query.clone()));
    }

    /// Gmail in the browser, on this account: the thread, or the list.
    fn browse(&self, thread: Option<&str>) {
        let who = super::query::enc(&self.email);
        let at = thread.map_or_else(|| "#inbox".to_string(), |id| format!("#all/{id}"));
        let _ = open::that_detached(format!("https://mail.google.com/mail/?authuser={who}{at}"));
    }

    /// A pasted client (the console's JSON) is the one to sign in with.
    pub fn paste(&mut self, text: &str) {
        match (&mut self.stage, super::grant::client_from_json(text)) {
            (Stage::Setup { found }, Some(c)) => *found = Some(("pasted client".into(), c)),
            (_, _) if self.typing.is_some() => {
                self.typing.get_or_insert_default().push_str(text.trim())
            }
            _ => {}
        }
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
            "enter read \u{b7} r reply \u{b7} e archive \u{b7} d delete \u{b7} u undo \u{b7} a ask smith \u{b7} / search \u{b7} m read \u{b7} s star \u{b7} t todo \u{b7} R refresh \u{b7} o gmail".into()
        }
    }
}

impl InboxPane {
    /// The wheel moves the selection in the list and scrolls the reader.
    pub fn scroll_wheel(&mut self, lines: i32, cols: u16, rows: u16) {
        let (up, n) = (lines > 0, lines.unsigned_abs() as usize);
        let key = Key::Named(if up {
            NamedKey::ArrowUp
        } else {
            NamedKey::ArrowDown
        });
        for _ in 0..n.min(10) {
            self.press(&key, cols, rows);
        }
    }
}
