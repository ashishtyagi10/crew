//! A long output cut from the middle, so its conclusion survives the cut.
//!
//! A worker writes its findings first and its verdict last: "…so the fix is
//! in route.rs:412", "All 159 tests passed", a long file listing and then the
//! answer. Both readers of a worker's output, the task that depends on it
//! (`apiagent::context`) and the broker's closing answer and judge (crew-plugin
//! `swarmanswer`), used to keep only its head, which kept the preamble and cut
//! exactly the line the reader needed; the closing answer then missed the
//! conclusion or guessed it. The head says what an output is about and the
//! tail says what it found, so the middle is the part a reader can lose.

#[cfg(test)]
#[path = "clipmiddle_tests.rs"]
mod tests;

/// The head gets one part in this many of the budget and the tail the rest:
/// a third is enough to say what the output is about, and the tail is where
/// the verdict, the summary and the answer after a long listing are.
const HEAD_SHARE: usize = 3;

/// `s` held to `max` chars by cutting out its middle: about the first third
/// of the budget from the head and the rest from the tail, with one line
/// between them saying how much went (`… [N chars cut from the middle] …`).
///
/// Each side ends on a line boundary when one is within reach, since half a
/// line reads as a thought the worker never finished; a side with no line end
/// near its cut (one huge line) is cut at a char instead. Chars, never bytes,
/// so multibyte text is never split, and under budget `s` passes through
/// byte-identical. The marker line rides on top of `max`, as a clip marker
/// always has.
pub fn clip_middle(s: &str, max: usize) -> String {
    let total = s.chars().count();
    if total <= max {
        return s.to_owned();
    }
    let head_max = max / HEAD_SHARE;
    let head = &s[..head_end(s, head_max)];
    let tail = &s[tail_start(s, total, max - head_max)..];
    let cut = total - head.chars().count() - tail.chars().count();
    let marker = format!("\u{2026} [{cut} chars cut from the middle] \u{2026}");
    let mut out = String::with_capacity(head.len() + marker.len() + tail.len() + 2);
    if !head.is_empty() {
        out.push_str(head.strip_suffix('\n').unwrap_or(head));
        out.push('\n');
    }
    out.push_str(&marker);
    if !tail.is_empty() {
        out.push('\n');
        out.push_str(tail);
    }
    out
}

/// Where the head stops, in bytes: after the last whole line that fits in
/// `max` chars, or at `max` chars when that line end is out of reach.
fn head_end(s: &str, max: usize) -> usize {
    let (mut kept, mut end) = (0, 0);
    for line in s.split_inclusive('\n') {
        let n = line.chars().count();
        if kept + n > max {
            break;
        }
        kept += n;
        end += line.len();
    }
    if within_reach(kept, max) {
        end
    } else {
        byte_at(s, max)
    }
}

/// Where the tail starts, in bytes: at the first of the last whole lines that
/// fit in `max` chars, or `max` chars from the end when that is out of reach.
fn tail_start(s: &str, total: usize, max: usize) -> usize {
    let (mut kept, mut start) = (0, s.len());
    for line in s.split_inclusive('\n').rev() {
        let n = line.chars().count();
        if kept + n > max {
            break;
        }
        kept += n;
        start -= line.len();
    }
    if within_reach(kept, max) {
        start
    } else {
        byte_at(s, total - max)
    }
}

/// A line end is within reach when cutting there still keeps at least half of
/// a side's share. Further back, keeping whole lines would throw away more of
/// the output than the one half-line a char cut costs.
fn within_reach(kept: usize, max: usize) -> bool {
    kept * 2 >= max
}

/// The byte offset of char `n` in `s`, or its end when it is shorter.
fn byte_at(s: &str, n: usize) -> usize {
    s.char_indices().nth(n).map_or(s.len(), |(i, _)| i)
}
