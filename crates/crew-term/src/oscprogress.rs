//! The OSC 9;4 progress report (ConEmu / Windows Terminal), split from `osc`
//! for the line cap.

/// What a program said about its own progress (OSC 9;4).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Progress {
    /// `0..=100`, or `None` while the work is indeterminate.
    pub percent: Option<u8>,
    /// The program called this an error or a warning rather than plain work.
    pub alarm: bool,
}

impl Progress {
    /// `state ; percent` — state 0 clears, 1 sets, 2 is an error, 3 is
    /// indeterminate, 4 is a warning (the ConEmu/Windows Terminal ladder).
    pub(crate) fn parse(rest: &str) -> Option<Progress> {
        let (state, pct) = rest.split_once(';').unwrap_or((rest, ""));
        let pct = pct.trim().parse::<u8>().ok().map(|p| p.min(100));
        match state.trim() {
            "0" => None,
            "3" => Some(Progress {
                percent: None,
                alarm: false,
            }),
            "2" | "4" => Some(Progress {
                percent: pct,
                alarm: true,
            }),
            _ => Some(Progress {
                percent: Some(pct?),
                alarm: false,
            }),
        }
    }
}
