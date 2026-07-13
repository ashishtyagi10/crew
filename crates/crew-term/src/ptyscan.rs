//! Watch-pattern scanning over PTY output (ANSI-stripped, chunk-boundary
//! tolerant) — child module of `pty`, split for the 200-line cap.
use super::*;

/// Scan a freshly-read `chunk` for any watched `patterns` (already lowercased,
/// matched case-insensitively). `tail` carries the trailing partial line between
/// calls so a pattern split across reads still matches. ANSI escape sequences are
/// stripped before matching. Returns the patterns that matched a line completed
/// by this chunk. Pure and bounded — safe to run on the main thread.
pub(super) fn scan(tail: &mut String, chunk: &[u8], patterns: &[String]) -> Vec<String> {
    if patterns.is_empty() {
        return Vec::new();
    }
    tail.push_str(&strip_ansi(&String::from_utf8_lossy(chunk)));
    let mut hits = Vec::new();
    // Everything up to and including the last line break is a set of completed
    // lines — scan it, then carry only the trailing partial line.
    if let Some(idx) = tail.rfind(['\n', '\r']) {
        let rest = tail.split_off(idx + 1);
        scan_into(tail, patterns, &mut hits);
        *tail = rest;
    }
    // A newline-free flood must not grow the carry without bound.
    if tail.len() > SCAN_CARRY_CAP {
        scan_into(tail, patterns, &mut hits);
        let cut = tail.len() - SCAN_CARRY_CAP;
        let cut = (cut..=tail.len())
            .find(|&i| tail.is_char_boundary(i))
            .unwrap_or(tail.len());
        *tail = tail[cut..].to_string();
    }
    hits
}

/// Push every pattern present in `hay` (case-insensitive) into `hits`, once each.
fn scan_into(hay: &str, patterns: &[String], hits: &mut Vec<String>) {
    let lower = hay.to_lowercase();
    for p in patterns {
        if lower.contains(p) && !hits.contains(p) {
            hits.push(p.clone());
        }
    }
}

/// Strip ANSI escape sequences (CSI `ESC [ … final`, OSC `ESC ] … BEL/ST`, and
/// other two-char `ESC x` escapes) and stray C0 control bytes (keeping `\n \r
/// \t`), so pattern matching sees plain text rather than colour codes.
pub(super) fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            match chars.peek() {
                Some('[') => {
                    chars.next();
                    // CSI runs until a final byte in 0x40..=0x7e (`@`..=`~`).
                    while let Some(&nc) = chars.peek() {
                        chars.next();
                        if ('@'..='~').contains(&nc) {
                            break;
                        }
                    }
                }
                Some(']') => {
                    chars.next();
                    // OSC runs until BEL or ST (`ESC \`).
                    while let Some(&nc) = chars.peek() {
                        chars.next();
                        if nc == '\u{07}' {
                            break;
                        }
                        if nc == '\u{1b}' {
                            if chars.peek() == Some(&'\\') {
                                chars.next();
                            }
                            break;
                        }
                    }
                }
                _ => {
                    chars.next();
                }
            }
            continue;
        }
        if c.is_control() && c != '\n' && c != '\r' && c != '\t' {
            continue;
        }
        out.push(c);
    }
    out
}
