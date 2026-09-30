//! The line under a `sys:read_file` page that says where it stopped and how
//! to go on. Split out of `sysread` when the page learned `"lines"`, which
//! put that file over the line cap.

/// The page's last line: where it sits, in lines as grep and edit results
/// count them, and the offset to go on from. One line with `{"offset": N}`
/// LAST, because `toolclip` keeps the final line verbatim and the agent
/// copies that JSON straight into its next call.
pub(super) fn note(
    lines: Option<(usize, usize, usize, bool)>,
    from: usize,
    to: usize,
    total: usize,
) -> String {
    let n = grouped;
    let lines = match lines {
        Some((a, _, of, true)) => format!("part of line {} of {}, ", n(a), n(of)),
        Some((a, b, of, _)) if a == b => format!("line {} of {}, ", n(a), n(of)),
        Some((a, b, of, _)) => format!("lines {}\u{2013}{} of {}, ", n(a), n(b), n(of)),
        None => String::new(),
    };
    format!(
        "\u{2026} ({lines}bytes {}\u{2013}{} of {} \u{2014} continue with {{\"offset\": {to}}})",
        n(from),
        n(to),
        n(total)
    )
}

/// `40112` → `40,112`, for the prose only: the `{"offset": N}` beside it
/// stays bare digits, because the agent pastes that into JSON.
pub(crate) fn grouped(n: usize) -> String {
    let s = n.to_string();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}
