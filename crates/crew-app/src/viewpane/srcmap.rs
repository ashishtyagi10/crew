//! Bytes of the prepared text back to bytes of the file.
//!
//! [`super::whitespace::prepare`] rewrites a file before any rung sees it: a tab becomes the
//! spaces it advances to, a carriage return is dropped, and with the invisibles on a trailing
//! space becomes a `·`. Every one of those changes the byte length of the line. A viewer never
//! cared — it only draws — but the markdown editor's caret IS a byte, read off the rendered
//! cells, and a byte of the *prepared* text spliced into the *file* lands one byte out for every
//! dropped `\r` above it. In a CRLF file that is every line.
//!
//! So `prepare` records each substitution it made, and this maps across them. Between two
//! substitutions the texts are byte-for-byte the same, which is what makes a sorted list of the
//! differences enough: everything else is an offset.
//!
//! A byte INSIDE a substitution — the third space of an expanded tab — has no byte of its own in
//! the file, and maps to `None`. That is the same answer [`crate::md::source`] gives an entity:
//! a place the caret cannot stand, rather than a place one byte out.

/// One place `prepare` wrote something other than what the file has.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Sub {
    /// Where it starts in the prepared text, and how many bytes it is there.
    pub prep: u32,
    pub prep_len: u32,
    /// Where it starts in the file, and how many bytes it was.
    pub orig: u32,
    pub orig_len: u32,
}

/// Every substitution, in order. Empty — the common case — is the identity.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct SrcMap {
    pub subs: Vec<Sub>,
}

impl SrcMap {
    pub(crate) fn is_identity(&self) -> bool {
        self.subs.is_empty()
    }

    /// The file's byte for byte `p` of the prepared text, or `None` inside a substitution.
    pub(crate) fn to_orig(&self, p: u32) -> Option<u32> {
        let i = self.subs.partition_point(|s| s.prep <= p);
        let Some(s) = i.checked_sub(1).map(|i| self.subs[i]) else {
            return Some(p);
        };
        if p < s.prep + s.prep_len {
            return None;
        }
        Some(p - (s.prep + s.prep_len) + s.orig + s.orig_len)
    }

    /// The prepared text's byte for byte `o` of the file. A byte inside a substitution answers
    /// where the substitution starts — the nearest place that exists.
    pub(crate) fn to_prep(&self, o: u32) -> u32 {
        let i = self.subs.partition_point(|s| s.orig <= o);
        let Some(s) = i.checked_sub(1).map(|i| self.subs[i]) else {
            return o;
        };
        if o < s.orig + s.orig_len {
            return s.prep;
        }
        o - (s.orig + s.orig_len) + s.prep + s.prep_len
    }

    /// The code and plain rungs' rows mapped back ([`super::srcstamp`]). Unlike [`Self::remap`],
    /// the FIRST byte of a substitution is the file's byte where it starts: the first space of an
    /// expanded tab is the place before the tab (or a tab-indented line could not be typed at
    /// column 0), and a blank CRLF line's anchor stands before its `\r`, not between it and `\n`.
    pub(crate) fn mapped(
        &self,
        mut lines: Vec<crate::chatbody::CardLine>,
    ) -> Vec<crate::chatbody::CardLine> {
        if self.is_identity() {
            return lines;
        }
        for cell in lines.iter_mut().flatten() {
            cell.src = cell.src.and_then(|p| {
                let i = self.subs.partition_point(|s| s.prep < p);
                match self.subs.get(i) {
                    Some(s) if s.prep == p => Some(s.orig),
                    _ => self.to_orig(p),
                }
            });
        }
        lines
    }

    /// Rewrite every cell's source byte from the prepared text's to the file's.
    pub(crate) fn remap(&self, lines: &mut [crate::chatbody::CardLine]) {
        if self.is_identity() {
            return;
        }
        for cell in lines.iter_mut().flatten() {
            cell.src = cell.src.and_then(|s| self.to_orig(s));
        }
    }
}

#[cfg(test)]
#[path = "srcmap_tests.rs"]
mod tests;
