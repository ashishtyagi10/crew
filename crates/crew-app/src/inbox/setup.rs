//! The page a pane shows before it is signed in: how to make the Google
//! client crew needs, a key per console page in the order they come, and the
//! client crew found.
//!
//! Gmail's read scope is restricted, so no app may read mail for everyone
//! without Google's review; each user makes a client of their own (goal
//! 2026-10-20-inbox, phase 0). Publishing it matters: an app left in
//! "Testing" has its sign-in ended by Google after seven days.
use std::path::{Path, PathBuf};

use super::compose::Kind;
use super::draw::{wrap, Canvas};
use super::grant::{downloaded_client, Client};
use super::keys::InboxAction;
use super::InboxPane;

/// The console pages, in order (Google's own deep links).
pub(crate) const STEPS: [(&str, &str); 5] = [
    (
        "make a project",
        "https://console.cloud.google.com/projectcreate",
    ),
    (
        "turn on the Gmail API",
        "https://console.cloud.google.com/apis/enableflow;apiid=gmail.googleapis.com",
    ),
    (
        "Branding: an app name and your email; Audience: External",
        "https://console.cloud.google.com/auth/branding",
    ),
    (
        "Audience: Publish app (in Testing, Google signs you out weekly)",
        "https://console.cloud.google.com/auth/audience",
    ),
    (
        "Clients: Create client, Desktop app, then Download JSON",
        "https://console.cloud.google.com/auth/clients",
    ),
];

/// The client downloaded last into the Downloads folder, if any. Tests never
/// read the real Downloads (nor sign in with what is there), as they never
/// read the real grant (`grant::path`).
pub(crate) fn look() -> Option<(PathBuf, Client)> {
    if cfg!(test) {
        return None;
    }
    downloaded_client(&dirs::download_dir()?)
}

/// The page; `kept` names a reply waiting out the sign-in, if one is.
pub(crate) fn draw(cv: &mut Canvas, found: Option<&(PathBuf, Client)>, kept: Option<&str>) {
    let t = crew_theme::theme();
    let accent = crate::palette::accent();
    let width = cv.cols.saturating_sub(4) as usize;
    let lead = "Gmail in this pane: read, file and reply. It needs a Google OAuth client of \
                your own (Google lets no app read everyone's mail without a review): about \
                five minutes, once.";
    let mut r = 1;
    for line in wrap(lead, width) {
        cv.text(r, 2, cv.cols, &line, t.ink, false);
        r += 1;
    }
    r += 1;
    for (i, (what, _)) in STEPS.iter().enumerate() {
        cv.text(r, 2, cv.cols, &format!("{}", i + 1), accent, true);
        cv.text(r, 5, cv.cols, what, t.ink, false);
        r += 1;
    }
    r += 1;
    let keys =
        "Press 1\u{2013}5 to open each page. crew finds the JSON in Downloads, or paste it here.";
    for line in wrap(keys, width) {
        cv.text(r, 2, cv.cols, &line, t.text_muted, false);
        r += 1;
    }
    r += 1;
    match found {
        Some((path, _)) => {
            let line = format!("\u{2713} {} \u{2014} Enter signs in", name(path));
            cv.text(r, 2, cv.cols, &line, accent, true);
        }
        None => {
            let line = "waiting for client_secret_\u{2026}.json in Downloads";
            cv.text(r, 2, cv.cols, line, t.text_muted, false);
        }
    }
    if let Some(kept) = kept {
        let line = format!("{kept} is kept: it opens again once you are signed in");
        for (i, line) in wrap(&line, width).iter().enumerate() {
            cv.text(r + 2 + i as u16, 2, cv.cols, line, t.ink, false);
        }
    }
}

impl InboxPane {
    /// The reply (or forward) waiting out a sign-in, named: "your reply to
    /// Ann Lee".
    pub(super) fn kept(&self) -> Option<String> {
        let c = self.compose.as_ref()?;
        let what = match c.kind {
            Kind::Reply => "your reply",
            Kind::Forward => "your forward",
        };
        let who = super::addr::names(&c.to.text);
        Some(match who.is_empty() {
            true => what.to_string(),
            false => format!("{what} to {who}"),
        })
    }

    /// Esc on the setup page closes the pane, and a reply kept for after the
    /// sign-in went with it, unsaid. With one kept, the first Esc says so and
    /// only a second lets it go, as Esc does over words in the box.
    pub(super) fn leave_setup(&mut self) -> Option<InboxAction> {
        let Some(kept) = self.kept() else {
            return Some(InboxAction::Close);
        };
        match self.compose.as_mut() {
            Some(c) if !c.armed => {
                c.armed = true;
                self.note =
                    format!("{kept} is kept for after the sign-in \u{b7} esc again discards it");
                Some(InboxAction::Redraw)
            }
            _ => Some(InboxAction::Close),
        }
    }
}

/// While Google's sign-in page is open in the browser.
pub(crate) fn signing_in(cv: &mut Canvas) {
    let t = crew_theme::theme();
    let said = "finish signing in in your browser\u{2026}";
    cv.text(1, 2, cv.cols, said, t.ink, false);
    let why = "(crew is listening for Google's answer)";
    cv.text(2, 2, cv.cols, why, t.text_muted, false);
}

fn name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "the pasted client".into())
}

#[cfg(test)]
#[path = "setup_tests.rs"]
mod tests;
