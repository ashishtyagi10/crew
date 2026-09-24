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
