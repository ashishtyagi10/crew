//! Hold a modifier, see what it does — contextual shortcut hints.
//!
//! `/keys` is a manual: fifty bindings in one scrolling column, correct and
//! unreadable in the two seconds anyone actually has. It answers "what can
//! crew do", which is a question people ask once. The question they ask
//! constantly is narrower — *I have my thumb on Cmd; what are my options
//! right now?* — and the answer depends on what is focused, which a static
//! table cannot know.
//!
//! So: rest on a bare modifier and a single row of chips appears above the
//! input bar, naming what that modifier reaches from where you are. Press
//! anything, or let go, and it is gone. It teaches by being in the way of
//! nothing: you were already holding the key.
//!
//! Two rules keep it from becoming noise. It only opens on a modifier held
//! **alone** — a chord in progress is someone who already knows what they are
//! doing. And it waits out [`DWELL_MS`] first, so an ordinary Cmd+C never
//! flashes a panel on its way past.
use winit::keyboard::ModifiersState;

/// How long a modifier must be held alone before the hints appear.
///
/// Long enough that a deliberate chord (press, strike, release — well under a
/// fifth of a second for a practised hand) never triggers it, short enough
/// that hesitating is answered rather than punished.
pub(crate) const DWELL_MS: u64 = 450;

/// Which modifier is being rested on, if it is one crew has hints for.
///
/// `Shift` is deliberately absent: it is half of a dozen chords but reaches
/// nothing on its own, and it is held while typing capitals, which would make
/// the panel flicker through ordinary prose.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Held {
    /// `Cmd` on macOS, `Ctrl` elsewhere — the primary action modifier.
    Primary,
    /// `Ctrl` on macOS (pane cycling and the theme/gradient walks live here),
    /// `Alt` elsewhere.
    Secondary,
}

/// The modifier held alone, if any. Any combination of two is a chord in
/// progress, and a chord's author is not the person this is for.
pub(crate) fn held_alone(m: ModifiersState) -> Option<Held> {
    let (primary, secondary) = if cfg!(target_os = "macos") {
        (m.super_key(), m.control_key())
    } else {
        (m.control_key(), m.alt_key())
    };
    // Shift rides along with several of these chords, so it is not counted as
    // company; the other three are.
    let others = usize::from(primary)
        + usize::from(secondary)
        + usize::from(if cfg!(target_os = "macos") {
            m.alt_key()
        } else {
            m.super_key()
        });
    if others != 1 {
        return None;
    }
    if primary {
        Some(Held::Primary)
    } else if secondary {
        Some(Held::Secondary)
    } else {
        None
    }
}

/// What the user is looking at, since that is what changes the answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Where {
    /// The input bar has focus.
    Input,
    /// An agent/chat pane is focused.
    Chat,
    /// A terminal pane is focused.
    Terminal,
    /// Anything else focused — a viewer, settings, the file browser.
    Other,
}

/// The display name of the primary modifier on this platform.
pub(crate) fn primary_name() -> &'static str {
    if cfg!(target_os = "macos") {
        "Cmd"
    } else {
        "Ctrl"
    }
}

/// The display name of the secondary modifier on this platform.
pub(crate) fn secondary_name() -> &'static str {
    if cfg!(target_os = "macos") {
        "Ctrl"
    } else {
        "Alt"
    }
}

/// The chips to show for `held` from `at`, in priority order — the row is
/// clipped from the right when the window is narrow, so the most useful hint
/// has to come first.
///
/// Deliberately short. This is a glance, not the manual: `/keys` is one row
/// away and says so at the end of every list.
pub(crate) fn chips(held: Held, at: Where) -> Vec<(&'static str, &'static str)> {
    match held {
        Held::Primary => {
            let mut v = vec![
                ("1\u{2026}9", "pane"),
                ("\u{2190}\u{2191}\u{2192}\u{2193}", "focus"),
                ("I", "input"),
                ("T", "shell"),
                ("J", "chat"),
            ];
            match at {
                Where::Input => v.push(("\u{21b5}", "send")),
                Where::Chat | Where::Terminal => {
                    v.push(("K", "clear"));
                    v.push(("Z", "zoom"));
                    v.push(("W", "close"));
                }
                Where::Other => v.push(("W", "close")),
            }
            v.push((",", "settings"));
            v
        }
        Held::Secondary => vec![
            ("Tab", "next pane"),
            ("Shift+L", "theme"),
            ("Shift+G", "gradient"),
            ("Shift+P", "palette"),
        ],
    }
}

/// The row as one line of text: `Cmd  1…9 pane · ←↑→↓ focus · …`.
///
/// Built here rather than in the renderer so the width the layout reserves and
/// the string that gets drawn are the same string — a hint panel sized from a
/// different measurement than its content is the `/keys` truncation bug again.
pub(crate) fn line(held: Held, at: Where) -> String {
    let name = match held {
        Held::Primary => primary_name(),
        Held::Secondary => secondary_name(),
    };
    let body = chips(held, at)
        .into_iter()
        .map(|(k, d)| format!("{k} {d}"))
        .collect::<Vec<_>>()
        .join("  \u{b7}  ");
    format!("{name}   {body}")
}

/// The hint row as a 3-row fieldset card, `cols` wide.
///
/// A card and not a bare row: crew draws nothing loose on the page, and the
/// legend is where the row says which modifier it is answering for.
pub(crate) fn peek_card(text: &str, cols: u16) -> Vec<crew_render::CellView> {
    let t = crew_theme::theme();
    let border = crate::palette::accent();
    let mut cells = crate::modernring::gradient_card(cols, 3, "hold", border, border, t.page_bg);
    let body = crate::chatwidth::clip_w(text, cols.saturating_sub(4) as usize);
    crate::chatwidth::place_row(
        2,
        cols - 1,
        body.chars().map(|c| (c, t.ink)),
        |col, c, fg| {
            cells.push(crew_render::CellView {
                col,
                row: 1,
                c,
                fg,
                bg: t.page_bg,
                bold: false,
                italic: false,
            });
        },
    );
    cells
}

impl crate::app::CrewApp {
    /// Whether the hint row should be on screen right now: a bare modifier
    /// held past [`DWELL_MS`].
    pub(crate) fn peek_open(&self, now: u64) -> bool {
        self.peek_since
            .is_some_and(|t| now.saturating_sub(t) >= DWELL_MS)
            && held_alone(self.mods.state()).is_some()
    }

    /// The row to draw, or `None` when it is not open.
    pub(crate) fn peek_line(&self, now: u64) -> Option<String> {
        if !self.peek_open(now) {
            return None;
        }
        Some(line(held_alone(self.mods.state())?, self.peek_where()))
    }

    /// Where the user is, which is what makes these hints worth having over
    /// the `/keys` table.
    fn peek_where(&self) -> Where {
        if self.input.focused {
            return Where::Input;
        }
        match self.panes.get(self.focused).map(|p| &p.content) {
            Some(crate::pane::PaneContent::Chat(_)) => Where::Chat,
            Some(crate::pane::PaneContent::Terminal(_)) => Where::Terminal,
            _ => Where::Other,
        }
    }
}

#[cfg(test)]
#[path = "keypeek_tests.rs"]
mod tests;
