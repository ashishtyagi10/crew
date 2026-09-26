//! The onboarding's paragraph wrap: balanced, on spaces, dashes kept
//! with the word they follow. Split from `chatempty` at its line cap.

/// Columns a wrapped onboarding line may take.
pub(crate) fn wrap_width(cols: u16) -> usize {
    (cols.saturating_sub(4)).max(12) as usize
}

/// Wrap `advice` to the pane's width, on spaces, with a sentence capital —
/// BALANCED: the fewest rows the width allows, at the narrowest measure that
/// still needs no more. Greedy filling left `commands.` alone under a full
/// row; text that reads as a paragraph ends near where it started.
/// Pure so the wrapping is testable without a pane.
pub(crate) fn wrap_to(advice: &str, cols: u16) -> Vec<String> {
    let width = wrap_width(cols);
    let rows = greedy(advice, width).len();
    let even = (width / 2..width)
        .find(|&w| greedy(advice, w).len() == rows)
        .unwrap_or(width);
    let mut out = greedy(advice, even);
    if let Some(first) = out.first_mut() {
        let mut c = first.chars();
        if let Some(f) = c.next() {
            *first = f.to_uppercase().collect::<String>() + c.as_str();
        }
    }
    out
}

/// `advice` filled word by word into rows of at most `width` columns. A
/// spaced dash rides with the word before it: a row may end `result —`,
/// never start `— or “draft a plan first”`.
pub(crate) fn greedy(advice: &str, width: usize) -> Vec<String> {
    let mut words: Vec<String> = Vec::new();
    for word in advice.split_whitespace() {
        match words.last_mut() {
            Some(last) if word == "\u{2014}" => *last = format!("{last} {word}"),
            _ => words.push(word.to_string()),
        }
    }
    let mut out: Vec<String> = Vec::new();
    let mut line = String::new();
    for word in words {
        if !line.is_empty() && line.chars().count() + 1 + word.chars().count() > width {
            out.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(&word);
    }
    if !line.is_empty() {
        out.push(line);
    }
    out
}

#[cfg(test)]
#[path = "balancewrap_tests.rs"]
mod tests;
