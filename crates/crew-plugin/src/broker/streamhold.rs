//! Control lines never reach the live card.
//!
//! The relay's agents speak a line protocol — `@done`, `@next <agent>`,
//! `@tool <server>:<tool> {…}` — on lines of their own, and the stream carried
//! the model's text raw. Measured live (2026-09-27): the smith pane typed out
//! `@tool sys:read_file {"path": "README.md", "offset": 0}` mid-answer, a
//! gated JSON tail ` 0}` then headed the next round's text, and every answer
//! ended on a dangling `@` until the settled card replaced it.
//!
//! [`Hold`] sits in front of the text gate: a line that STARTS with `@` or
//! `<` (after indentation or markdown emphasis) is held until it ends; a
//! control line or a `<tool_call>` block (`streamline::Tagged`) is dropped,
//! anything else — `@editor, over to you`, `<div>` — is released
//! whole. Nothing else is delayed. A line still held when the hop ends is
//! never shown; the settled reply, already stripped, arrives right after.
//!
//! A call does not always fit its line. Pretty-printed JSON ran on under a
//! dropped `@tool` line and typed itself out as `"path": "a.rs",` and a lone
//! `}`; a call fenced in a code block left an empty block on the card. So a
//! dropped call's JSON is swallowed until it closes, and a fence line is held
//! until the next line says whether it wraps a directive — if it does, the
//! fence and its closer go with it. A fence wrapping anything else is shown as
//! written, one line late.

#[path = "streamline.rs"]
mod streamline;
use crew_hive::tools::JsonDepth;
use streamline::{control, is_fence, open_args, Tagged};

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
    /// An opening fence, held back until the next line shows what it wraps.
    fence: String,
    /// A fence around a dropped directive was dropped; its closer goes too.
    closer: bool,
    /// Inside a code block being shown, so the next fence closes it.
    block: bool,
    /// A dropped call's JSON that has not closed yet (or, with nothing fed,
    /// may still open on the line under the call).
    args: Option<JsonDepth>,
    /// A `<tool_call>` block that has not closed yet.
    tag: Tagged,
}

impl Hold {
    /// Feed one fragment; returns the part that is safe to show now.
    pub(crate) fn feed(&mut self, frag: &str) -> String {
        let mut out = String::new();
        for c in frag.chars() {
            if self.swallow(c) || self.tag.swallow(c) {
                continue;
            }
            if self.held {
                if c == '\n' {
                    let line = std::mem::take(&mut self.line);
                    self.held = false;
                    self.mid = false;
                    self.settle(line, &mut out);
                } else {
                    self.line.push(c);
                }
                continue;
            }
            if !self.mid {
                match c {
                    '@' | '<' => {
                        self.held = true;
                        self.line = std::mem::take(&mut self.lead);
                        self.line.push(c);
                        continue;
                    }
                    ' ' | '\t' | '`' | '*' | '_' => {
                        self.lead.push(c);
                        if self.lead.trim_start() == "```" {
                            self.held = true;
                            self.line = std::mem::take(&mut self.lead);
                        }
                        continue;
                    }
                    '\n' => {
                        self.release(&mut out);
                        out.push_str(&std::mem::take(&mut self.lead));
                        out.push(c);
                        continue;
                    }
                    _ => {
                        self.closer = false;
                        self.release(&mut out);
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

    /// A held line is complete: drop it, hold it back as a fence, or show it.
    fn settle(&mut self, line: String, out: &mut String) {
        let word = control(&line);
        if word.is_some() || self.tag.opens(&line) {
            self.closer |= !std::mem::take(&mut self.fence).is_empty();
            if word == Some("@tool") {
                self.args = open_args(&line);
            }
            return;
        }
        if is_fence(&line) {
            if std::mem::take(&mut self.closer) {
                return;
            }
            self.release(out);
            if !self.block {
                self.fence = line + "\n";
                return;
            }
            self.block = false;
        } else {
            self.closer = false;
            self.release(out);
        }
        out.push_str(&line);
        out.push('\n');
    }

    /// Show the fence held back: the line after it was not a directive.
    fn release(&mut self, out: &mut String) {
        if !self.fence.is_empty() {
            out.push_str(&std::mem::take(&mut self.fence));
            self.block = true;
        }
    }

    /// Whether `c` belongs to a dropped call's JSON: the value, the rest of
    /// the line it closes on, or the indent before a `{` under the call.
    fn swallow(&mut self, c: char) -> bool {
        let Some(args) = self.args.as_mut() else {
            return false;
        };
        if args.closed() {
            if c == '\n' {
                self.args = None;
                self.mid = false;
            }
            return true;
        }
        if args.open() || c == '{' {
            args.push(c);
            return true;
        }
        if c == ' ' || c == '\t' {
            return true;
        }
        self.args = None;
        false
    }
}

#[cfg(test)]
#[path = "streamhold_tests.rs"]
mod tests;
