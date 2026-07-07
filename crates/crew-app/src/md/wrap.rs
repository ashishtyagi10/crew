//! Char-count word-wrap and hard-truncation primitives shared by prose,
//! list and table layout. Split out of `layout.rs` to keep that file under
//! its line budget.
use crate::md::{MdSpan, MdStyle};

/// Below this width, wrapping degrades to a single hard-truncated line
/// rather than word-wrapping — narrow enough that hanging indents and
/// bullets wouldn't fit anyway.
const MIN_WRAP_COLS: usize = 4;

pub(super) fn plain_span(text: String) -> MdSpan {
    MdSpan {
        text,
        style: MdStyle::default(),
        link: None,
    }
}

/// Hard-truncates `spans` to the first `cols` chars, splitting the boundary
/// span if needed. Used both as the narrow-column degrade path and to clip
/// table lines that don't fit.
pub(super) fn truncate_spans(spans: Vec<MdSpan>, cols: usize) -> Vec<MdSpan> {
    let mut out = Vec::new();
    let mut used = 0usize;
    for s in spans {
        if used >= cols {
            break;
        }
        let remaining = cols - used;
        let len = s.text.chars().count();
        if len <= remaining {
            used += len;
            out.push(s);
        } else {
            let text: String = s.text.chars().take(remaining).collect();
            out.push(MdSpan {
                text,
                style: s.style,
                link: s.link,
            });
            break;
        }
    }
    out
}

/// Word-wrap ranges (char offsets, half-open) over `full`, mirroring
/// `chatlayout::wrap_indices`' semantics but in char counts (layout never
/// tracks display width — that's the chat pane's job when it re-chunks).
fn wrap_ranges(full: &[char], cols: usize) -> Vec<(usize, usize)> {
    if cols == 0 || full.is_empty() {
        return vec![(0, full.len())];
    }
    let n = full.len();
    let mut ranges = Vec::new();
    let mut start = 0;
    while start < n {
        let max_end = (start + cols).min(n);
        if max_end == n {
            ranges.push((start, n));
            break;
        }
        match full[start..max_end].iter().rposition(|&c| c == ' ') {
            Some(p) if p > 0 => {
                ranges.push((start, start + p));
                start += p + 1;
            }
            _ => {
                ranges.push((start, max_end));
                start = max_end;
            }
        }
    }
    ranges
}

/// Cumulative char-count boundaries of `spans`: `bounds[i]` is the char
/// offset where span `i` starts, `bounds[len]` is the total char count.
fn span_bounds(spans: &[MdSpan]) -> Vec<usize> {
    let mut bounds = Vec::with_capacity(spans.len() + 1);
    let mut total = 0;
    bounds.push(0);
    for s in spans {
        total += s.text.chars().count();
        bounds.push(total);
    }
    bounds
}

/// Slices `spans` to the char range `[s, e)`, splitting spans that straddle
/// the boundary so styling survives the cut.
fn spans_for_range(spans: &[MdSpan], bounds: &[usize], s: usize, e: usize) -> Vec<MdSpan> {
    let mut out = Vec::new();
    for (i, sp) in spans.iter().enumerate() {
        let (sp_start, sp_end) = (bounds[i], bounds[i + 1]);
        let (lo, hi) = (s.max(sp_start), e.min(sp_end));
        if lo < hi {
            let text: String = sp.text.chars().skip(lo - sp_start).take(hi - lo).collect();
            out.push(MdSpan {
                text,
                style: sp.style,
                link: sp.link.clone(),
            });
        }
    }
    out
}

/// Word-wraps one run of styled spans (no hard breaks inside) to `cols`,
/// returning the spans for each output line.
pub(super) fn wrap_group(spans: &[MdSpan], cols: usize) -> Vec<Vec<MdSpan>> {
    if cols < MIN_WRAP_COLS {
        return vec![truncate_spans(spans.to_vec(), cols)];
    }
    let full: Vec<char> = spans.iter().flat_map(|s| s.text.chars()).collect();
    let bounds = span_bounds(spans);
    wrap_ranges(&full, cols)
        .into_iter()
        .map(|(s, e)| spans_for_range(spans, &bounds, s, e))
        .collect()
}

/// Splits `spans` on hard-break markers (`text == "\n"`, never rendered)
/// into the groups that must land on separate lines.
pub(super) fn split_hardbreaks(spans: Vec<MdSpan>) -> Vec<Vec<MdSpan>> {
    let mut groups = vec![Vec::new()];
    for s in spans {
        if s.text == "\n" {
            groups.push(Vec::new());
        } else {
            groups.last_mut().unwrap().push(s);
        }
    }
    groups
}
