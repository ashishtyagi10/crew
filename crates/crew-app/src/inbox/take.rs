//! What the pane does with each thing that comes back: a finished sign-in,
//! and the worker's answers.
use super::grant::{self, Grant};
use super::setup;
use super::worker::{self, Ask, Got};
use super::{InboxPane, Stage};

impl InboxPane {
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
            }
            Got::Unread(n) => self.unread = Some(n),
            Got::Opened(id, mails) => {
                self.busy = false;
                if let Some(r) = self.reading.as_mut().filter(|r| r.thread.id == id) {
                    r.mails = Some(mails);
                    if let Some(then) = self.then.take() {
                        self.act(then);
                    }
                }
            }
            Got::Drafted(subject) => {
                self.note = format!(
                    "draft saved in Gmail: \u{201c}{subject}\u{201d} \u{2014} review and send it there (o)"
                );
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
