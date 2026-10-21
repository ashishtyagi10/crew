//! What the pane does with each thing that comes back: a finished sign-in,
//! and the worker's answers.
use super::grant::{self, Grant};
use super::setup;
use super::worker::{self, Ask, Got};
use super::{InboxPane, Stage};

/// How often a signed-in pane asks whether the mailbox moved (2 units).
const CHECK_MS: u64 = 60_000;
/// How often the setup page looks in Downloads for the client file.
const LOOK_MS: u64 = 2_000;

impl InboxPane {
    /// Drain what came back; true when the pane should repaint.
    pub fn poll(&mut self) -> bool {
        let now = crate::anim::now_ms();
        let due = now.saturating_sub(self.ticked_ms) >= self.tick_ms();
        if due {
            self.ticked_ms = now;
        }
        match &mut self.stage {
            Stage::Setup { found } if due && found.is_none() => {
                *found = setup::look();
                found.is_some()
            }
            Stage::Setup { .. } => false,
            Stage::SigningIn(rx) => match rx.try_recv() {
                Ok(got) => self.signed(got),
                Err(_) => false,
            },
            Stage::Ready(w) => {
                let got: Vec<Got> = w.rx.try_iter().collect();
                let changed = !got.is_empty();
                for g in got {
                    self.take(g);
                }
                let landed = self.pics.drain();
                if landed {
                    self.sync_sizes();
                }
                let changed = landed || changed;
                if due && !self.busy {
                    self.ask(Ask::Check);
                }
                changed
            }
        }
    }

    fn tick_ms(&self) -> u64 {
        match self.stage {
            Stage::Setup { .. } => LOOK_MS,
            _ => CHECK_MS,
        }
    }

    /// The browser sign-in ended: keep the grant and start listing, or say why
    /// not and go back to setup.
    pub(crate) fn signed(&mut self, got: Result<Grant, String>) -> bool {
        match got {
            Ok(g) => {
                self.note = match grant::save(&g) {
                    Ok(()) => "signed in".into(),
                    Err(e) => format!("signed in, but the grant was not saved: {e}"),
                };
                self.stage = Stage::Ready(worker::start(g));
                self.ask(Ask::List(self.query.clone()));
            }
            Err(e) => {
                self.note = format!("sign-in failed: {e}");
                self.stage = Stage::Setup {
                    found: setup::look(),
                };
            }
        }
        true
    }

    pub(crate) fn take(&mut self, got: Got) {
        match got {
            Got::Account(email) => {
                if let Some(mut g) = grant::load().filter(|g| g.email != email) {
                    g.email.clone_from(&email);
                    let _ = grant::save(&g);
                }
                self.email = email;
            }
            Got::Page {
                query,
                rows,
                next,
                more,
            } => {
                if query != self.query {
                    return; // an older search's page, overtaken
                }
                self.busy = false;
                self.more = next.is_some();
                match more {
                    true => self.rows.extend(rows),
                    false => {
                        let keep = self.rows.get(self.sel).map(|t| t.id.clone());
                        self.rows = rows;
                        self.sel = keep
                            .and_then(|id| self.rows.iter().position(|t| t.id == id))
                            .unwrap_or(0);
                    }
                }
                self.note.clear();
                self.failed = None;
                self.checked_ms = Some(chrono::Utc::now().timestamp_millis());
            }
            Got::Unread(n) => self.unread = Some(n),
            Got::Checked => self.checked_ms = Some(chrono::Utc::now().timestamp_millis()),
            Got::Opened(id, mails) => {
                self.busy = false;
                if let Some(r) = self.reading.as_mut().filter(|r| r.thread.id == id) {
                    r.mails = Some(mails);
                    if let Some(then) = self.then.take() {
                        self.act(then);
                    }
                    self.fetch_carried();
                    self.fetch_web();
                }
            }
            Got::Picture(key, bm) => self.pics.set(key, bm),
            Got::Saved(Ok(path)) => {
                let name = path.file_name().map(|n| n.to_string_lossy().into_owned());
                self.note = format!(
                    "saved {} to Downloads \u{2014} opening it",
                    name.unwrap_or_default()
                );
                crate::linkclick::open_external(&path.to_string_lossy());
            }
            Got::Saved(Err(e)) => self.note = e,
            Got::Drafted(subject) => {
                self.note = format!(
                    "draft saved in Gmail: \u{201c}{subject}\u{201d} \u{2014} review and send it there (o)"
                );
            }
            Got::Unsubscribed(who) => {
                self.note = format!("unsubscribed from {who} \u{b7} e archives this mail");
            }
            Got::Failed(e) if self.rows.is_empty() && self.reading.is_none() => {
                // Nothing listed: the list's own place says what went wrong.
                self.busy = false;
                self.failed = Some(e);
            }
            Got::Failed(e) => {
                self.busy = false;
                // A grant made for the read-only scope (v0.27.29) cannot file
                // or draft, and Google adds no scope to an installed app's
                // grant without a fresh consent.
                self.note = match e.contains("insufficient authentication scopes") {
                    true => {
                        "filing and drafts need a new sign-in: /inbox signout, then Enter".into()
                    }
                    false => e,
                };
            }
            Got::Ended(e) => {
                grant::clear();
                self.busy = false;
                self.note = format!(
                    "Google ended the sign-in ({e}). An app left in Testing is \
                     signed out after 7 days: Audience \u{2192} Publish app keeps it"
                );
                self.stage = Stage::Setup {
                    found: setup::look(),
                };
            }
        }
    }
}
