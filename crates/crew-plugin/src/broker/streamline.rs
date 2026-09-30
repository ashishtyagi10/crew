//! What a held line of the live stream is: a directive the relay parses, a
//! code fence, or the start of a model's own `<tool_call>` block.
//!
//! Split from `streamhold` when the tagged block joined the list and put that
//! file over the line cap.

use crew_hive::tools::JsonDepth;

/// `@done`, `@next …` or `@tool …`, however wrapped — the directives the
/// engine parses, which are the relay's business and not the reader's.
pub(super) fn control(line: &str) -> Option<&'static str> {
    let bare = line
        .trim()
        .trim_matches(|c: char| matches!(c, '`' | '*' | '_'))
        .trim_start()
        .to_ascii_lowercase();
    ["@done", "@next", "@tool"].into_iter().find(|d| {
        bare.strip_prefix(d)
            .is_some_and(|rest| rest.is_empty() || rest.starts_with(char::is_whitespace))
    })
}

/// A code fence line: backticks and at most a language tag.
pub(super) fn is_fence(line: &str) -> bool {
    let t = line.trim();
    t.starts_with("```") && !t.trim_start_matches('`').contains(['`', ' ', '\t'])
}

/// Where a dropped call's JSON stands at the end of its `@tool` line: `None`
/// once it closed there, unfed when the line has none — it may open under it.
pub(super) fn open_args(line: &str) -> Option<JsonDepth> {
    let mut args = JsonDepth::default();
    let Some(i) = line.find(['{', '[']) else {
        return Some(args);
    };
    (!line[i..].chars().any(|c| args.push(c))).then_some(args)
}

const OPEN: &str = "<tool_call>";
const CLOSE: &str = "</tool_call>";

/// A `<tool_call>` block being kept off the card.
///
/// Qwen 3.x writes a call in its trained shape, `<tool_call>{…}</tool_call>`,
/// and the parser runs it (`crew_hive::tools`), but the stream typed it out
/// first: the block's JSON sat on the live card until the settled reply
/// replaced it. The block goes the way a `@tool` line goes, its JSON lines
/// with it. The closing tag usually never arrives — generation stops there
/// (`stop`) — and then the rest of the hop is dropped, which is nothing.
#[derive(Default)]
pub(super) struct Tagged {
    on: bool,
    line: String,
}

impl Tagged {
    /// Whether held `line` opens a block, and if it does not close on the
    /// same line, start swallowing what follows.
    pub(super) fn opens(&mut self, line: &str) -> bool {
        let t = line.trim_start_matches(|c: char| c.is_whitespace() || "`*_".contains(c));
        if !t.starts_with(OPEN) {
            return false;
        }
        self.on = !t.contains(CLOSE);
        true
    }

    /// Whether `c` belongs to an open block; the line it closes on goes too.
    pub(super) fn swallow(&mut self, c: char) -> bool {
        if !self.on {
            return false;
        }
        if c == '\n' {
            self.on = !self.line.contains(CLOSE);
            self.line.clear();
        } else {
            self.line.push(c);
        }
        true
    }
}
