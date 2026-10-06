//! A count put into words, as the app's `wording::count` does.
//!
//! The broker's replies said `1 task(s) stopped` and `3 round(s) complete`:
//! a form that tells the reader nobody looked at the number. Text a model
//! reads (the tool-round budget, a clipped thread) may keep it; text a
//! person reads picks the ending from the count.

/// `n` and its noun: `1 task`, `3 tasks`.
pub(crate) fn count(n: usize, one: &str) -> String {
    format!("{n} {one}{}", if n == 1 { "" } else { "s" })
}

#[cfg(test)]
mod tests {
    #[test]
    fn one_is_singular_and_everything_else_is_not() {
        assert_eq!(super::count(1, "task"), "1 task");
        assert_eq!(super::count(0, "task"), "0 tasks");
        assert_eq!(super::count(3, "failed task"), "3 failed tasks");
    }
}
