//! The inbox's keys. One letter each, as a terminal mail reader has them: the
//! arrows (or j/k) move, Enter reads, `/` searches with Gmail's own syntax,
//! `R` re-lists, `o` opens Gmail in the browser, Esc steps back.
use winit::event::KeyEvent;
use winit::keyboard::{Key, NamedKey};

use super::worker::Ask;
use super::{InboxPane, Stage};

pub enum InboxAction {
    Close,
    Redraw,
    /// `?`: `/keys`, open on the inbox's section.
    Keys,
}

impl InboxPane {
    /// `held`: Ctrl or Cmd is down, which makes Enter send a reply.
    pub fn on_key(
        &mut self,
        event: &KeyEvent,
        cols: u16,
        rows: u16,
        held: bool,
    ) -> Option<InboxAction> {
        let enter = matches!(event.logical_key, Key::Named(NamedKey::Enter));
        match event.state.is_pressed() {
            true if held && enter && self.compose.is_some() && self.ready() => {
                self.send_reply();
                Some(InboxAction::Redraw)
            }
            true => self.press(&event.logical_key, cols, rows),
            false => None,
        }
    }

    /// One key, pressed (`press` wraps it): the pane's whole keyboard.
    pub(crate) fn press_key(&mut self, key: &Key, cols: u16, rows: u16) -> Option<InboxAction> {
        // A reply box waits out a sign-in: the setup page's keys are its own.
        if self.compose.is_some() && self.reading.is_some() && self.ready() {
            self.compose_key(key);
            return Some(InboxAction::Redraw);
        }
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
        if ch == "?" {
            return Some(InboxAction::Keys);
        }
        if let Some(r) = self.reading.as_mut().filter(|_| ch != "u") {
            r.unsub_armed = false;
        }
        let named = |n: NamedKey| matches!(key, Key::Named(k) if *k == n);
        if matches!(self.stage, Stage::Ready(_)) && (self.triage(&ch) || self.handoff(&ch)) {
            return Some(InboxAction::Redraw);
        }
        match &mut self.stage {
            Stage::Setup { found } => match (ch.as_str(), found.clone()) {
                (n @ ("1" | "2" | "3" | "4" | "5"), _) => {
                    let i = n.parse::<usize>().unwrap_or(1) - 1;
                    crate::linkclick::open_external(super::setup::STEPS[i].1);
                    None
                }
                (_, Some((_, client))) if named(NamedKey::Enter) => {
                    match super::signin::spawn(client) {
                        Some(rx) => self.stage = Stage::SigningIn(rx),
                        None => self.note = "could not open a port for the sign-in".into(),
                    }
                    Some(InboxAction::Redraw)
                }
                (_, None) if named(NamedKey::Enter) => {
                    self.note =
                        "no client yet: download its JSON (step 5), or paste it here".into();
                    Some(InboxAction::Redraw)
                }
                _ if named(NamedKey::Escape) => self.leave_setup(),
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
            Stage::Ready(_) => self.list_key(key, &ch, cols, rows),
        }
    }

    fn list_key(&mut self, key: &Key, ch: &str, cols: u16, rows: u16) -> Option<InboxAction> {
        let named = |n: NamedKey| matches!(key, Key::Named(k) if *k == n);
        if named(NamedKey::ArrowUp) || ch == "k" {
            self.select(self.sel.saturating_sub(1));
        } else if named(NamedKey::ArrowDown) || ch == "j" {
            self.select((self.sel + 1).min(self.rows.len().saturating_sub(1)));
            if self.sel + 1 >= self.rows.len() && self.more && !self.busy {
                self.ask(Ask::More);
            }
        } else if named(NamedKey::Enter) {
            if !self.open_at(self.sel) {
                return None;
            }
        } else if ch == "/" {
            self.typing = Some(self.query.clone());
        } else if ch == "R" {
            self.relist();
        } else if ch == "o" {
            self.browse(None);
        } else if ch == "L" {
            self.note = self.flip_density(self.body_rows(cols, rows)).into();
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
        let page = self.reader_rows(cols, rows).saturating_sub(1).max(1) as usize;
        let (_, width) = self.reader_box(cols);
        let r = self.reading.as_mut()?;
        let last = super::reader::height(r, width).saturating_sub(page);
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
        } else if matches!(ch, "n" | "p" | "J" | "K") {
            self.note = self.next_thread(matches!(ch, "n" | "J")).into();
        } else if ch == "x" {
            self.note = r.fold_all().into();
        } else if ch == "w" {
            r.pop_out = true;
        } else if ch == "i" {
            self.note = self.show_web();
        } else if ch == "u" {
            self.note = self.unsubscribe();
        } else if ch == "v" || ch == "q" {
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
        self.failed = None;
        self.ask(Ask::List(self.query.clone()));
    }

    /// Gmail in the browser, on this account: the thread, or the list.
    fn browse(&self, thread: Option<&str>) {
        let who = super::query::enc(&self.email);
        let at = thread.map_or_else(|| "#inbox".to_string(), |id| format!("#all/{id}"));
        crate::linkclick::open_external(&format!(
            "https://mail.google.com/mail/?authuser={who}{at}"
        ));
    }
}
