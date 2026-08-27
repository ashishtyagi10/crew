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

    /// DECSET 2031 rides `feed` like every other sniffer, its query replies
    /// land in the same `take_replies` channel, and the enabled flag is what
    /// the app's scheme push keys off. The reply matches the ACTIVE theme's
    /// darkness — the same source OSC 10/11 answers from.
    #[test]
    fn decset_2031_enables_and_scheme_query_answers_from_theme() {
        let mut t = HeadlessTerm::new(GridSize { cols: 20, rows: 4 });
        assert!(!t.scheme_notify_enabled());
        t.feed(b"\x1b[?2031h\x1b[?996n");
        assert!(t.scheme_notify_enabled());
        let ps = if crew_theme::theme().dark { 1 } else { 2 };
        assert_eq!(
            t.take_replies().as_deref(),
            Some(format!("\x1b[?997;{ps}n").as_str())
        );
        t.feed(b"\x1b[?2031l");
        assert!(!t.scheme_notify_enabled());
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

    /// Double-click. A word is whatever alacritty's own separators say it is,
    /// so this pins the behaviour rather than a hand-rolled definition.
    #[test]
    fn a_word_click_selects_the_word_under_the_cell() {
        for col in 6..=10 {
            let mut t = term("hello world");
            t.sel_word(col, 0);
            assert_eq!(
                t.sel_text().as_deref(),
                Some("world"),
                "clicking column {col} of 'world'"
            );
        }
        let mut t = term("hello world");
        t.sel_word(0, 0);
        assert_eq!(t.sel_text().as_deref(), Some("hello"), "the first word");
    }

    /// A path is one word: `/usr/local/bin` must not come back as `usr`, which
    /// is the whole point of double-clicking one.
    #[test]
    fn a_path_counts_as_one_word() {
        let mut t = term("cd /usr/local/bin");
        t.sel_word(8, 0);
        assert_eq!(t.sel_text().as_deref(), Some("/usr/local/bin"));
    }

    /// Triple-click takes the line, and never stops at a word boundary. The
    /// trailing newline is deliberate and is what every terminal copies for a
    /// line selection: pasting a triple-clicked command should *run* it.
    #[test]
    fn a_line_click_takes_the_whole_line() {
        let mut t = term("hello world");
        t.sel_line(3, 0);
        assert_eq!(t.sel_text().as_deref(), Some("hello world\n"));
    }

    /// The three gestures widen in order: a word is more than a cell, and a
    /// line is at least a word. A regression that made `sel_line` behave like
    /// `sel_word` would pass every assertion above about "world".
    #[test]
    fn the_gestures_widen_and_do_not_collapse_into_each_other() {
        let text = |f: fn(&mut super::super::HeadlessTerm)| {
            let mut t = term("hello world");
            f(&mut t);
            t.sel_text().unwrap_or_default()
        };
        let cell = text(|t| {
            t.sel_start(6, 0, false);
            t.sel_update(6, 0);
        });
        let word = text(|t| t.sel_word(6, 0));
        let line = text(|t| t.sel_line(6, 0));
        assert!(
            word.len() > cell.len(),
            "{word:?} should widen past {cell:?}"
        );
        assert!(
            line.len() > word.len(),
            "{line:?} should widen past {word:?}"
        );
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
        assert_eq!(bg('h'), Some(super::super::modelcells::selection_bg()));
        assert_eq!(bg('e'), Some(super::super::modelcells::selection_bg()));
        // 'o' is outside the selection — it keeps the normal background.
        assert_ne!(bg('o'), Some(super::super::modelcells::selection_bg()));
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

/// The decorations a program asks for with SGR have to survive the whole trip:
/// parser → grid → `RenderCell`. Before this landed the grid knew about them
/// and the render cell had nowhere to put them, so every one was dropped
/// silently — a diagnostic squiggle rendered as plain text.
#[cfg(test)]
mod deco_tests {
    use super::super::{GridSize, HeadlessTerm, TermModel};
    use crew_theme::deco::DecoLine;

    fn line_under(seq: &str) -> DecoLine {
        let mut t = HeadlessTerm::new(GridSize { cols: 20, rows: 4 });
        t.feed(format!("{seq}x").as_bytes());
        let cells = t.cells(true);
        let c = cells
            .iter()
            .find(|c| c.c == 'x')
            .expect("the decorated glyph is rendered");
        c.deco.line
    }

    #[test]
    fn each_sgr_underline_reaches_the_render_cell() {
        assert_eq!(line_under("\x1b[4m"), DecoLine::Single);
        assert_eq!(line_under("\x1b[4:2m"), DecoLine::Double);
        assert_eq!(line_under("\x1b[4:3m"), DecoLine::Curly);
        assert_eq!(line_under("\x1b[4:4m"), DecoLine::Dotted);
        assert_eq!(line_under("\x1b[4:5m"), DecoLine::Dashed);
        assert_eq!(line_under(""), DecoLine::None);
    }

    #[test]
    fn sgr_9_strikes_the_cell_through_without_underlining_it() {
        let mut t = HeadlessTerm::new(GridSize { cols: 20, rows: 4 });
        t.feed(b"\x1b[9mx");
        let c = t.cells(true).into_iter().find(|c| c.c == 'x').unwrap();
        assert!(c.deco.strike);
        assert_eq!(c.deco.line, DecoLine::None);
    }

    /// `foo bar` underlined is one rule, not two: the space carries it.
    #[test]
    fn a_space_inside_an_underlined_run_is_kept_and_an_undecorated_one_is_not() {
        let mut t = HeadlessTerm::new(GridSize { cols: 20, rows: 4 });
        t.feed(b"\x1b[4mfoo bar\x1b[0m qux");
        let cells = t.cells(true);
        // The block cursor is painted after the filter, so it is a space cell
        // too — read the run itself, not every space on the row.
        let space = |col: u16| cells.iter().any(|c| c.col == col && c.c == ' ');
        assert!(
            space(3),
            "the underlined space is dropped, breaking the rule"
        );
        assert!(!space(7), "an undecorated space still costs nothing");
        assert_eq!(
            cells.iter().find(|c| c.col == 3).unwrap().deco.line,
            DecoLine::Single
        );
    }

    /// SGR 58 colours the rule independently of the text. Underline red under
    /// white text is how a language server marks an error inline.
    #[test]
    fn sgr_58_colours_the_rule_and_leaves_the_text_alone() {
        let mut t = HeadlessTerm::new(GridSize { cols: 20, rows: 4 });
        t.feed(b"\x1b[4:3m\x1b[58:2::255:0:0mx");
        let c = t.cells(true).into_iter().find(|c| c.c == 'x').unwrap();
        assert_eq!(c.deco.color, Some((255, 0, 0)));
        assert_ne!(c.fg, (255, 0, 0), "the glyph keeps its own colour");
    }

    /// SGR 24 / 29 put a cell back to plain — a run that never ends is a rule
    /// under the rest of the screen.
    #[test]
    fn the_reset_sequences_clear_the_rules_again() {
        let mut t = HeadlessTerm::new(GridSize { cols: 20, rows: 4 });
        t.feed(b"\x1b[4:3m\x1b[9ma\x1b[24mb\x1b[29mc");
        let cells = t.cells(true);
        let at = |ch: char| cells.iter().find(|c| c.c == ch).unwrap().deco;
        assert_eq!(at('a').line, DecoLine::Curly);
        assert!(at('a').strike);
        assert_eq!(at('b').line, DecoLine::None, "SGR 24 lifts the underline");
        assert!(at('b').strike, "and leaves the strike alone");
        assert!(!at('c').strike, "SGR 29 lifts the strike");
        assert!(at('c').is_blank());
    }
}

/// OSC 8 hyperlinks: the text a program shows and the target it points at are
/// different things, and the grid is the only place that knows the second one.
#[cfg(test)]
mod link_tests {
    use super::super::{GridSize, HeadlessTerm, TermModel};
    use crew_theme::deco::DecoLine;

    const URI: &str = "https://example.com/notes";

    fn linked() -> HeadlessTerm {
        let mut t = HeadlessTerm::new(GridSize { cols: 40, rows: 4 });
        t.feed(format!("\x1b]8;;{URI}\x1b\\see notes\x1b]8;;\x1b\\ plain").as_bytes());
        t
    }

    #[test]
    fn the_target_is_readable_under_every_cell_of_the_link_and_nowhere_else() {
        let t = linked();
        for col in 0..9 {
            assert_eq!(t.link_at(col, 0).as_deref(), Some(URI), "col {col}");
        }
        assert_eq!(t.link_at(9, 0), None, "the space after the link is not it");
        assert_eq!(t.link_at(12, 0), None, "and neither is the prose");
        assert_eq!(t.link_at(0, 3), None, "nor an empty row");
    }

    /// A click lands on a cell, so an out-of-range cell must answer rather
    /// than index the grid and panic.
    #[test]
    fn a_cell_outside_the_grid_has_no_link() {
        let t = linked();
        assert_eq!(t.link_at(500, 0), None);
        assert_eq!(t.link_at(0, 90), None);
    }

    /// The words are prose — "see notes" — so nothing about the text says it
    /// is a link. Being drawn as one is the only cue there is.
    #[test]
    fn link_text_is_tinted_and_ruled_even_though_it_is_not_a_url() {
        let t = linked();
        let cells = t.cells(false);
        let at = |col: u16| cells.iter().find(|c| c.col == col && c.row == 0).unwrap();
        let link_fg = at(0).fg;
        assert_eq!(at(0).deco.line, DecoLine::Single, "the link is not ruled");
        assert_eq!(at(8).deco.line, DecoLine::Single);
        let plain = at(12);
        assert_eq!(plain.deco.line, DecoLine::None, "prose got ruled too");
        assert_ne!(plain.fg, link_fg, "the link is not tinted apart");
    }

    /// A hyperlink spanning two words carries the space between them; without
    /// it the rule under the link breaks in half.
    #[test]
    fn a_space_inside_a_hyperlink_is_kept() {
        let t = linked();
        let cells = t.cells(false);
        assert!(cells.iter().any(|c| c.col == 3 && c.c == ' '));
        assert_eq!(
            cells.iter().find(|c| c.col == 3).unwrap().deco.line,
            DecoLine::Single
        );
    }
}

/// DECSCUSR through the parser and out the other side: what the program asked
/// for has to survive into the cell, or an editor's insert-mode bar renders as
/// the same block its normal mode does.
#[cfg(test)]
mod cursor_shape_tests {
    use super::super::{GridSize, HeadlessTerm, TermModel};
    use crew_theme::deco::CursorShape;

    fn shape_after(seq: &str, focused: bool) -> CursorShape {
        let mut t = HeadlessTerm::new(GridSize { cols: 10, rows: 3 });
        t.feed(format!("{seq}ab").as_bytes());
        t.cells(focused)
            .into_iter()
            .find(|c| c.col == 2 && c.row == 0)
            .expect("a cell where the cursor is")
            .cursor
            .shape
    }

    #[test]
    fn decscusr_picks_the_shape_the_cell_carries() {
        assert_eq!(shape_after("\x1b[2 q", true), CursorShape::Block);
        assert_eq!(shape_after("\x1b[4 q", true), CursorShape::Underline);
        assert_eq!(shape_after("\x1b[6 q", true), CursorShape::Beam);
        assert_eq!(
            shape_after("", true),
            CursorShape::Block,
            "block by default"
        );
    }

    #[test]
    fn the_same_program_in_an_unfocused_pane_shows_an_outline() {
        assert_eq!(shape_after("\x1b[6 q", false), CursorShape::Hollow);
        assert_eq!(shape_after("\x1b[2 q", false), CursorShape::Hollow);
    }

    /// The block still inverts the cell it lands on; a bar must not, or the
    /// character the bar sits beside changes colour when the cursor arrives.
    #[test]
    fn only_the_block_repaints_the_cell_under_it() {
        let mut t = HeadlessTerm::new(GridSize { cols: 10, rows: 3 });
        t.feed(b"ab\x1b[1;1H");
        let block = t.cells(true).into_iter().find(|c| c.col == 0).unwrap();
        assert_eq!(
            block.bg,
            crew_theme::readable::cursor(crew_theme::theme(), true)
        );
        let mut t = HeadlessTerm::new(GridSize { cols: 10, rows: 3 });
        t.feed(b"\x1b[6 qab\x1b[1;1H");
        let beam = t.cells(true).into_iter().find(|c| c.col == 0).unwrap();
        assert_eq!(beam.c, 'a');
        assert_eq!(beam.cursor.shape, CursorShape::Beam);
        assert_ne!(
            beam.bg,
            crew_theme::readable::cursor(crew_theme::theme(), true),
            "the bar painted the cell as well as itself"
        );
    }
}

/// What a TUI draws with is a coloured space. Every one of them used to be
/// dropped before its background was even resolved, so a status line, a
/// progress bar, a selected row and a diff block were all invisible.
#[cfg(test)]
mod painted_background_tests {
    use super::super::{GridSize, HeadlessTerm, TermModel};

    fn feed(seq: &[u8]) -> Vec<crate::model::RenderCell> {
        let mut t = HeadlessTerm::new(GridSize { cols: 20, rows: 3 });
        t.feed(seq);
        t.cells(false)
    }

    #[test]
    fn a_run_of_coloured_spaces_is_drawn() {
        let cells = feed(b"\x1b[41m   \x1b[0m");
        let painted: Vec<&crate::model::RenderCell> =
            cells.iter().filter(|c| c.col < 3 && c.row == 0).collect();
        assert_eq!(painted.len(), 3, "the red block vanished");
        assert!(painted.iter().all(|c| c.c == ' '));
        assert!(
            painted.iter().all(|c| c.bg != crate::color::default_bg()),
            "kept, but with the colour thrown away"
        );
    }

    /// The flat-canvas rule still holds: the near-grey an agent CLI paints
    /// behind the line you just sent is flattened, so it does not come back as
    /// a run of grey boxes now that blanks survive.
    #[test]
    fn the_echo_grey_a_cli_paints_is_still_dropped() {
        let cells = feed(b"\x1b[48;2;55;55;55m   \x1b[0m");
        assert!(
            !cells.iter().any(|c| c.col < 3 && c.row == 0 && c.c == ' '),
            "the flattened grey was kept as an empty cell"
        );
    }

    /// Dragging across empty space used to highlight nothing at all: the
    /// selection sets a background, and a blank cell had already been thrown
    /// away by the time it did.
    #[test]
    fn a_selection_over_blank_space_is_visible() {
        let mut t = HeadlessTerm::new(GridSize { cols: 20, rows: 3 });
        t.feed(b"ab");
        t.sel_start(0, 0, false);
        t.sel_update(6, 0);
        let cells = t.cells(false);
        let blank = cells
            .iter()
            .find(|c| c.col == 4 && c.row == 0)
            .expect("the blank inside the selection is drawn");
        assert_eq!(blank.c, ' ');
        assert_ne!(blank.bg, crate::color::default_bg());
    }

    /// And an ordinary empty screen still costs nothing: a terminal is mostly
    /// blank, and every kept blank is a cell shaped and a quad drawn.
    #[test]
    fn plain_spaces_are_still_dropped() {
        let cells = feed(b"a   b");
        let spaces = cells.iter().filter(|c| c.c == ' ' && c.col < 5).count();
        assert_eq!(spaces, 0, "{} plain spaces were kept", spaces);
    }
}

/// SGR 2 and SGR 8: the two attributes that change whether a cell is read at
/// all. Half of what an agent CLI prints is dim, and a password prompt that
/// conceals its field means it.
#[cfg(test)]
mod quiet_tests {
    use super::super::{GridSize, HeadlessTerm, TermModel};
    use crate::contrast::{ratio, DIM_CONTRAST};

    fn cell(seq: &[u8], col: u16) -> crate::model::RenderCell {
        let mut t = HeadlessTerm::new(GridSize { cols: 30, rows: 3 });
        t.feed(seq);
        t.cells(false)
            .into_iter()
            .find(|c| c.col == col && c.row == 0)
            .expect("a cell at that column")
    }

    #[test]
    fn dim_text_is_quieter_than_the_same_text_undimmed() {
        let plain = cell(b"x", 0);
        let dim = cell(b"\x1b[2mx", 0);
        assert_eq!(dim.c, 'x');
        assert!(
            ratio(dim.fg, dim.bg) < ratio(plain.fg, plain.bg),
            "dim {:?} reads as loud as plain {:?}",
            dim.fg,
            plain.fg
        );
    }

    /// …but never so quiet it is gone. A dim that cannot be read is a line
    /// dropped, not a line whispered.
    #[test]
    fn dim_text_is_still_readable_even_when_the_program_picked_badly() {
        // A foreground the program tuned for the other kind of page.
        let dim = cell(b"\x1b[2m\x1b[38;2;20;20;24mx", 0);
        assert!(
            ratio(dim.fg, dim.bg) >= DIM_CONTRAST - 0.01,
            "{:?} on {:?} reads at {}",
            dim.fg,
            dim.bg,
            ratio(dim.fg, dim.bg)
        );
    }

    /// Dim keeps the colour it dims: a dim red is still red, or a CLI's
    /// colour-coded secondary lines all turn the same grey.
    #[test]
    fn dim_preserves_the_hue_it_was_given() {
        let dim = cell(b"\x1b[2m\x1b[38;2;220;40;40mx", 0);
        assert!(
            dim.fg.0 > dim.fg.1 && dim.fg.0 > dim.fg.2,
            "dim red came out {:?}",
            dim.fg
        );
    }

    /// SGR 22 puts the voice back — a dim run that never ends is the rest of
    /// the session in half-ink.
    #[test]
    fn sgr_22_ends_the_dim_run() {
        let mut t = HeadlessTerm::new(GridSize { cols: 30, rows: 3 });
        t.feed(b"\x1b[2ma\x1b[22mb");
        let cells = t.cells(false);
        let at = |col: u16| cells.iter().find(|c| c.col == col).unwrap().fg;
        assert_ne!(at(0), at(1), "SGR 22 left the text dim");
        assert_eq!(at(1), cell(b"b", 0).fg);
    }

    /// SGR 8 conceals. The characters are still in the grid — a selection
    /// copies what is there — but nothing is drawn.
    #[test]
    fn concealed_text_draws_nothing_and_sgr_28_reveals_again() {
        let mut t = HeadlessTerm::new(GridSize { cols: 30, rows: 3 });
        t.feed(b"\x1b[8mhide\x1b[28mshow");
        let cells = t.cells(false);
        let drawn: String = (0..4)
            .filter_map(|c| cells.iter().find(|x| x.col == c).map(|x| x.c))
            .collect();
        assert!(drawn.trim().is_empty(), "the concealed run drew {drawn:?}");
        assert!(cells.iter().any(|c| c.col == 4 && c.c == 's'));
    }
}
