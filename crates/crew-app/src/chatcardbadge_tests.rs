use super::*;
use crate::chatmsgs::tests::msg;
use crate::glyphs::force;
use crew_theme::contrast_ratio;

fn chars(line: &CardLine) -> String {
    line.iter().map(|c| c.c).collect()
}

/// On main the header was `▍planner`: the name in the agent's colour as
/// ink. It is a badge now — the name on the agent's colour as a BLOCK,
/// capped, the gutter still in that colour, and the ink on the block
/// walked to the text floor.
#[test]
fn an_agents_name_is_a_badge_on_its_roster_colour() {
    let _g = crate::app::theme_test_guard();
    let _off = force(false);
    let line = header_line(&msg("planner", "hello"), 0, None);
    assert_eq!(chars(&line), format!("{GUTTER}\u{2590} planner \u{258c}"));
    let want = crate::chatroster::agent_color("planner");
    assert_eq!(line[0].fg, want, "the gutter keeps the colour");
    assert_eq!(
        (line[1].fg, line[1].bg),
        (want, None),
        "left cap on the page"
    );
    assert_eq!((line[11].fg, line[11].bg), (want, None), "right cap");
    let floor = crew_theme::contrast::text_floor();
    for cell in &line[2..11] {
        assert_eq!(cell.bg, Some(want), "{:?} on the block", cell.c);
        assert!(
            contrast_ratio(cell.fg, want) >= floor,
            "{:?}: {:?} on {want:?}",
            cell.c,
            cell.fg
        );
    }
    assert!(line[3..10].iter().all(|c| c.bold), "the name is bold");
}

/// The user's own cards keep the plain name: "me" and "them" part by shape.
/// The system voice and a tool card stay quiet too — a badge is for a
/// voice, and those are the machine talking.
#[test]
fn the_user_the_system_voice_and_a_tool_card_keep_the_plain_name() {
    let _g = crate::app::theme_test_guard();
    let _off = force(false);
    let user = header_line(&msg("user", "hi"), 0, None);
    assert_eq!(chars(&user), format!("{GUTTER}user"));
    assert!(user.iter().all(|c| c.bg.is_none()), "no block on the user");
    assert!(user[1].bold, "still the bold name");
    let sys = header_line(&msg("crew", "x"), 0, None);
    assert_eq!(chars(&sys), "\u{2506}crew");
    let tool = header_line(&msg("planner", &format!("{TOOL_PREFIX}ls")), 0, None);
    assert_eq!(chars(&tool), "\u{2506}planner");
    assert!(tool.iter().all(|c| c.bg.is_none()));
}

/// A hand-off badges each agent in its own colour, the user plain, the
/// arrow muted between them — from → to, at a glance.
#[test]
fn a_handoff_badges_each_agent_and_leaves_the_user_plain() {
    let _g = crate::app::theme_test_guard();
    let _off = force(false);
    let line = header_line(&msg("planner \u{2192} user", "x"), 0, None);
    assert_eq!(
        chars(&line),
        format!("{GUTTER}\u{2590} planner \u{258c} \u{2192} user")
    );
    let both = header_line(&msg("planner \u{2192} coder", "x"), 0, None);
    assert_eq!(
        chars(&both),
        format!("{GUTTER}\u{2590} planner \u{258c} \u{2192} \u{2590} coder \u{258c}")
    );
    let blocks: Vec<Color> = both.iter().filter_map(|c| c.bg).collect();
    assert!(blocks.contains(&crate::chatroster::agent_color("planner")));
    assert!(blocks.contains(&crate::chatroster::agent_color("coder")));
    let arrow = both.iter().find(|c| c.c == '\u{2192}').expect("arrow");
    assert_eq!(arrow.fg, crew_theme::theme().text_muted);
}

/// On a Nerd Font the caps are the powerline arcs, one cell each.
#[test]
fn the_caps_are_powerline_arcs_on_a_nerd_font() {
    let _g = crate::app::theme_test_guard();
    let _on = force(true);
    let line = header_line(&msg("coder", "x"), 0, None);
    assert_eq!(chars(&line), format!("{GUTTER}\u{e0b6} coder \u{e0b4}"));
    assert!(line.iter().all(|c| crate::chatwidth::char_w(c.c) == 1));
}
