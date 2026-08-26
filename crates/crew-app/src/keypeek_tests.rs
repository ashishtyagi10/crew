use super::*;
use winit::keyboard::ModifiersState;

/// The whole point of the "alone" rule: someone mid-chord already knows what
/// they are doing, and a panel that opens during Cmd+Shift+G is in the way of
/// the thing it claims to teach.
#[test]
fn only_a_bare_modifier_opens_the_hints() {
    let mac = cfg!(target_os = "macos");
    let primary = if mac {
        ModifiersState::SUPER
    } else {
        ModifiersState::CONTROL
    };
    let secondary = if mac {
        ModifiersState::CONTROL
    } else {
        ModifiersState::ALT
    };
    assert_eq!(held_alone(primary), Some(Held::Primary));
    assert_eq!(held_alone(secondary), Some(Held::Secondary));
    assert_eq!(held_alone(ModifiersState::empty()), None, "nothing held");
    assert_eq!(
        held_alone(primary | secondary),
        None,
        "two modifiers is a chord in progress"
    );
    // Shift rides along with real chords (Cmd+Shift+arrow), so it must not
    // count as company — and must not open the panel on its own either.
    assert_eq!(
        held_alone(primary | ModifiersState::SHIFT),
        Some(Held::Primary)
    );
    assert_eq!(held_alone(ModifiersState::SHIFT), None, "typing capitals");
}

/// Context is the entire reason this exists beside `/keys`. If every place
/// answered identically, the static table would already be the better tool.
#[test]
fn the_hints_change_with_what_is_focused() {
    let input = line(Held::Primary, Where::Input);
    let chat = line(Held::Primary, Where::Chat);
    assert_ne!(input, chat);
    assert!(input.contains("send"), "{input}");
    assert!(chat.contains("clear") && chat.contains("zoom"), "{chat}");
    assert!(
        !input.contains("clear"),
        "the input bar has no scrollback to clear: {input}"
    );
}

/// A row that names the wrong key on this platform is worse than no row.
#[test]
fn the_row_leads_with_this_platforms_modifier_name() {
    for (held, name) in [
        (Held::Primary, primary_name()),
        (Held::Secondary, secondary_name()),
    ] {
        let l = line(held, Where::Other);
        assert!(l.starts_with(name), "{l} should start with {name}");
    }
    assert_ne!(primary_name(), secondary_name());
}

/// Every chip has to carry both halves — a key with no verb teaches nothing,
/// and a verb with no key cannot be pressed.
#[test]
fn every_chip_names_a_key_and_what_it_does() {
    for held in [Held::Primary, Held::Secondary] {
        for at in [Where::Input, Where::Chat, Where::Terminal, Where::Other] {
            let c = chips(held, at);
            assert!(!c.is_empty(), "{held:?} at {at:?} offered nothing");
            for (k, d) in c {
                assert!(
                    !k.is_empty() && !d.is_empty(),
                    "{held:?} {at:?}: {k:?} {d:?}"
                );
            }
        }
    }
}

/// The dwell is the whole reason an ordinary Cmd+C never flashes a panel.
/// A `peek_open` that ignored it would put a card on screen on every chord.
#[test]
fn the_panel_waits_out_the_dwell_and_closes_on_release() {
    let mut app = crate::app::CrewApp::default();
    let primary = if cfg!(target_os = "macos") {
        ModifiersState::SUPER
    } else {
        ModifiersState::CONTROL
    };
    app.mods = winit::event::Modifiers::from(primary);
    app.peek_since = Some(1_000);

    assert!(!app.peek_open(1_000), "open at the instant of the press");
    assert!(
        !app.peek_open(1_000 + DWELL_MS - 1),
        "a chord struck just under the dwell must never flash the panel"
    );
    assert!(app.peek_open(1_000 + DWELL_MS));
    assert!(app.peek_line(1_000 + DWELL_MS).is_some());

    // Letting go closes it even though the dwell has long since passed —
    // `peek_since` alone is not enough, the modifier has to still be down.
    app.mods = winit::event::Modifiers::default();
    assert!(!app.peek_open(9_999));
    assert!(app.peek_line(9_999).is_none());
}

/// A second modifier turns a rest into a chord, and the panel must get out of
/// the way — otherwise it covers the palette that Cmd+Shift+P just opened.
#[test]
fn reaching_for_a_second_modifier_closes_the_panel() {
    let mut app = crate::app::CrewApp::default();
    let (primary, secondary) = if cfg!(target_os = "macos") {
        (ModifiersState::SUPER, ModifiersState::CONTROL)
    } else {
        (ModifiersState::CONTROL, ModifiersState::ALT)
    };
    app.peek_since = Some(0);
    app.mods = winit::event::Modifiers::from(primary);
    assert!(app.peek_open(DWELL_MS));
    app.mods = winit::event::Modifiers::from(primary | secondary);
    assert!(!app.peek_open(DWELL_MS), "a chord in progress");
}

/// The card must fit the width it is given, or it is the `/keys` truncation
/// bug again — a hint panel that teaches the half of the instruction that fit.
#[test]
fn the_card_stays_inside_the_columns_it_is_given() {
    for cols in [24u16, 40, 80, 200] {
        let cells = peek_card(&line(Held::Primary, Where::Chat), cols);
        assert!(!cells.is_empty(), "{cols} columns drew nothing");
        assert!(
            cells.iter().all(|c| c.col < cols),
            "a cell escaped {cols} columns"
        );
        assert!(cells.iter().all(|c| c.row < 3), "a cell escaped 3 rows");
    }
}
