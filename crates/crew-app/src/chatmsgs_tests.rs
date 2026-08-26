use super::*;
use crate::chat::ChatPane;
use crew_plugin::Plugin;

fn msg(sender: &str, text: &str) -> Message {
    Message {
        sender: sender.into(),
        text: text.into(),
        ts: String::new(),
        meta: String::new(),
        usage: None,
        expanded: false,
    }
}

fn row_text(cells: &[CellView], row: u16) -> String {
    let mut v: Vec<(u16, char)> = cells
        .iter()
        .filter(|c| c.row == row)
        .map(|c| (c.col, c.c))
        .collect();
    v.sort_unstable();
    v.into_iter().map(|(_, c)| c).collect()
}

#[test]
fn card_has_header_then_indented_body() {
    let cells = message_cells(&[&msg("planner", "hello")], 40, 10, 0, 0, View::default());
    assert_eq!(row_text(&cells, 0), format!("{GUTTER}planner"));
    assert_eq!(row_text(&cells, 1), " hello");
}

#[test]
fn cards_are_separated_by_a_blank_line() {
    let m = [msg("planner", "a"), msg("coder", "b")];
    let refs: Vec<&Message> = m.iter().collect();
    let cells = message_cells(&refs, 40, 10, 0, 0, View::default());
    assert_eq!(row_text(&cells, 2), ""); // spacer
    assert_eq!(row_text(&cells, 3), format!("{GUTTER}coder"));
}

#[test]
fn multiline_reply_renders_each_line() {
    let cells = message_cells(&[&msg("coder", "one\ntwo")], 40, 10, 0, 0, View::default());
    assert_eq!(row_text(&cells, 1), " one");
    assert_eq!(row_text(&cells, 2), " two");
}

#[test]
fn fenced_code_renders_as_bordered_card() {
    let cells = message_cells(
        &[&msg("coder", "fix:\n```rust\nlet x = 1;\n```")],
        40,
        10,
        0,
        0,
        View::default(),
    );
    assert_eq!(row_text(&cells, 1), " fix:");
    assert_eq!(row_text(&cells, 2), " \u{256d}\u{2500} rust");
    assert_eq!(row_text(&cells, 3), " let x = 1;");
    assert_eq!(row_text(&cells, 4), " \u{2570}\u{2500}");
    // The code row sits on a bg different from the page background.
    let page = crew_theme::theme().page_bg;
    assert!(
        cells
            .iter()
            .any(|c| c.row == 3 && c.col > 0 && c.bg != page),
        "code should be on a dimmed card background"
    );
}

#[test]
fn header_tail_keeps_relative_time_but_drops_latency() {
    // The muted card-header tail carries "when" (relative time), never the
    // per-card reply latency — reductionist chrome, one signal per question.
    let m = Message {
        sender: "coder".into(),
        text: "done".into(),
        ts: "999700000".into(),
        meta: "4.2s".into(),
        usage: None,
        expanded: false,
    };
    let chars: String = header_line(&m, 1_000_000_000, None)
        .iter()
        .map(|c| c.c)
        .collect();
    assert!(chars.contains("5m ago"), "relative time shown: {chars}");
    assert!(!chars.contains("4.2s"), "latency must be gone: {chars}");
}

#[test]
fn handoff_sender_colours_each_name_separately() {
    let cells = message_cells(
        &[&msg("planner \u{2192} coder", "x")],
        40,
        10,
        0,
        0,
        View::default(),
    );
    assert_eq!(
        row_text(&cells, 0),
        format!("{GUTTER}planner \u{2192} coder")
    );
    let muted = crew_theme::theme().text_muted;
    let cell_at = |col: u16| cells.iter().find(|c| c.row == 0 && c.col == col).unwrap();
    assert_ne!(cell_at(1).fg, muted, "planner keeps its agent colour");
    assert_ne!(cell_at(11).fg, muted, "coder keeps its agent colour");
}

#[test]
fn system_sender_is_muted_and_agents_are_not() {
    assert_eq!(sender_color("crew"), crew_theme::theme().text_muted);
    assert_ne!(sender_color("planner"), crew_theme::theme().text_muted);
}

#[test]
fn crew_message_uses_the_dotted_system_gutter() {
    let cells = message_cells(&[&msg("crew", "hello")], 40, 10, 0, 0, View::default());
    assert_eq!(row_text(&cells, 0), "\u{2506}crew");
}

#[test]
fn agent_message_keeps_the_solid_gutter() {
    let cells = message_cells(
        &[&msg("planner \u{2192} user", "hello")],
        40,
        10,
        0,
        0,
        View::default(),
    );
    assert_eq!(
        row_text(&cells, 0),
        format!("{GUTTER}planner \u{2192} user")
    );
}

#[test]
fn count_matches_rendered_lines_and_scroll_shows_older() {
    let m = [msg("a", "one"), msg("b", "two")];
    let refs: Vec<&Message> = m.iter().collect();
    // 2 cards × (header + body) + 1 spacer = 5 lines.
    assert_eq!(card_line_count(&refs, 40, View::default()), 5);
    // A 2-row window scrolled 3 up from the bottom shows the first card.
    let cells = message_cells(&refs, 40, 2, 0, 3, View::default());
    assert_eq!(row_text(&cells, 0), format!("{GUTTER}a"));
}

#[test]
fn top_row_offsets_and_width_clips() {
    let cells = message_cells(
        &[&msg("planner", "wide text here")],
        5,
        4,
        3,
        0,
        View::default(),
    );
    assert!(cells.iter().all(|c| c.row >= 3 && c.col < 5));
}

#[test]
fn wide_glyphs_advance_two_columns() {
    // "中x": the wide glyph sits at its column and `x` lands TWO columns
    // later, so it can't overlap the glyph's second cell.
    let cells = message_cells(&[&msg("a", "\u{4e2d}x")], 20, 4, 0, 0, View::default());
    let body: Vec<(u16, char)> = cells
        .iter()
        .filter(|c| c.row == 1 && c.c != ' ')
        .map(|c| (c.col, c.c))
        .collect();
    let wide = body
        .iter()
        .find(|(_, c)| *c == '\u{4e2d}')
        .expect("wide glyph present");
    let x = body.iter().find(|(_, c)| *c == 'x').expect("x present");
    assert_eq!(x.0, wide.0 + 2, "got: {body:?}");
}

#[test]
fn header_line_shows_a_dim_chip_for_task_tagged_messages() {
    let m = Message {
        sender: "planner \u{2192} user".into(),
        text: "done".into(),
        ts: String::new(),
        meta: "task:2 \u{00b7} 0.0s".into(),
        usage: None,
        expanded: false,
    };
    let line = header_line(&m, 0, None);
    let muted = crew_theme::theme().text_muted;
    let hash = line.iter().find(|c| c.c == '#').expect("chip # present");
    assert_eq!(hash.fg, muted, "chip # is muted");
    let id = line.iter().find(|c| c.c == '2').expect("chip id present");
    assert_eq!(id.fg, muted, "chip id is muted");
    let chars: String = line.iter().map(|c| c.c).collect();
    assert!(
        !chars.contains("0.0s"),
        "per-card latency must be gone: {chars}"
    );
    assert!(
        !chars.contains("task"),
        "tag must not leak into the header: {chars}"
    );
}

#[test]
fn header_line_has_no_chip_for_untagged_messages() {
    let mut m = msg("coder", "done");
    m.meta = "4.2s".into();
    let line = header_line(&m, 0, None);
    assert!(
        !line.iter().any(|c| c.c == '#'),
        "no task tag means no chip"
    );
}

#[test]
fn splash_renders_headerless_and_centered() {
    // The startup nameplate: no `agent smith · time` header line above it,
    // and every line centered in the pane width.
    let art = "\u{2554}\u{2550}\u{2550}\u{2557}\n\u{2551} AGENT \u{2551}\n\u{255a}\u{2550}\u{2550}\u{255d}";
    let m = msg("agent smith", art);
    assert!(
        is_splash(&m),
        "nameplate art must be detected as the splash"
    );
    let lines = card_lines(&[&m], 40, 0, View::default());
    let texts: Vec<String> = lines
        .iter()
        .map(|l| l.iter().map(|c| c.c).collect())
        .collect();
    assert!(
        !texts.iter().any(|t| t.contains("agent smith")),
        "no header line on the splash: {texts:?}"
    );
    let top = texts
        .iter()
        .find(|t| t.contains('\u{2554}'))
        .expect("box top present");
    let lead = top.chars().take_while(|c| *c == ' ').count();
    assert!(lead > 10, "box must be centered, got lead {lead}: {top:?}");
}

#[test]
fn splash_art_is_centered_verbatim_no_injected_glyphs() {
    // The nameplate interior renders exactly as the broker sent it — no
    // decorations inside the box (review feedback: the side glyphs read as
    // typos next to the name) — just centered.
    let art = "\u{2551}      AGENT      \u{2551}";
    let mut body: Vec<CardLine> = crate::chatbody::body_lines(art, 40, (9, 9, 9), true);
    splash_style(&mut body, 40);
    let text: String = body[0].iter().map(|c| c.c).collect();
    assert_eq!(text.trim_start(), art, "art must be untouched: {text:?}");
    assert!(text.starts_with(' '), "and centered");
}

#[test]
fn same_task_cards_chain_with_a_tree_connector_and_no_spacer() {
    let mk = |sender: &str, meta: &str| Message {
        usage: None,
        expanded: false,
        sender: sender.into(),
        text: "x".into(),
        ts: String::new(),
        meta: meta.into(),
    };
    let m = [
        mk("planner \u{2192} user", "task:2 \u{00b7} 0.0s"),
        mk("coder \u{2192} user", "task:2 \u{00b7} 1.0s"),
        mk("planner \u{2192} user", "task:3"),
    ];
    let refs: Vec<&Message> = m.iter().collect();
    let cells = message_cells(&refs, 60, 12, 0, 0, View::default());
    // Card 1: header + body. Card 2 chains directly underneath (no spacer),
    // its header led by the muted └ connector and without a repeated #2.
    let follow = row_text(&cells, 2);
    assert!(
        follow.starts_with("\u{2514} coder"),
        "chained header connects with \u{2514}: {follow:?}"
    );
    assert!(!follow.contains("#2"), "no repeated task chip: {follow:?}");
    // Card 3 is a different task: spacer, then a fresh gutter header with #3.
    assert_eq!(row_text(&cells, 4), "", "unrelated cards keep the spacer");
    let fresh = row_text(&cells, 5);
    assert!(fresh.contains("#3"), "chain root keeps its chip: {fresh:?}");
}

#[test]
fn middle_chained_cards_get_tee_last_gets_corner() {
    // Three replies on task #7: root keeps its gutter, the middle chained
    // card connects with ├, the final one closes with └.
    let mk = |text: &str| Message {
        sender: "coder".into(),
        text: text.into(),
        ts: String::new(),
        meta: "task:7".into(),
        usage: None,
        expanded: false,
    };
    let msgs = [mk("root"), mk("mid"), mk("last")];
    let refs: Vec<&Message> = msgs.iter().collect();
    let lines = card_lines(&refs, 80, 0, View::default());
    let texts: Vec<String> = lines
        .iter()
        .map(|l| l.iter().map(|c| c.c).collect())
        .collect();
    let headers: Vec<&String> = texts.iter().filter(|t| t.contains("coder")).collect();
    assert!(headers[1].starts_with("\u{251c} "), "{:?}", headers[1]);
    assert!(headers[2].starts_with("\u{2514} "), "{:?}", headers[2]);
}

#[test]
fn fade_t_ramps_with_message_age() {
    // Counting pass (now == 0) and unstamped messages render fully drawn.
    assert_eq!(fade_t("1000", 0), 1.0);
    assert_eq!(fade_t("", 5_000), 1.0);
    // A just-landed message starts faded and finishes after FADE_MS.
    assert_eq!(fade_t("5000", 5_000), 0.0);
    let mid = fade_t("5000", 5_000 + FADE_MS / 2);
    assert!(mid > 0.4 && mid < 0.6, "got: {mid}");
    assert_eq!(fade_t("5000", 5_000 + FADE_MS), 1.0);
}

fn test_pane(messages: Vec<Message>) -> ChatPane {
    let plugin = Plugin::spawn("sh", &["-c".to_string(), "cat >/dev/null".to_string()]).unwrap();
    let mut pane = ChatPane::new(plugin, "crew".into());
    pane.messages = messages;
    pane
}

#[test]
fn italic_cardcell_threads_through_to_cellview() {
    // `line_cells` is the per-row mapper `message_cells` maps over; a
    // hand-built italic cell pins that the flag survives to `CellView`
    // even before Task 4 wires a producer for it (markdown emphasis).
    let page = crew_theme::theme().page_bg;
    let line: CardLine = vec![CardCell {
        c: 'x',
        fg: (1, 2, 3),
        bold: false,
        italic: true,
        bg: None,
        link: None,
    }];
    let cells = line_cells(0, &line, 40, page);
    assert_eq!(cells.len(), 1);
    assert!(cells[0].italic, "italic must survive to the CellView");
}

#[test]
fn message_cells_is_a_thin_map_over_placed_lines_in_both_modes() {
    use std::collections::HashSet;
    let mut pane = test_pane(vec![
        msg("planner", "one"),
        msg("coder", "two"),
        msg("crew", "three"),
    ]);
    let (cols, rows) = (40u16, 30u16);

    // All four (source, compact) combinations — orthogonal flags, both can
    // be on at once.
    for (show_source, compact) in [(false, false), (true, false), (false, true), (true, true)] {
        pane.show_source = show_source;
        pane.compact_view = compact;
        let view = View {
            gap_rows: crate::density::Density::Cozy.card_gap_rows(),
            source: show_source,
            compact,
            streaming_from: pane.messages.len(),
        };
        let top = pane.status_rows(cols, rows);
        let bottom = crate::chatinput::composer_rows(&pane.input, cols, rows);
        let msg_rows = rows.saturating_sub(top + bottom);
        let refs: Vec<&Message> = pane.messages.iter().collect();
        let cells = message_cells(&refs, cols, msg_rows, top, pane.scroll, view);
        let placed = placed_lines(&pane, cols, rows);

        // Coverage independently derived from `placed_lines`, using the same
        // width/clip rules `message_cells` applies per row.
        let mut expected: HashSet<(u16, u16)> = HashSet::new();
        for (row, line) in &placed {
            let mut col = 0u16;
            for cell in line {
                let w = crate::chatwidth::char_w(cell.c) as u16;
                if w == 0 {
                    continue;
                }
                if col + w > cols {
                    break;
                }
                expected.insert((*row, col));
                col += w;
            }
        }
        let actual: HashSet<(u16, u16)> = cells.iter().map(|c| (c.row, c.col)).collect();
        assert_eq!(
            actual, expected,
            "cells/placed-lines mismatch with show_source={show_source} compact={compact}"
        );
        assert!(
            !actual.is_empty(),
            "sanity: the pane should render some cells (show_source={show_source} compact={compact})"
        );
    }
}

#[test]
fn msg_rows_budget_matches_view_math() {
    // Enough messages to overflow a modest pane, so `placed_lines` actually
    // gets clipped by the budget rather than trivially fitting everything.
    let messages: Vec<Message> = (0..30)
        .map(|i| msg("planner", &format!("line {i}")))
        .collect();
    let pane = test_pane(messages);
    let (cols, rows) = (40u16, 20u16);

    let budget = crate::chatplace::msg_rows_budget(&pane, cols, rows);
    let top = pane.status_rows(cols, rows);
    let placed = placed_lines(&pane, cols, rows);

    assert!(
        !placed.is_empty(),
        "sanity: overflowing pane still places lines"
    );
    assert!(
        placed.len() as u16 <= budget,
        "placed_lines returned more rows ({}) than the shared budget ({budget})",
        placed.len()
    );
    for (row, _) in &placed {
        assert!(
            *row >= top && *row < top + budget,
            "row {row} outside the message-area budget [{top}, {})",
            top + budget
        );
    }
}

#[test]
fn msg_rows_budget_shrinks_by_one_when_a_message_is_queued() {
    // The queued-messages indicator claims a row above the composer exactly
    // like the live swarm block does — `msg_rows_budget` must reserve it.
    let messages: Vec<Message> = (0..30)
        .map(|i| msg("planner", &format!("line {i}")))
        .collect();
    let mut pane = test_pane(messages);
    let (cols, rows) = (40u16, 20u16);

    let budget_before = crate::chatplace::msg_rows_budget(&pane, cols, rows);
    pane.queued.push_back("queued while busy".into());
    let budget_after = crate::chatplace::msg_rows_budget(&pane, cols, rows);

    assert_eq!(
        budget_after,
        budget_before - 1,
        "the indicator's row comes out of the message budget"
    );

    pane.queued.push_back("another one".into());
    assert_eq!(
        crate::chatplace::msg_rows_budget(&pane, cols, rows),
        budget_after,
        "queue depth beyond 1 doesn't claim more rows"
    );
}

// -- Compact transcript view (Ctrl+O) ---------------------------------------

#[test]
fn compact_view_clamps_multiline_body_and_appends_hidden_suffix() {
    let m = [msg("coder", "one\ntwo\nthree")];
    let refs: Vec<&Message> = m.iter().collect();
    let full = card_lines(
        &refs,
        40,
        0,
        View {
            gap_rows: crate::density::Density::Cozy.card_gap_rows(),
            source: false,
            compact: false,
            streaming_from: usize::MAX,
        },
    );
    assert_eq!(full.len(), 4, "header + 3 body lines, no spacer (one msg)");

    let compact = card_lines(
        &refs,
        40,
        0,
        View {
            gap_rows: crate::density::Density::Cozy.card_gap_rows(),
            source: false,
            compact: true,
            streaming_from: usize::MAX,
        },
    );
    assert_eq!(
        compact.len(),
        2,
        "header + first body line only in compact mode"
    );
    let body_text: String = compact[1].iter().map(|c| c.c).collect();
    assert_eq!(
        body_text, " one \u{2026} +2",
        "first line keeps its text plus a muted ` … +N` suffix for the 2 hidden lines"
    );
    let muted = crew_theme::theme().text_muted;
    let suffix_start = body_text.find('\u{2026}').expect("suffix present");
    assert_eq!(
        compact[1][suffix_start].fg, muted,
        "the ` … +N` suffix is muted"
    );
}

#[test]
fn compact_view_leaves_single_line_message_unchanged() {
    let m = [msg("planner", "just one line")];
    let refs: Vec<&Message> = m.iter().collect();
    let full = card_lines(&refs, 40, 0, View::default());
    let compact = card_lines(
        &refs,
        40,
        0,
        View {
            gap_rows: crate::density::Density::Cozy.card_gap_rows(),
            source: false,
            compact: true,
            streaming_from: usize::MAX,
        },
    );
    let text = |lines: &[CardLine]| -> Vec<String> {
        lines
            .iter()
            .map(|l| l.iter().map(|c| c.c).collect())
            .collect()
    };
    assert_eq!(
        text(&full),
        text(&compact),
        "a single-line body renders identically in both modes"
    );
}

#[test]
fn compact_view_shrinks_card_line_count() {
    let m = [
        msg("planner", "one\ntwo\nthree"),
        msg("coder", "just one line"),
    ];
    let refs: Vec<&Message> = m.iter().collect();
    let full = card_line_count(&refs, 40, View::default());
    let compact = card_line_count(
        &refs,
        40,
        View {
            gap_rows: crate::density::Density::Cozy.card_gap_rows(),
            source: false,
            compact: true,
            streaming_from: usize::MAX,
        },
    );
    assert!(
        compact < full,
        "compact mode must shrink the total line count: full={full} compact={compact}"
    );
    // header+body(clamped to 1)+spacer+header+body(unchanged, single line) = 5
    assert_eq!(compact, 5);
}

#[test]
fn compact_view_and_source_view_are_orthogonal() {
    // Both on at once: raw text, clamped to one line — no special case.
    let m = [msg("coder", "**one**\ntwo\nthree")];
    let refs: Vec<&Message> = m.iter().collect();
    let both = card_lines(
        &refs,
        40,
        0,
        View {
            gap_rows: crate::density::Density::Cozy.card_gap_rows(),
            source: true,
            compact: true,
            streaming_from: usize::MAX,
        },
    );
    assert_eq!(both.len(), 2, "header + clamped first line");
    let body_text: String = both[1].iter().map(|c| c.c).collect();
    assert!(
        body_text.starts_with(" **one**"),
        "source mode keeps literal markdown even while compact: {body_text}"
    );
    assert!(body_text.ends_with(" \u{2026} +2"), "got: {body_text}");
}

// --- streaming caret --------------------------------------------------------

/// The caret is the one unambiguous sign that text is still arriving, as
/// distinct from a reply that happened to end mid-sentence.
#[test]
fn a_streaming_card_ends_in_a_caret() {
    let _g = crate::app::motion_test_guard();
    crate::motion::set_level(crate::motion::MotionLevel::Full);
    let m = msg("alice", "thinking about it");
    let settled = card_lines(
        &[&m],
        40,
        0,
        View {
            gap_rows: crate::density::Density::Cozy.card_gap_rows(),
            source: false,
            compact: false,
            streaming_from: usize::MAX,
        },
    );
    let live = card_lines(
        &[&m],
        40,
        0,
        View {
            gap_rows: crate::density::Density::Cozy.card_gap_rows(),
            source: false,
            compact: false,
            streaming_from: 0,
        },
    );
    let tail = |ls: &[CardLine]| ls.last().and_then(|l| l.last().map(|c| c.c));
    assert_eq!(tail(&settled), Some('t'), "a settled card ends in its text");
    assert_eq!(tail(&live), Some('\u{258c}'), "a live one ends in a caret");
}

/// It pulses rather than blinking: a caret that vanishes half the time reads,
/// at a glance, like the text stopped.
#[test]
fn the_caret_pulses_without_disappearing() {
    let _g = crate::app::motion_test_guard();
    crate::motion::set_level(crate::motion::MotionLevel::Full);
    let m = msg("alice", "streaming");
    let caret_fg = |now| {
        card_lines(
            &[&m],
            40,
            now,
            View {
                gap_rows: crate::density::Density::Cozy.card_gap_rows(),
                source: false,
                compact: false,
                streaming_from: 0,
            },
        )
        .last()
        .and_then(|l| l.last().map(|c| c.fg))
        .unwrap()
    };
    // Two points a half-period apart differ — it is moving...
    assert_ne!(caret_fg(1), caret_fg(450));
    // ...and it is present at both, because the glyph never goes away.
    for now in [1_u64, 225, 450, 675] {
        let last = card_lines(
            &[&m],
            40,
            now,
            View {
                gap_rows: crate::density::Density::Cozy.card_gap_rows(),
                source: false,
                compact: false,
                streaming_from: 0,
            },
        );
        assert_eq!(
            last.last().and_then(|l| l.last().map(|c| c.c)),
            Some('\u{258c}'),
            "the caret vanished at {now}ms"
        );
    }
}

/// Reduce-motion keeps the caret — it carries information — but holds it still.
#[test]
fn motion_off_leaves_a_steady_caret() {
    let _g = crate::app::motion_test_guard();
    crate::motion::set_level(crate::motion::MotionLevel::Off);
    let m = msg("alice", "streaming");
    let fg = |now| {
        card_lines(
            &[&m],
            40,
            now,
            View {
                gap_rows: crate::density::Density::Cozy.card_gap_rows(),
                source: false,
                compact: false,
                streaming_from: 0,
            },
        )
        .last()
        .and_then(|l| l.last().map(|c| c.fg))
        .unwrap()
    };
    assert_eq!(fg(1), fg(450), "the caret must not pulse at off");
    crate::motion::set_level(crate::motion::MotionLevel::Full);
}

// --- Per-reply usage trailer (see chatusage) ---

fn msg_with_usage(sender: &str, text: &str) -> Message {
    let mut m = msg(sender, text);
    m.usage = Some((900, 50, 12_000));
    m
}

#[test]
fn usage_trailer_renders_muted_under_the_body() {
    // The render and the muted assertion each read the process-global theme —
    // serialised so a theme-mutating test can't flip it in between.
    let _g = crate::app::theme_test_guard();
    let m = msg_with_usage("coder", "hello");
    let cells = message_cells(&[&m], 40, 10, 0, 0, View::default());
    assert_eq!(row_text(&cells, 1), " hello");
    assert_eq!(row_text(&cells, 2), " 900 in / 50 out \u{00b7} $0.012");
    let muted = crew_theme::theme().text_muted;
    assert!(
        cells.iter().filter(|c| c.row == 2).all(|c| c.fg == muted),
        "the trailer renders in the muted system ink"
    );
}

#[test]
fn card_line_count_includes_the_trailer() {
    let with = msg_with_usage("coder", "hello");
    let without = msg("coder", "hello");
    assert_eq!(
        card_line_count(&[&with], 40, View::default()),
        card_line_count(&[&without], 40, View::default()) + 1,
        "the scroll clamp must count the trailer line"
    );
}

#[test]
fn zero_usage_renders_no_trailer() {
    let mut m = msg("coder", "hello");
    m.usage = Some((0, 0, 0));
    assert_eq!(
        card_line_count(&[&m], 40, View::default()),
        card_line_count(&[&msg("coder", "hello")], 40, View::default()),
        "an all-zero usage row renders exactly like one with none"
    );
}

#[test]
fn compact_view_excludes_the_usage_trailer_from_the_clamp() {
    // The trailer is metadata, not content: a single-line reply with usage
    // must render unclamped in compact view — a ` … +1` stamped on every
    // such reply would claim content is hidden when none is.
    let m = msg_with_usage("coder", "hello");
    let view = View {
        compact: true,
        ..View::default()
    };
    let cells = message_cells(&[&m], 40, 10, 0, 0, view);
    assert_eq!(
        row_text(&cells, 1),
        " hello",
        "a single-line reply renders unclamped, as pre-0.10.6"
    );
    assert!(
        !(0..10).any(|r| {
            let t = row_text(&cells, r);
            t.contains("in /") || t.contains('\u{2026}')
        }),
        "no trailer text and no hidden-count stamp in compact view"
    );
    // A genuinely multi-line body still clamps — counting body lines only.
    let m = msg_with_usage("coder", "one\ntwo");
    let cells = message_cells(&[&m], 40, 10, 0, 0, view);
    assert_eq!(
        row_text(&cells, 1),
        " one \u{2026} +1",
        "the trailer is not part of the hidden-line count"
    );
}

/// Density is only a real setting if the transcript actually re-spaces. Three
/// unrelated cards, one density apart: the blank rows between them are the
/// whole feature, and a `card_gap_rows` that was read but ignored would leave
/// all three lengths equal.
#[test]
fn density_sets_the_blank_rows_between_unrelated_cards() {
    use crate::density::Density;
    let a = msg("planner", "one");
    let b = msg("coder", "two");
    let c = msg("planner", "three");
    let refs = [&a, &b, &c];

    let mut blanks = Vec::new();
    for d in Density::ALL {
        let view = View {
            gap_rows: d.card_gap_rows(),
            ..View::default()
        };
        let lines = card_lines(&refs, 40, 0, view);
        blanks.push(lines.iter().filter(|l| l.is_empty()).count());
        // Two gaps, each `card_gap_rows` tall, on top of three one-line cards.
        assert_eq!(
            lines.len(),
            6 + 2 * d.card_gap_rows(),
            "{} laid out {} lines",
            d.as_str(),
            lines.len()
        );
    }
    assert_eq!(blanks, vec![0, 2, 4], "the ladder must actually step");
}

/// Chained replies of one task are held together by the tree connector, and
/// spacing them at any density would contradict it — `roomy` must open the
/// space *between* threads, not inside one.
#[test]
fn chained_replies_take_no_spacer_even_at_the_roomiest() {
    use crate::density::Density;
    let mut a = msg("planner", "one");
    let mut b = msg("planner", "two");
    a.meta = "task:7".into();
    b.meta = "task:7".into();
    // Two `None`s compare equal, so an unparseable tag would make this test
    // pass by proving nothing about chaining. Require a real id.
    assert_eq!(crate::chattime::task_tag(&a.meta), Some(7));
    assert_eq!(crate::chattime::task_tag(&b.meta), Some(7));
    let view = View {
        gap_rows: Density::Roomy.card_gap_rows(),
        ..View::default()
    };
    let lines = card_lines(&[&a, &b], 40, 0, view);
    assert_eq!(
        lines.iter().filter(|l| l.is_empty()).count(),
        0,
        "a chained pair took a spacer"
    );
}
