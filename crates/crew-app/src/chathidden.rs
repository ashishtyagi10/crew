//! The ` … +N` a clamped card ends its one shown line with.
use crate::chatbody::{plain, CardLine};
use crate::chatwidth::char_w;

/// Appends a muted ` … +N` suffix (`hidden` = number of clamped-away body
/// lines) to a compact-clamped first body line, trimming trailing cells so
/// the line plus the suffix still fits `cols` (`line_cells` would otherwise
/// silently drop overflow, cutting the suffix instead of the body text).
///
/// A line that has to give up cells gives up whole WORDS. Popping cells
/// until it fit left `…you already started, touche … +9` on every folded
/// fan reply — a word cut at a letter reads as a typo, not as a cut. A line
/// that is one long word (a path, a URL) has no boundary to back up to and
/// keeps the letter cut.
pub(crate) fn append_hidden_suffix(line: &mut CardLine, hidden: usize, cols: usize) {
    let muted = crew_theme::theme().text_muted;
    let suffix = format!(" \u{2026} +{hidden}");
    let suffix_w: usize = suffix.chars().map(char_w).sum();
    let mut w: usize = line.iter().map(|c| char_w(c.c)).sum();
    if w + suffix_w > cols {
        let mut cut = line.len();
        while cut > 0 && w + suffix_w > cols {
            cut -= 1;
            w -= char_w(line[cut].c);
        }
        let mid_word = cut > 0 && line[cut].c != ' ' && line[cut - 1].c != ' ';
        let word_start = line[..cut]
            .iter()
            .rposition(|c| c.c == ' ')
            .filter(|&sp| line[..sp].iter().any(|c| c.c != ' '));
        if let (true, Some(sp)) = (mid_word, word_start) {
            cut = sp;
        }
        line.truncate(cut);
        while line.len() > 1 && line.last().is_some_and(|c| c.c == ' ') {
            line.pop();
        }
    }
    line.extend(suffix.chars().map(|c| plain(c, muted, false)));
}

#[cfg(test)]
#[path = "chathidden_tests.rs"]
mod tests;
