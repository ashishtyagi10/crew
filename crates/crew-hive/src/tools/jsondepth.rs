//! Bracket depth for JSON that arrives a line — or a character — at a time.
//!
//! Its own file because two readers share it: the `@tool` parser, which reads
//! a call's JSON across the lines under it, and the broker's live stream,
//! which has to hide that JSON from the reader until it closes.

/// Where a JSON value closes, fed one char at a time.
///
/// Brackets count only outside strings, and escapes inside them are honoured,
/// so `{"old": "}\""}` closes at its last brace. A state machine rather than
/// a function over a whole string because the stream has to ask while the
/// JSON is still arriving.
#[derive(Debug, Default, Clone, Copy)]
pub struct JsonDepth {
    depth: u32,
    string: bool,
    escaped: bool,
    closed: bool,
}

impl JsonDepth {
    /// Feed one char; `true` once the first bracket fed has been matched.
    pub fn push(&mut self, c: char) -> bool {
        if self.closed {
            return true;
        }
        if self.string {
            if self.escaped {
                self.escaped = false;
            } else if c == '\\' {
                self.escaped = true;
            } else if c == '"' {
                self.string = false;
            }
            return false;
        }
        match c {
            '"' if self.depth > 0 => self.string = true,
            '{' | '[' => self.depth += 1,
            '}' | ']' if self.depth > 0 => {
                self.depth -= 1;
                self.closed = self.depth == 0;
            }
            _ => {}
        }
        self.closed
    }

    /// A bracket has been fed and not matched yet.
    pub fn open(&self) -> bool {
        self.depth > 0
    }

    /// The value has closed; anything fed now is past its end.
    pub fn closed(&self) -> bool {
        self.closed
    }
}
