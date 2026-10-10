use super::*;

/// On paper the code block is a step off the page — visible, never the
/// mid-grey slab the tube floor made of it — and its code still reads.
#[test]
fn a_light_page_gets_a_quiet_field() {
    for id in crew_theme::ALL_THEMES {
        let t = id.theme();
        if t.dark {
            continue;
        }
        let d = crate::chatink::derive(t);
        let field = contrast_ratio(d.code_bg, t.page_bg);
        assert!(
            (1.15..1.3).contains(&field),
            "{}: code field vs page = {field:.3}",
            id.as_str()
        );
        assert!(contrast_ratio(d.code, d.code_bg) >= CODE_ON_FIELD_FLOOR);
    }
}

/// A comment is read on the field, so it is floored there: 3.5:1 against
/// the page left it at 2.7:1 on every tube's field (3.3 on paper-dark). A
/// tube gives a little of that to its ladder (comment a rung under code),
/// and still clears 3:1.
#[test]
fn a_comment_reads_on_the_field_it_is_drawn_on() {
    for id in crew_theme::ALL_THEMES {
        let t = id.theme();
        let ink = crate::chatink::derive(t);
        let got = contrast_ratio(ink.comment, ink.code_bg);
        let need = if t.is_tube() { 3.0 } else { 3.5 };
        assert!(
            got >= need,
            "{}: comment on its field is {got:.2}",
            id.as_str()
        );
    }
}

/// On the glass the field is a frost the desktop shows through, so its code,
/// strings and comments are floored on it over a black desktop AND a white
/// one: on the opaque colour alone a light palette's comment read 2.8:1
/// over white, its strings 3.0 (glass survey D-H1). A tube keeps its
/// lightness ladder (`glassed`).
#[test]
fn the_field_reads_over_any_desktop() {
    for id in crew_theme::ALL_THEMES {
        let t = id.theme();
        if t.is_tube() {
            continue;
        }
        let d = crate::chatink::derive(t);
        let grounds = field_grounds(t, d.code_bg);
        let room = comment_room(t, d.code, d.code_bg);
        for (what, c, floor) in [
            ("comment", d.comment, room),
            ("string", d.string, room),
            ("code", d.code, CODE_ON_FIELD_FLOOR),
        ] {
            let worst = crew_theme::glasslegend::worst(c, &grounds);
            assert!(
                worst >= floor - 0.01,
                "{}: {what} {c:?} reads {worst:.2} on the field (floor {floor:.2})",
                id.as_str()
            );
        }
    }
}
