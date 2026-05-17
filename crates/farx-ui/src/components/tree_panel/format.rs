/// Format Unix permission mode bits as rwxrwxrwx string.
pub(super) fn format_permissions(mode: u32) -> String {
    let mut s = String::with_capacity(9);
    let flags = [
        (0o400, 'r'),
        (0o200, 'w'),
        (0o100, 'x'),
        (0o040, 'r'),
        (0o020, 'w'),
        (0o010, 'x'),
        (0o004, 'r'),
        (0o002, 'w'),
        (0o001, 'x'),
    ];
    for (bit, ch) in flags {
        s.push(if mode & bit != 0 { ch } else { '-' });
    }
    s
}

pub(super) fn format_size(size: u64) -> String {
    if size < 1_000 {
        format!("{size} B")
    } else if size < 1_000_000 {
        format!("{:.1}K", size as f64 / 1_024.0)
    } else if size < 1_000_000_000 {
        format!("{:.1}M", size as f64 / 1_048_576.0)
    } else {
        format!("{:.1}G", size as f64 / 1_073_741_824.0)
    }
}
