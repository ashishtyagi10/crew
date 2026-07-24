#[cfg(test)]
mod reply_tests {
    use super::super::{GridSize, HeadlessTerm, TermModel};

    /// OSC 11 (background query) must be answered from the active theme — agent
    /// CLIs (claude, codex) probe it to pick a light or dark output palette;
    /// unanswered, they assume dark and paint light text onto light themes.
    #[test]
    fn osc11_background_query_is_answered_from_theme() {
        let mut t = HeadlessTerm::new(GridSize { cols: 20, rows: 4 });
        t.feed(b"\x1b]11;?\x1b\\");
        let reply = t.take_replies().expect("background query answered");
        let (r, g, b) = crew_theme::theme().term_bg;
        assert_eq!(
            reply,
            format!("\x1b]11;rgb:{r:02x}{r:02x}/{g:02x}{g:02x}/{b:02x}{b:02x}\x1b\\")
        );
        assert_eq!(t.take_replies(), None, "replies drain on take");
    }

    /// OSC 10 (foreground query) answers with the theme's terminal foreground.
    #[test]
    fn osc10_foreground_query_is_answered_from_theme() {
        let mut t = HeadlessTerm::new(GridSize { cols: 20, rows: 4 });
        t.feed(b"\x1b]10;?\x1b\\");
        let reply = t.take_replies().expect("foreground query answered");
        let (r, _, _) = crew_theme::theme().term_fg;
        assert!(reply.starts_with("\x1b]10;rgb:"), "{reply:?}");
        assert!(reply.contains(&format!("{r:02x}{r:02x}/")), "{reply:?}");
    }

    /// DSR 6 (cursor position report) flows back through `PtyWrite`.
    #[test]
    fn dsr_cursor_position_is_reported() {
        let mut t = HeadlessTerm::new(GridSize { cols: 20, rows: 4 });
        t.feed(b"ab\x1b[6n");
        assert_eq!(t.take_replies().as_deref(), Some("\x1b[1;3R"));
    }

    /// A program painting truecolor text at (or near) the terminal background —
    /// a dark-theme palette left running across a live switch to a light theme,
    /// say — must still render legibly: the contrast floor nudges the fg.
    #[test]
    fn near_background_truecolor_text_stays_legible() {
        let mut t = HeadlessTerm::new(GridSize { cols: 20, rows: 4 });
        let (r, g, b) = crew_theme::theme().term_bg;
        t.feed(format!("\x1b[38;2;{r};{g};{b}mhi").as_bytes());
        let cells = t.cells(false);
        let h = cells.iter().find(|c| c.c == 'h').expect("cell rendered");
        assert!(
            crate::contrast::ratio(h.fg, h.bg) >= crate::contrast::MIN_CONTRAST - 0.1,
            "bg-on-bg text rendered at ratio {} (fg {:?} on bg {:?})",
            crate::contrast::ratio(h.fg, h.bg),
            h.fg,
            h.bg
        );
    }
}

#[cfg(test)]
mod selection_tests {
    use super::super::{GridSize, HeadlessTerm, TermModel};

    fn term(text: &str) -> HeadlessTerm {
        let mut t = HeadlessTerm::new(GridSize { cols: 20, rows: 4 });
        t.feed(text.as_bytes());
        t
    }

    #[test]
    fn no_selection_yields_no_text() {
        assert_eq!(term("hello").sel_text(), None);
    }

    #[test]
    fn inverse_video_is_not_drawn_as_a_highlight() {
        // 'X' is plain; 'H' is reverse-video (SGR 7). With the program's
        // highlight suppressed, the inverse cell must render with the SAME
        // colours as the plain one — no swapped fg/bg "highlight" block.
        let mut t = HeadlessTerm::new(GridSize { cols: 20, rows: 2 });
        t.feed(b"X\x1b[7mH\x1b[0m");
        let cells = t.cells(false);
        let x = cells.iter().find(|c| c.c == 'X').expect("X rendered");
        let h = cells.iter().find(|c| c.c == 'H').expect("H rendered");
        assert_eq!(h.fg, x.fg, "inverse cell should keep the normal foreground");
        assert_eq!(h.bg, x.bg, "inverse cell should keep the normal background");
    }

    #[test]
    fn dim_grey_echo_background_is_dropped() {
        // Agent CLIs paint the just-sent line with a dark-grey background
        // (ESC[48;2;55;55;55m). 'X' is plain; 'H' carries that grey bg — which
        // must be dropped so it renders on the same canvas as the plain cell.
        let mut t = HeadlessTerm::new(GridSize { cols: 20, rows: 2 });
        t.feed(b"X\x1b[48;2;55;55;55mH\x1b[0m");
        let cells = t.cells(false);
        let x = cells.iter().find(|c| c.c == 'X').expect("X rendered");
        let h = cells.iter().find(|c| c.c == 'H').expect("H rendered");
        assert_eq!(h.bg, x.bg, "dark-grey echo background should be dropped");
    }

    #[test]
    fn mid_grey_program_background_is_flattened_in_dark_theme() {
        // The regression `is_echo_grey` missed: a MID-grey highlight (neither
        // near-black nor near-white) still reads as an ugly box on the flat
        // dark canvas and must be dropped too. Only meaningful if the test
        // theme is dark (default is PaperDark — see crew-theme's
        // `default_is_paper_dark`); the pure `should_drop_bg` tests below are
        // the primary coverage regardless of theme.
        let mut t = HeadlessTerm::new(GridSize { cols: 20, rows: 2 });
        t.feed(b"X\x1b[48;2;140;140;140mH\x1b[0m");
        let cells = t.cells(false);
        let x = cells.iter().find(|c| c.c == 'X').expect("X rendered");
        let h = cells.iter().find(|c| c.c == 'H').expect("H rendered");
        if crew_theme::theme().term_bg == crew_theme::PAPER_DARK.term_bg {
            assert_eq!(
                h.bg, x.bg,
                "mid-grey program background should be flattened in a dark theme"
            );
        }
    }

    #[test]
    fn light_grey_echo_background_is_dropped() {
        // The same echo highlight painted for the OPPOSITE theme: a CLI that
        // detected a light background (or outlived a live switch to dark)
        // paints the just-sent line light-grey (ESC[48;2;230;230;230m). On the
        // dark canvas that reads as white word-boxes — drop it like the dark
        // variant.
        let mut t = HeadlessTerm::new(GridSize { cols: 20, rows: 2 });
        t.feed(b"X\x1b[48;2;230;230;230mH\x1b[0m");
        let cells = t.cells(false);
        let x = cells.iter().find(|c| c.c == 'X').expect("X rendered");
        let h = cells.iter().find(|c| c.c == 'H').expect("H rendered");
        assert_eq!(h.bg, x.bg, "light-grey echo background should be dropped");
    }

    #[test]
    fn saturated_dark_background_is_kept() {
        // A dark-but-coloured background (e.g. a diff's green) carries meaning and
        // must survive — only desaturated greys are treated as echo highlights.
        let mut t = HeadlessTerm::new(GridSize { cols: 20, rows: 2 });
        t.feed(b"\x1b[48;2;0;60;0mD\x1b[0m");
        let cells = t.cells(false);
        let d = cells.iter().find(|c| c.c == 'D').expect("D rendered");
        assert_eq!(d.bg, (0, 60, 0), "saturated dark background should be kept");
    }

    #[test]
    fn drag_selects_an_inclusive_character_span() {
        let mut t = term("hello world");
        // Drag from column 0 to column 4 on row 0 — the cell under the cursor is
        // included, so this is "hello", not "hell".
        t.sel_start(0, 0, false);
        t.sel_update(4, 0);
        assert_eq!(t.sel_text().as_deref(), Some("hello"));
    }

    /// The same span, dragged the other way, must copy the same text.
    ///
    /// `sel_start` hard-coded `Side::Left` and `sel_update` `Side::Right`,
    /// which is only right for a FORWARD drag. On a reverse drag alacritty's
    /// `to_range` swaps the anchors but keeps their sides, then trims the last
    /// cell when `end.side == Left` and the first when `start.side == Right` —
    /// so a right-to-left drag over "hello" copied "ell". The suite only ever
    /// dragged left-to-right, so it never saw it.
    #[test]
    fn a_backward_drag_selects_the_same_span_as_a_forward_one() {
        let forward = {
            let mut t = term("hello world");
            t.sel_start(0, 0, false);
            t.sel_update(4, 0);
            t.sel_text()
        };
        let mut t = term("hello world");
        t.sel_start(4, 0, false); // press on the 'o'
        t.sel_update(0, 0); // drag back to the 'h'
        assert_eq!(
            t.sel_text().as_deref(),
            Some("hello"),
            "a right-to-left drag lost characters"
        );
        assert_eq!(t.sel_text(), forward, "drag direction changed the text");
    }

    /// 256-colour output must render in colour.
    ///
    /// `resolve_color` sends every Indexed value >= 16 to the default fg when
    /// alacritty's palette has no entry — so the entire xterm cube (16-231)
    /// and greyscale ramp (232-255) render monochrome: bat, fzf, btop, vim
    /// colorschemes and p10k prompts all lose their colour. `query_color`
    /// implements the cube correctly and its doc claims to mirror
    /// `resolve_color`, which is how the two drifted unnoticed.
    #[test]
    fn indexed_256_colours_render_in_colour() {
        // SGR 38;5;196 = the cube's bright red.
        let mut t = term("");
        t.feed(b"\x1b[38;5;196mR\x1b[0m");
        let cells = t.cells(true);
        let r = cells.iter().find(|c| c.c == 'R').expect("R rendered");
        assert_ne!(
            r.fg,
            crate::color::default_fg(),
            "indexed colour 196 fell back to the default fg — 256-colour output is monochrome"
        );
        // The xterm cube's 196 is pure red.
        assert_eq!(r.fg, (255, 0, 0), "196 should be the cube's red");
    }

    #[test]
    fn clearing_drops_the_selection() {
        let mut t = term("hello");
        t.sel_start(0, 0, false);
        t.sel_update(4, 0);
        t.sel_clear();
        assert_eq!(t.sel_text(), None);
    }

    #[test]
    fn selected_cells_render_with_the_selection_background() {
        let mut t = term("hello");
        // Select "he" (columns 0..=1 on row 0).
        t.sel_start(0, 0, false);
        t.sel_update(1, 0);
        let cells = t.cells(false);
        let bg = |ch| cells.iter().find(|c| c.c == ch).map(|c| c.bg);
        assert_eq!(bg('h'), Some(super::super::modelcells::SELECTION_BG));
        assert_eq!(bg('e'), Some(super::super::modelcells::SELECTION_BG));
        // 'o' is outside the selection — it keeps the normal background.
        assert_ne!(bg('o'), Some(super::super::modelcells::SELECTION_BG));
    }

    #[test]
    fn block_selection_takes_a_column_range_across_rows() {
        let mut t = term("abcde\r\nABCDE");
        // Rectangular columns 1..=3 over rows 0..=1 → "bcd" and "BCD".
        t.sel_start(1, 0, true);
        t.sel_update(3, 1);
        let txt = t.sel_text().unwrap_or_default();
        assert!(txt.contains("bcd") && txt.contains("BCD"), "got {txt:?}");
    }
}

#[cfg(test)]
mod should_drop_bg_tests {
    use super::super::modelcells::should_drop_bg;

    #[test]
    fn dark_theme_drops_mid_grey() {
        // The regression the old `is_echo_grey` missed: a MID-grey highlight
        // (neither near-black nor near-white) reads just as ugly on a flat
        // dark canvas as the extremes did.
        assert!(should_drop_bg((140, 140, 140), true));
    }

    #[test]
    fn dark_theme_drops_light_grey() {
        assert!(should_drop_bg((230, 230, 230), true));
    }

    #[test]
    fn dark_theme_keeps_saturated_diff_green() {
        assert!(!should_drop_bg((30, 110, 50), true));
    }

    #[test]
    fn dark_theme_keeps_saturated_diff_red() {
        assert!(!should_drop_bg((110, 40, 45), true));
    }

    #[test]
    fn light_theme_keeps_mid_grey() {
        // Light-theme behaviour is unchanged: `is_echo_grey`'s extremes-only
        // check does not treat mid-grey as an echo highlight.
        assert!(!should_drop_bg((140, 140, 140), false));
    }
}
