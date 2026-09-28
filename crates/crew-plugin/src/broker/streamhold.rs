//! Control lines never reach the live card.
//!
//! The relay's agents speak a line protocol — `@done`, `@next <agent>`,
//! `@tool <server>:<tool> {…}` — on lines of their own, and the stream carried
//! the model's text raw. Measured live (2026-09-27): the smith pane typed out
//! `@tool sys:read_file {"path": "README.md", "offset": 0}` mid-answer, a
//! gated JSON tail ` 0}` then headed the next round's text, and every answer
//! ended on a dangling `@` until the settled card replaced it.
//!
//! [`Hold`] sits in front of the text gate: a line that STARTS with `@`
//! (after indentation or markdown emphasis) is held until it ends; a control
//! line is dropped, anything else — `@editor, over to you` — is released
//! whole. Nothing else is delayed. A line still held when the hop ends is
//! never shown; the settled reply, already stripped, arrives right after.

/// Streaming filter for one hop's text (spanning its tool rounds).
#[derive(Default)]
pub(crate) struct Hold {
    /// The held line, from its first character.
    line: String,
    /// A line starting with `@` is being held.
    held: bool,
    /// Indentation/emphasis seen at the start of a line, not yet released.
    lead: String,
    /// The next character begins a line.
    mid: bool,
}

impl Hold {
    /// Feed one fragment; returns the part that is safe to show now.
    pub(crate) fn feed(&mut self, frag: &str) -> String {
        let mut out = String::new();
        for c in frag.chars() {
            if self.held {
                if c == '\n' {
                    if !is_control(&self.line) {
                        out.push_str(&self.line);
                        out.push('\n');
                    }
                    self.line.clear();
                    self.held = false;
                    self.mid = false;
                } else {
                    self.line.push(c);
                }
                continue;
            }
            if !self.mid {
                match c {
                    '@' => {
                        self.held = true;
                        self.line = std::mem::take(&mut self.lead);
                        self.line.push(c);
                        continue;
                    }
                    ' ' | '\t' | '`' | '*' | '_' => {
                        self.lead.push(c);
                        continue;
                    }
                    '\n' => {
                        out.push_str(&std::mem::take(&mut self.lead));
                        out.push(c);
                        continue;
                    }
                    _ => {
                        out.push_str(&std::mem::take(&mut self.lead));
                        self.mid = true;
                    }
                }
            }
            out.push(c);
            if c == '\n' {
                self.mid = false;
            }
        }
        out
    }
}

/// `@done`, `@next …` or `@tool …`, however wrapped — the directives the
/// engine parses, which are the relay's business and not the reader's.
fn is_control(line: &str) -> bool {
    let bare = line
        .trim()
        .trim_matches(|c: char| matches!(c, '`' | '*' | '_'))
        .trim_start()
        .to_ascii_lowercase();
    ["@done", "@next", "@tool"].iter().any(|d| {
        bare.strip_prefix(d)
            .is_some_and(|rest| rest.is_empty() || rest.starts_with(char::is_whitespace))
    })
}

#[cfg(test)]
#[path = "streamhold_tests.rs"]
mod tests;
