//! The page a pane shows before it is signed in: how to make the Google
//! client crew needs, a key per console page in the order they come, and the
//! client crew found.
//!
//! Gmail's read scope is restricted, so no app may read mail for everyone
//! without Google's review; each user makes a client of their own (goal
//! 2026-10-20-inbox, phase 0). Publishing it matters: an app left in
//! "Testing" has its sign-in ended by Google after seven days.
use std::path::{Path, PathBuf};

use super::draw::{wrap, Canvas};
use super::grant::{downloaded_client, Client};

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

/// The client downloaded last into the Downloads folder, if any.
pub(crate) fn look() -> Option<(PathBuf, Client)> {
    downloaded_client(&dirs::download_dir()?)
}

pub(crate) fn draw(cv: &mut Canvas, found: Option<&(PathBuf, Client)>) {
    let t = crew_theme::theme();
    let accent = crate::palette::accent();
    let width = cv.cols.saturating_sub(4) as usize;
    let lead = "Gmail in this pane, read-only. It needs a Google OAuth client of your own \
                (Google lets no app read everyone's mail without a review): about five \
                minutes, once.";
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
}

fn name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "the pasted client".into())
}
