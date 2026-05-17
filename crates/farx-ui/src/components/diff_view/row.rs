pub(super) fn truncate_pad(text: &str, width: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() >= width {
        let mut s: String = chars[..width.saturating_sub(1)].iter().collect();
        s.push('~');
        s
    } else {
        format!("{:<width$}", text, width = width)
    }
}
