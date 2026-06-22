//! Environment-variable expansion for `cd` arguments typed in the input bar:
//! `$NAME` and `${NAME}` are replaced with their values (unset → empty), so
//! `cd $HOME/src` and `cd ${PROJECT}` work like they do in a shell.

/// Expand every `$NAME` / `${NAME}` reference in `s` using the process
/// environment. A bare `$`, or `$` followed by a non-name character, is literal.
pub(crate) fn expand_env(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'$' {
            if let Some((name, next)) = read_var(s, i) {
                out.push_str(&std::env::var(name).unwrap_or_default());
                i = next;
                continue;
            }
        }
        let ch = s[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

/// Parse a `$NAME` / `${NAME}` token whose `$` is at `start`; returns the name
/// and the byte index just past the token, or `None` if it isn't a variable.
/// Names start with a letter or `_` (so `$5` stays literal).
fn read_var(s: &str, start: usize) -> Option<(&str, usize)> {
    let b = s.as_bytes();
    if b.get(start + 1) == Some(&b'{') {
        let close = s[start + 2..].find('}')?;
        let name = &s[start + 2..start + 2 + close];
        return (!name.is_empty()).then_some((name, start + 2 + close + 1));
    }
    let first = *b.get(start + 1)?;
    if !(first.is_ascii_alphabetic() || first == b'_') {
        return None;
    }
    let mut j = start + 2;
    while j < b.len() && (b[j] == b'_' || b[j].is_ascii_alphanumeric()) {
        j += 1;
    }
    Some((&s[start + 1..j], j))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_simple_and_braced() {
        std::env::set_var("CREW_EE_A", "/tmp/a");
        assert_eq!(expand_env("$CREW_EE_A/x"), "/tmp/a/x");
        assert_eq!(expand_env("${CREW_EE_A}y"), "/tmp/ay");
        assert_eq!(expand_env("pre $CREW_EE_A post"), "pre /tmp/a post");
    }

    #[test]
    fn unset_is_empty_and_literals_kept() {
        std::env::remove_var("CREW_EE_UNSET");
        assert_eq!(expand_env("a${CREW_EE_UNSET}b"), "ab");
        assert_eq!(expand_env("no vars here"), "no vars here");
        assert_eq!(expand_env("$"), "$");
        assert_eq!(expand_env("~/plain"), "~/plain");
        // `$5` is not a variable (names can't start with a digit).
        assert_eq!(expand_env("cost $5"), "cost $5");
    }
}
