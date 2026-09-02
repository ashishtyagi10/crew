//! Off-screen render of the `/tools` listing — the action ledger, opened in
//! the viewer the way a person opens it. `toolsview` and `toolsrow` fit a
//! row into sixty columns by arithmetic; nothing had ever drawn one.
//!
//! `#[ignore]`d (needs a GPU adapter, writes PNGs):
//! `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew tools_shot -- --ignored --nocapture`
use crate::goalshot_tests::dump;
use crate::shotgpu_tests::shot_at;
use crate::viewpane::detect::{detect, Probe};
use crate::viewpane::load::Loaded;
use crate::viewpane::{LoadState, ViewPane};
use crew_plugin::ledger::Record;

/// A fixed 'now' so relative times are deterministic: 2026-08-31T00:00:00Z.
pub(crate) const NOW: u64 = 1_787_788_800_000;

fn rec(
    ago_s: u64,
    tool: &str,
    tier: &str,
    who: &str,
    decision: &str,
    outcome: &str,
    note: &str,
) -> Record {
    Record {
        ts_ms: NOW - ago_s * 1000,
        tool: tool.into(),
        tier: tier.into(),
        requester: who.into(),
        decision: decision.into(),
        outcome: outcome.into(),
        note: note.into(),
    }
}

/// A ledger with one of everything a row can carry: a name too long for its
/// column, a denial with its reason, a call from the phone, a failure with a
/// long error, a timeout, a decision still waiting on its outcome.
pub(crate) fn ledger() -> Vec<Record> {
    vec![
        rec(3_000, "sys:list_dir", "read", "pane", "allow", "ran", ""),
        rec(2_400, "sys:run", "irreversible", "pane", "allow", "ran", ""),
        rec(
            1_900,
            "google_workspace:search_gmail_messages",
            "read",
            "channel:telegram:8812",
            "allow",
            "ran",
            "",
        ),
        rec(
            1_200,
            "sys:write_file",
            "reversible",
            "pane",
            "allow",
            "failed",
            "permission denied (os error 13): /etc/hosts",
        ),
        rec(
            900,
            "gh:merge_pull_request",
            "irreversible",
            "channel:telegram:8812",
            "deny",
            "",
            "irreversible from a channel needs approval",
        ),
        rec(
            600,
            "weather:current",
            "read",
            "trigger:morning",
            "allow",
            "timed_out",
            "",
        ),
        rec(
            300,
            "sys:run",
            "irreversible",
            "pane",
            "ask",
            "",
            "approval a1 pending",
        ),
        rec(45, "sys:read_file", "read", "pane", "allow", "ran", ""),
        rec(2, "sys:run", "irreversible", "pane", "allow", "ran", ""),
    ]
}

/// The listing as `/tools` opens it: written to a `.log`, detected like any
/// other file, so the rung it lands in is the rung the shot draws.
fn pane(text: &str) -> ViewPane {
    let path = crate::lastout::temp_path(usize::MAX, "tools");
    let probe = Probe {
        textutil: false,
        pdftotext: false,
    };
    let format = detect(&path, text.as_bytes(), probe);
    let mut p = ViewPane::open(path);
    p.state = LoadState::Ready {
        format,
        loaded: Loaded {
            text: text.into(),
            truncated: None,
            meta: None,
            image: None,
        },
    };
    p
}

/// Every drawn row is a whole line of the listing, or a piece of one that
/// was broken between words: the plain rung wraps prose on words, and the
/// rows a listing builds to fit the tile must not wrap at all.
pub(crate) fn intact(rows: &[String], text: &str, name: &str) {
    for row in rows.iter().map(|r| r.trim_end()).filter(|r| !r.is_empty()) {
        let piece = row.trim_start();
        let at = text
            .find(piece)
            .unwrap_or_else(|| panic!("{name}: row {row:?} is not a piece of the listing"));
        let after = text[at + piece.len()..].chars().next();
        assert!(
            matches!(after, None | Some(' ') | Some('\n')),
            "{name}: row {row:?} ends mid-word (next char {after:?})"
        );
    }
}

pub(crate) fn tools_shot(name: &str, text: &str, w: u32) -> Option<Vec<String>> {
    let p = pane(text);
    let mut dumped = Vec::new();
    shot_at(name, w, 520, 13.0, "tools", |cols, rows, aspect| {
        let (cells, paint) = p.art(cols, rows, aspect);
        dumped = dump(&cells, cols, rows);
        eprintln!("--- {name} {cols}x{rows}");
        for l in &dumped {
            eprintln!("|{l}");
        }
        (cells, paint)
    })?;
    Some(dumped)
}

#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn tools_shot_wide_and_as_a_tile() {
    let _g = crate::app::theme_test_guard();
    let text = crate::toolsview::listing(&ledger(), 2, "", NOW);
    for (name, w) in [("tools-wide", 1000u32), ("tools-tile", 420)] {
        let Some(rows) = tools_shot(name, &text, w) else {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        };
        let all = rows.join("\n");
        assert!(all.contains("9 call(s)"), "{name}:\n{all}");
        assert!(all.contains("2 unreadable"), "{name}:\n{all}");
        assert!(
            all.contains("4 read \u{b7} 1 reversible \u{b7} 4 irreversible"),
            "{name}:\n{all}"
        );
        assert!(
            all.contains("1 denied \u{b7} 2 failed \u{b7} 1 pending"),
            "{name}:\n{all}"
        );
        // Built to fit the tile: every line the listing wrote is one row of
        // the viewer, whole — and no row wears a line number.
        intact(&rows, &text, name);
        assert!(!all.contains("1 # tools"), "{name} has a gutter:\n{all}");
    }
}

#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn tools_shot_filtered_and_empty() {
    let _g = crate::app::theme_test_guard();
    for (name, text) in [
        (
            "tools-filtered",
            crate::toolsview::listing(&ledger(), 0, "deny", NOW),
        ),
        (
            "tools-nomatch",
            crate::toolsview::listing(&ledger(), 0, "zzz", NOW),
        ),
        ("tools-empty", crate::toolsview::listing(&[], 0, "", NOW)),
    ] {
        if tools_shot(name, &text, 640).is_none() {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        }
    }
}
