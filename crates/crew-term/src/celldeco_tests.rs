use super::*;

#[test]
fn every_underline_flag_maps_to_its_own_rule() {
    for (flag, want) in [
        (Flags::UNDERLINE, DecoLine::Single),
        (Flags::DOUBLE_UNDERLINE, DecoLine::Double),
        (Flags::UNDERCURL, DecoLine::Curly),
        (Flags::DOTTED_UNDERLINE, DecoLine::Dotted),
        (Flags::DASHED_UNDERLINE, DecoLine::Dashed),
    ] {
        assert_eq!(line_of(flag), want, "{flag:?}");
    }
}

/// SGR 4 then SGR 4:3 leaves both bits set. The squiggle is what the program
/// last asked for, and the one that carries meaning.
#[test]
fn the_most_specific_underline_wins_when_several_are_set() {
    let both = Flags::UNDERLINE | Flags::UNDERCURL;
    assert_eq!(line_of(both), DecoLine::Curly);
    let three = both | Flags::DOUBLE_UNDERLINE;
    assert_eq!(line_of(three), DecoLine::Curly);
    assert_eq!(
        line_of(Flags::UNDERLINE | Flags::DOUBLE_UNDERLINE),
        DecoLine::Double
    );
}

#[test]
fn an_ordinary_cell_wears_nothing() {
    let plain = Flags::BOLD | Flags::ITALIC;
    assert_eq!(line_of(plain), DecoLine::None);
    assert!(!decorated(plain));
    assert!(deco_of(plain, None).is_blank());
}

/// A struck cell has no underline at all, and still has to survive the
/// blank-cell filter — the strike crosses the spaces between words.
#[test]
fn a_struck_cell_is_decorated_without_an_underline() {
    let f = Flags::STRIKEOUT;
    assert_eq!(line_of(f), DecoLine::None);
    assert!(decorated(f));
    let d = deco_of(f, None);
    assert!(d.strike && !d.is_blank());
}

#[test]
fn sgr58s_colour_rides_along_and_is_absent_when_unset() {
    let f = Flags::UNDERCURL;
    assert_eq!(deco_of(f, Some((255, 0, 0))).color, Some((255, 0, 0)));
    assert_eq!(deco_of(f, None).color, None);
}
