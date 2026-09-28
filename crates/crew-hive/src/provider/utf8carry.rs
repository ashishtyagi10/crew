//! Bytes off the wire, decoded only where a character ends.
//!
//! A network read ends wherever the packet did, and that is often inside a
//! character: an em dash is three bytes, a CJK character three, an emoji
//! four. Decoding each read on its own turned the two halves of one such
//! character into replacement marks (`��`) in the pane. Every streamed
//! provider reads through this instead, so the unfinished tail of one read
//! waits for the bytes that complete it.

/// The bytes of a character that has not finished arriving yet.
#[derive(Default)]
pub(crate) struct Utf8Carry {
    tail: Vec<u8>,
}

impl Utf8Carry {
    /// The text `bytes` completes, with any unfinished character at the end
    /// held back for the next read. Bytes that can never be text still come
    /// out as `�` — held back, they would only stall everything after them.
    pub(crate) fn push(&mut self, bytes: &[u8]) -> String {
        self.tail.extend_from_slice(bytes);
        let keep = unfinished(&self.tail);
        let done: Vec<u8> = self.tail.drain(..self.tail.len() - keep).collect();
        String::from_utf8_lossy(&done).into_owned()
    }

    /// What is still held when the stream ends: a character that will never
    /// finish now, so it is shown as `�` rather than dropped without a trace.
    pub(crate) fn finish(self) -> String {
        String::from_utf8_lossy(&self.tail).into_owned()
    }
}

/// How many bytes at the end of `bytes` begin a character they do not
/// complete — 0 when the last character is whole, or is not UTF-8 at all.
fn unfinished(bytes: &[u8]) -> usize {
    // Walk back over continuation bytes (`10xxxxxx`) to the byte that
    // starts the last character; no character is longer than four.
    for back in 1..=bytes.len().min(4) {
        let b = bytes[bytes.len() - back];
        if b & 0xC0 == 0x80 {
            continue;
        }
        let len = match b {
            0xC2..=0xDF => 2,
            0xE0..=0xEF => 3,
            0xF0..=0xF4 => 4,
            _ => 1,
        };
        return if back < len { back } else { 0 };
    }
    0
}

#[cfg(test)]
#[path = "utf8carry_tests.rs"]
mod tests;
