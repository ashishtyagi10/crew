//! The combining marks a cell carries after its character: a virama, a nasal
//! dot, a vowel sign drawn over or under its letter, an accent typed as its
//! own code point. Width 0 in unicode-width, so they never take a column of
//! their own, and a cell that dropped them turned `क्ष` into `कष`.
//!
//! Lives here, beside [`crate::deco`], because both ends need it: the terminal
//! model reads them off alacritty's grid and every in-app widget off its text,
//! and the renderer shapes them with the character before them.

/// Up to [`Marks::CAP`] marks, inline. No allocation, and empty — `'\0'` in
/// every slot — for almost every cell there is. Marks past the cap are
/// dropped: four holds every stack Indic or Thai text puts on one letter.
#[derive(Hash, Clone, Copy, PartialEq, Eq, Default, Debug)]
pub struct Marks([char; Marks::CAP]);

impl Marks {
    pub const CAP: usize = 4;

    /// Add `c` after the marks already held; `false` when full or `c` is NUL.
    pub fn push(&mut self, c: char) -> bool {
        match self.0.iter_mut().find(|m| **m == '\0') {
            Some(slot) if c != '\0' => {
                *slot = c;
                true
            }
            _ => false,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.0[0] == '\0'
    }

    /// The marks, in the order they were typed.
    pub fn chars(&self) -> impl Iterator<Item = char> + '_ {
        self.0.iter().copied().take_while(|&c| c != '\0')
    }

    /// The marks of `s`, in order — what a widget hands a cell when it
    /// places a character and the run of width-0 code points after it.
    pub fn of(s: impl IntoIterator<Item = char>) -> Self {
        let mut m = Self::default();
        for c in s {
            m.push(c);
        }
        m
    }
}

#[cfg(test)]
#[path = "marks_tests.rs"]
mod tests;
