use super::*;

#[test]
fn fmt_uptime_buckets() {
    assert_eq!(fmt_uptime(30), "up 0m");
    assert_eq!(fmt_uptime(12 * 60), "up 12m");
    assert_eq!(fmt_uptime(3 * 3600 + 12 * 60), "up 3h 12m");
    assert_eq!(fmt_uptime(2 * 86400 + 3 * 3600), "up 2d 3h");
}

#[test]
fn host_section_has_rule_and_name() {
    let _g = crate::app::theme_test_guard();
    let cells = host_cells("mbp · macOS", "up 1h 2m", 24);
    assert!(cells.iter().any(|c| c.c == '─' && c.row == 0));
    assert!(!cells.iter().any(|c| c.c == '╭'));
    assert!(cells.iter().any(|c| c.c == 'H' && c.row == 0)); // HOST legend
    assert!(cells.iter().any(|c| c.c == 'm' && c.row == 1)); // name
}

/// A name too long for a narrow nav says so, rather than stopping in the
/// middle of a word and looking like the machine is called `Darw`.
#[test]
fn a_long_host_name_ellipsizes() {
    let _g = crate::app::theme_test_guard();
    let cells = host_cells("Mac.lan · Darwin", "up 2h 6m", 18);
    let row: String = {
        let mut v: Vec<_> = cells.iter().filter(|c| c.row == 1).collect();
        v.sort_by_key(|c| c.col);
        v.iter().map(|c| c.c).collect()
    };
    assert!(row.ends_with('…'), "{row:?}");
    assert!(
        row.starts_with("Mac.lan"),
        "the head still names it: {row:?}"
    );
}
