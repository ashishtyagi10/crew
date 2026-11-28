//! Rewind's pop-up: the list of messages, and the one-line confirmation that
//! names every file that is about to go back (`chatrewind`).
use crate::chatrewind::shown;
use crate::chatrewindmarks::{Ask, Rewind};
use crate::popupplace::Popup;
use crate::suggest::MenuItem;

/// The confirmation's rows, each a line of its own so a narrow pane clips the
/// least that matters: the question, `these go back: a.rs, b.rs (yours)
/// +2 more`, what is not certain, and the two keys. The files edited outside
/// crew are marked, because they are the ones whose loss crew cannot take
/// back by itself.
fn lines(a: &Ask, text: &str) -> Vec<String> {
    let list: Vec<String> = a
        .names
        .iter()
        .map(|n| match a.yours.contains(n) {
            true => format!("{n} (yours)"),
            false => n.clone(),
        })
        .collect();
    let more = (a.files as usize).saturating_sub(a.names.len());
    let mut files = format!("these go back: {}", list.join(", "));
    if more > 0 {
        files.push_str(&format!(" +{more} more"));
    }
    let mut out = vec![format!("rewind to \u{201c}{}\u{201d}?", shown(text)), files];
    if a.unsure {
        out.push("may include edits made outside crew".into());
    } else if more > 0 && a.outside && a.yours.len() == a.names.len() {
        out.push("some of the rest are yours too".into());
    }
    out.push("Enter rewinds \u{b7} Esc keeps".into());
    out
}

/// The open list or confirmation as a pop-up above the composer.
pub(crate) fn card(r: &Rewind, cols: u16) -> Option<Popup> {
    if let (Some(a), Some(m)) = (&r.ask, &r.want) {
        let rows: Vec<MenuItem> = lines(a, &m.text)
            .into_iter()
            .map(|label| MenuItem {
                label,
                ..Default::default()
            })
            .collect();
        return Some(crate::cmdmenu::popup("rewind", &rows, 0, cols));
    }
    let sel = r.sel?;
    let now = crate::chattime::unix_now_ms();
    let items: Vec<MenuItem> = r
        .picks()
        .iter()
        .map(|m| MenuItem {
            label: shown(&m.text),
            desc: crate::chattime::rel_time(&m.ts, now).unwrap_or_default(),
            ..Default::default()
        })
        .collect();
    Some(crate::cmdmenu::popup(
        "rewind to\u{2026}",
        &items,
        sel,
        cols,
    ))
}
