//! The mail window's input (`w`, `inboxwin`): its wheel and its chords, as
//! the grid's are. A slow trackpad's fractions add up, and a Cmd chord is a
//! command, never the plain letter (Cmd+D there once deleted the thread).
/// Whole lines out of a wheel `delta`, the fraction left over kept in `acc`
/// for the next: a slow trackpad sends deltas of a few pixels each, which
/// rounded alone never moved the mail (24 pixels a line, as the grid's).
pub(crate) fn wheel_lines(acc: &mut f32, delta: winit::event::MouseScrollDelta) -> i32 {
    *acc += match delta {
        winit::event::MouseScrollDelta::LineDelta(_, y) => y,
        winit::event::MouseScrollDelta::PixelDelta(d) => d.y as f32 / 24.0,
    };
    let lines = acc.trunc() as i32;
    *acc -= lines as f32;
    lines
}

/// What a chord does in the mail window, the grid's chords aside: Cmd+W
/// closes it, Cmd+Enter sends the reply, Cmd+V pastes; every other is no
/// key at all, as it is no letter in the grid's panes.
#[derive(Debug, PartialEq)]
pub(crate) enum Chord {
    Close,
    Send,
    Paste,
    Nothing,
}

/// `s`: the chord's character (`keychord::cmd_chord`), empty for a named
/// key; `enter`: the key is Enter.
pub(crate) fn chord(s: &str, enter: bool) -> Chord {
    match s {
        "w" => Chord::Close,
        "v" => Chord::Paste,
        "" if enter => Chord::Send,
        _ => Chord::Nothing,
    }
}

#[cfg(test)]
mod tests {
    use winit::dpi::PhysicalPosition;
    use winit::event::MouseScrollDelta;

    #[test]
    fn a_chord_is_a_command_never_a_letter() {
        use super::{chord, Chord};
        assert_eq!(chord("d", false), Chord::Nothing, "Cmd+D deletes nothing");
        assert_eq!(chord("e", false), Chord::Nothing, "Cmd+E archives nothing");
        assert_eq!(chord("w", false), Chord::Close);
        assert_eq!(chord("v", false), Chord::Paste);
        assert_eq!(chord("", true), Chord::Send, "Cmd+Enter");
        assert_eq!(
            chord("", false),
            Chord::Nothing,
            "Cmd+an arrow sends nothing"
        );
    }

    #[test]
    fn a_slow_trackpad_adds_up_to_lines() {
        let mut acc = 0.0;
        let moved: i32 = (0..10)
            .map(|_| {
                super::wheel_lines(
                    &mut acc,
                    MouseScrollDelta::PixelDelta(PhysicalPosition::new(0.0, 5.0)),
                )
            })
            .sum();
        assert_eq!(moved, 2, "fifty pixels, two lines");
        assert_eq!(
            super::wheel_lines(&mut acc, MouseScrollDelta::LineDelta(0.0, -3.0)),
            -2
        );
    }
}
