//! Sidebar host section: a `HOST` divider above the machine name, OS, and
//! uptime — static system info that complements the live clock + gauges.
use crew_render::CellView;

use crate::boxdraw::section_header;

use crate::palette::accent;

/// Current `(name, uptime)` display strings, e.g. `("mbp · macOS", "up 3h 12m")`.
pub fn host_strings() -> (String, String) {
    let (host, os, up) = host_parts();
    let name = if os.is_empty() {
        host
    } else {
        format!("{host} · {os}")
    };
    (name, up)
}

/// The nav card's two lines: the name alone, then `macOS · up 3h 12m`.
pub fn card_lines() -> (String, String) {
    let (host, os, up) = host_parts();
    match os.is_empty() {
        true => (host, up),
        false => (host, format!("{os} · {up}")),
    }
}

/// The machine as three words: `("mbp", "macOS", "up 3h 12m")`. The nav's
/// narrow card puts the OS beside the uptime rather than after the name,
/// where a MacBook's default hostname pushed it past the ellipsis.
pub fn host_parts() -> (String, String, String) {
    let host = sysinfo::System::host_name().unwrap_or_else(|| "crew".to_string());
    let os = sysinfo::System::name().unwrap_or_default();
    (
        plain_host(&host).to_string(),
        plain_os(&os).to_string(),
        fmt_uptime(sysinfo::System::uptime()),
    )
}

/// The name without its local-network suffix: every Mac answers
/// `name.local` (mDNS), and a home router hands out `.lan`, `.home` or
/// `.localdomain`. None of them says anything about the machine, and each
/// costs a narrow card its columns.
fn plain_host(host: &str) -> &str {
    const SUFFIXES: [&str; 4] = [".local", ".lan", ".home", ".localdomain"];
    SUFFIXES
        .iter()
        .find_map(|s| host.strip_suffix(s).filter(|h| !h.is_empty()))
        .unwrap_or(host)
}

/// `Darwin` is the kernel; the person at the keyboard runs macOS.
fn plain_os(os: &str) -> &str {
    match os {
        "Darwin" => "macOS",
        o => o,
    }
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
#[path = "host_tests.rs"]
mod tests;
