//! What happens as you TYPE into the crew pane's composer: the key handler,
//! and what each key does to the pane.
//!
//! Its sibling [`crate::chatghost`] holds what the composer OFFERS while you
//! type — the ghost completion and the palette that follows the text.
//!
//! Split out of [`crate::chat`] for the line cap. The composer's own state
//! (the field, its history, the palette) lives in its modules; this is the
//! `ChatPane` end of it — the one place a keystroke turns into a change.
use crate::chat::ChatPane;
use crate::chatinput::input_reduce;
use crate::chatkeys::{chat_key, ChatAction, ChatInput};
use crate::chatlayout::Message;
use winit::event::KeyEvent;

impl ChatPane {
    /// Handle a winit key event. Returns [`ChatAction::Close`] when the user asks
    /// to close the pane (Escape) — mirroring the Far/Settings panes. While the
    /// @file popup is open it gets keys first (Escape then closes the popup, not
    /// the pane). `shift` makes Enter insert a newline instead of sending.
    /// `cwd` roots mention scanning and expansion. (Ctrl+O's compact-transcript
    /// toggle is handled as a global intercept in `keys.rs`, not here.)
    pub fn on_key(
        &mut self,
        key: &KeyEvent,
        shift: bool,
        ctrl: bool,
        cwd: &std::path::Path,
    ) -> Option<ChatAction> {
        let k = chat_key(&key.logical_key, key.state.is_pressed(), shift, ctrl);
        self.on_input(k, cwd)
    }

    /// Handle a decoded [`ChatInput`] — the testable half of [`on_key`], split
    /// out so the popup-vs-pane key routing can be exercised without
    /// constructing a winit `KeyEvent`.
    pub(crate) fn on_input(&mut self, k: ChatInput, cwd: &std::path::Path) -> Option<ChatAction> {
        // The masked key prompt takes every key before anything else: it may
        // hold a half-typed secret, and letting a key leak past it to the
        // palette or the composer would be the one bug this feature exists
        // to avoid.
        if let Some(entry) = self.keyentry.as_mut() {
            match entry.key(&k) {
                crate::keyentry::KeyOutcome::Consumed => return None,
                crate::keyentry::KeyOutcome::Cancelled => {
                    self.keyentry = None;
                    self.cancel_oauth();
                    return None;
                }
                crate::keyentry::KeyOutcome::Submit(value) => {
                    let var = entry.var.clone();
                    self.keyentry = None;
                    // Answering by hand supersedes the browser: end that flow
                    // FIRST, so the note explaining the now-pointless tab
                    // reads before the one confirming the key.
                    self.cancel_oauth();
                    crate::chatkeystore::store_provider_key(self, &var, &value);
                    return None;
                }
            }
        }
        // Transcript find next (Cmd+F opens it app-side; Ctrl+F here): modal
        // while open — typed chars edit its query, never the composer. It
        // forwards Ctrl+R so histsearch can still open (which closes find:
        // one modal at a time).
        let fk = {
            let visible: Vec<&Message> =
                self.messages.iter().chain(self.streaming.iter()).collect();
            crate::chatfind::popup_key(&mut self.find, &visible, &k)
        };
        match fk {
            crate::chatfind::FindKey::Consumed => {
                if self.find.is_some() {
                    // Modal: nothing else may stay armed underneath, and the
                    // histsearch close restores the composer draft it saved.
                    crate::chathistsearch::close_restoring(&mut self.histsearch, &mut self.input);
                    self.palette = None;
                    self.mention = None;
                }
                return None;
            }
            // The match target moved — the app scrolls the transcript to it
            // (the jump needs grid geometry this handler doesn't have).
            crate::chatfind::FindKey::Jump => return Some(ChatAction::FindJump),
            crate::chatfind::FindKey::Forward => {}
        }
        // Ctrl+R search next: while open it is modal over the palette and
        // mention popups (typed chars edit its query, not the composer), and
        // Ctrl+R itself must open it before the plain-key handling below.
        match crate::chathistsearch::popup_key(
            &mut self.histsearch,
            &mut self.input,
            self.history.lines(),
            &k,
        ) {
            crate::chathistsearch::HistKey::Consumed => {
                // The search is modal over the palette and mention popups —
                // one left armed underneath (the render already hides them)
                // would eat the first key after the search closes. Clearing
                // here covers the popup OPENING and every key while open.
                if self.histsearch.is_some() {
                    self.palette = None;
                    self.mention = None;
                    self.find = None; // one modal at a time
                }
                return None;
            }
            // An accepted entry is a recalled line in the composer now — reset
            // any history walk, and (like Up/Down recall) do NOT re-sync the
            // palette: a recalled `/command` should not pop it open. A popup
            // armed before the search opened must not survive either — its
            // popup_key would swallow the next Enter against the recalled
            // line (same no-popup rule as Up/Down recall).
            crate::chathistsearch::HistKey::Accepted => {
                self.history.edited();
                self.palette = None;
                self.mention = None;
                return None;
            }
            crate::chathistsearch::HistKey::Forward => {}
        }
        // ORDER IS LOAD-BEARING: an open popup must get keys BEFORE the
        // `match k { Close/Up/Down/… }` block below, or Escape would close the
        // pane instead of the popup and arrows would never reach it. The
        // palette (leading token) and mention (mid-line) are mutually
        // exclusive, so their relative order is free — but both must precede
        // the pane's own key handling.
        match crate::chatpalette::popup_key(&mut self.palette, &mut self.input, &k) {
            crate::chatpalette::PaletteKey::Consumed => return None,
            // A filled-but-not-run row: the input changed, so the palette has
            // to be re-synced against it exactly as a typed character would.
            // `/model` fills `/model `, which opens the model picker.
            crate::chatpalette::PaletteKey::Accepted => {
                self.sync_palette(cwd);
                return None;
            }
            // A picked row is a command to run: the palette is closed and the
            // input holds it, so re-enter our own Enter path (no recursion —
            // `self.palette` is `None` now).
            crate::chatpalette::PaletteKey::Submit => return self.on_input(ChatInput::Enter, cwd),
            crate::chatpalette::PaletteKey::Forward => {}
            crate::chatpalette::PaletteKey::NeedsKey(var) => {
                let mut entry = crate::keyentry::KeyEntry::new(var.clone());
                if var == crate::oauth::OPENROUTER_KEY_VAR {
                    // OpenRouter is the one provider with a real third-party
                    // OAuth flow. The paste prompt still opens underneath, so
                    // a failed browser launch is never a dead end.
                    self.oauth = crate::oauth::spawn();
                    entry.set_waiting(self.oauth.is_some());
                }
                self.keyentry = Some(entry);
                return None;
            }
        }
        if matches!(
            crate::chatmention::popup_key(&mut self.mention, &mut self.input, &k),
            crate::chatmention::MentionKey::Consumed
        ) {
            return None;
        }
        let (ch, enter, backspace) = match k {
            ChatInput::Close => {
                // A pending plan owns Esc: it is the foreground question, and
                // answering it is what the user means. Only with an empty
                // composer — half-typed text means they moved on, and Esc
                // should not silently throw away a plan behind it.
                if self.plan_pending && self.input.is_empty() {
                    self.plan_pending = false;
                    // The bare word, not a slash command: the broker's plan
                    // gate matches it deterministically before any model call.
                    self.submit_command("reject".to_string());
                    return None;
                }
                // Esc means "interrupt the running turn" while busy (mirrors
                // Codex/Claude Code); it only means "close the pane" once
                // idle. A dead connection can't be interrupted, so it falls
                // back to closing too — no write to a dead pipe.
                if self.is_busy() && self.connected {
                    self.interrupt();
                    return None;
                }
                return Some(ChatAction::Close);
            }
            // HistSearch/FindNext never reach here (their popup routing above
            // consumes them — opening when closed), but the match must be total.
            ChatInput::Ignore | ChatInput::HistSearch | ChatInput::FindNext => return None,
            // No popup is open (both got these keys first, above), so the
            // arrows mean what they mean in every shell: walk what you already
            // sent. The palette is deliberately NOT re-synced from a recalled
            // line — it would open on any `/command` and then swallow the next
            // Up as popup navigation, which is the opposite of what the user
            // is in the middle of doing. Typing a character re-syncs it as
            // usual.
            ChatInput::Up => {
                self.history.prev(&mut self.input);
                return None;
            }
            ChatInput::Down => {
                self.history.next(&mut self.input);
                return None;
            }
            ChatInput::Complete => {
                // Tab completes the leading token when there is one to
                // complete, and otherwise takes the suggestion — the input
                // bar's exact fallback order, so one key does the obvious
                // thing in both composers.
                match crate::chatcomplete::complete(&self.input, &self.agents) {
                    Some(done) => self.input = done,
                    None => self.accept_ghost(),
                }
                return None;
            }
            ChatInput::Accept => {
                self.accept_ghost();
                return None;
            }
            ChatInput::Char(c) => (Some(c), false, false),
            ChatInput::Newline => (Some('\n'), false, false),
            ChatInput::Enter => {
                // Enter on an empty composer answers the pending plan. With
                // text typed it sends that text as usual — the plan stays
                // pending, since the user plainly had something else to say.
                if self.plan_pending && self.input.is_empty() {
                    self.plan_pending = false;
                    // Same bare-word rule as Esc's "reject" above.
                    self.submit_command("approve".to_string());
                    return None;
                }
                (None, true, false)
            }
            ChatInput::Backspace => (None, false, true),
        };
        if let Some(text) = input_reduce(&mut self.input, ch, enter, backspace) {
            self.scroll = 0; // sending snaps back to the live bottom
                             // Every submitted line, whatever happens to it next: the ones
                             // answered locally (`/theme`, `/export`) and the ones that never
                             // reach the broker are exactly as worth recalling as the rest, so
                             // this records BEFORE any of the intercepts below return.
            self.history.record(&text);
            if text.trim() == "/exit" {
                return Some(ChatAction::Close); // close the pane, like Escape
            }
            if crate::chatexport::intercept(self, &text) {
                return None; // answered locally (e.g. /export)
            }
            match crate::chattheme::intercept(self, &text) {
                // A switch must also be persisted app-side, or it silently
                // reverts on restart.
                crate::chattheme::ThemeIntercept::Switched => {
                    return Some(ChatAction::PersistTheme)
                }
                crate::chattheme::ThemeIntercept::Handled => return None,
                crate::chattheme::ThemeIntercept::NotTheme => {}
            }
            // `/font` needs the renderer, so the app runs it (and echoes the
            // status back here) — sending it to the broker did nothing.
            if let Some(arg) = crate::chatfont::parse(&text) {
                return Some(ChatAction::Font(arg));
            }
            // Note a `/model all <slug>` pick for the recents list — bare
            // bookkeeping, not a `ChatAction`: returning one here would end
            // the call and swallow the send below. The broker still gets the
            // command untouched; `poll` drains this into the config.
            if let Some(slug) = text.strip_prefix("/model all ") {
                let slug = slug.trim();
                if !slug.is_empty() && slug != "default" {
                    self.pending_recent = Some(slug.to_string());
                }
            }
            if !text.is_empty() {
                // Echo the user's own prompt into the transcript, mirroring how
                // agent replies are appended in `poll` (the `PluginEvent::Message`
                // arm). Without this only replies were ever added, so the pane
                // showed output with no matching input. Echo the RAW typed text,
                // not `expanded`: mention expansion appends whole file bodies
                // meant for the broker, which don't belong in the display.
                // Scroll was already snapped to 0 above, so this lands in view.
                self.push_capped(Message {
                    sender: "user".into(),
                    text: text.clone(),
                    ts: chrono::Local::now().timestamp_millis().to_string(),
                    meta: String::new(),
                    usage: None,
                    expanded: false,
                });
                let agent_names: Vec<String> = self.agents.iter().map(|a| a.name.clone()).collect();
                let expanded = crate::chatmention::expand(&text, cwd, &agent_names);
                // A bare `/stop` means stop, and everything waiting behind
                // the run is part of what has to stop — the same reasoning as
                // Esc, which sends exactly this. Typed rather than pressed is
                // not a different intention.
                if crate::chatqueue::is_stop_all(&text) {
                    if let Some(note) = self.drop_queue() {
                        self.push_note(note);
                    }
                }
                // Busy: queue instead of writing to a broker that's still
                // mid-turn — except `/stop`, which must reach it immediately
                // to cancel. Idle: send straight away, as before.
                if self.is_busy() && !crate::chatqueue::is_stop(&text) {
                    self.queued.push_back(expanded);
                } else {
                    self.send_now(expanded);
                }
            }
        } else {
            // A Char/Backspace edit: the composer's text is the user's own
            // again, whether or not it started as a recalled line.
            self.history.edited();
            // Sync the mention popup to the new input.
            let agents = self.agents.clone();
            crate::chatmention::after_edit(&mut self.mention, &self.input, || {
                crate::chatmention::scan_entries(cwd, &agents)
            });
            self.sync_palette(cwd);
        }
        None
    }
}
