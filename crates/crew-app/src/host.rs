//! Sidebar host section: a `HOST` divider above the machine name, OS, and
//! uptime — static system info that complements the live clock + gauges.
use crew_render::CellView;

use crate::boxdraw::section_header;

use crate::palette::accent;

/// Current `(name, uptime)` display strings, e.g. `("mbp · macOS", "up 3h 12m")`.
pub fn host_strings() -> (String, String) {
    let host = sysinfo::System::host_name().unwrap_or_else(|| "crew".to_string());
    let os = sysinfo::System::name().unwrap_or_default();
    let name = if os.is_empty() {
        host
    } else {
        format!("{host} · {os}")
    };
    (name, fmt_uptime(sysinfo::System::uptime()))
}

/// Format seconds of uptime compactly: `up 2d 3h`, `up 3h 12m`, or `up 12m`.
fn fmt_uptime(secs: u64) -> String {
    let (d, h, m) = (secs / 86400, (secs % 86400) / 3600, (secs % 3600) / 60);
    if d > 0 {
        format!("up {d}d {h}h")
    } else if h > 0 {
        format!("up {h}h {m}m")
    } else {
        format!("up {m}m")
    }
}

/// Render the host section: a `HOST` rule on row 0, `name` and `uptime`
/// beneath. Both are prose, so both ellipsize rather than cutting mid-word —
/// at the narrow end of the resize range this read `Mac.lan · Darw`.
pub fn host_cells(name: &str, uptime: &str, cols: u16) -> Vec<CellView> {
    if cols < 10 {
        return Vec::new();
    }
    let t = crew_theme::theme();
    let mut out = section_header("HOST", cols, t.border_normal, accent(), t.page_bg);
    crate::navtext::put(&mut out, name, 1, cols, t.ink);
    crate::navtext::put(&mut out, uptime, 2, cols, t.text_muted);
    out
}

#[cfg(test)]
mod tests {
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
}
