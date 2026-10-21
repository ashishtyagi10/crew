//! The `to` and `cc` lines of the reply box, as addresses: split where a
//! comma (or a semicolon) is not inside a quoted name or `<…>`, each one
//! checked before anything is sent, and written back as one header.
use base64::Engine;

/// The addresses in a line someone typed, or a header: trimmed, the empty
/// ones dropped. `"Lee, Ann" <ann@x.org>, bo@y.org` is two.
pub(crate) fn split(line: &str) -> Vec<String> {
    let (mut out, mut one) = (Vec::new(), String::new());
    let (mut quoted, mut angle) = (false, false);
    for c in line.chars() {
        match c {
            '"' => quoted = !quoted,
            '<' if !quoted => angle = true,
            '>' if !quoted => angle = false,
            ',' | ';' if !quoted && !angle => {
                out.push(std::mem::take(&mut one));
                continue;
            }
            _ => {}
        }
        one.push(c);
    }
    out.push(one);
    out.into_iter()
        .map(|a| a.trim().to_string())
        .filter(|a| !a.is_empty())
        .collect()
}

/// The address in `Name <a@b>`, or the whole of a bare one.
pub(crate) fn address(one: &str) -> &str {
    match (one.rfind('<'), one.rfind('>')) {
        (Some(l), Some(r)) if l < r => one[l + 1..r].trim(),
        _ => one.trim(),
    }
}

/// Whether `one` names a mailbox Gmail can send to: something, `@`, and a
/// domain with a dot, no spaces.
pub(crate) fn valid(one: &str) -> bool {
    let a = address(one);
    let Some((local, domain)) = a.split_once('@') else {
        return false;
    };
    !local.is_empty()
        && !a.chars().any(char::is_whitespace)
        && !domain.contains('@')
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
}

/// The first address in `line` that is not one, to name in the note.
pub(crate) fn first_bad(line: &str) -> Option<String> {
    split(line).into_iter().find(|a| !valid(a))
}

/// `addrs` without `me`, and each mailbox once, in order.
pub(crate) fn without(addrs: &[String], me: &[&str]) -> Vec<String> {
    let mut seen: Vec<String> = me.iter().map(|m| address(m).to_lowercase()).collect();
    let mut out = Vec::new();
    for a in addrs {
        let key = address(a).to_lowercase();
        if !key.is_empty() && !seen.contains(&key) {
            seen.push(key);
            out.push(a.clone());
        }
    }
    out
}

/// The addresses as a header's value: joined with `, `, a display name
/// outside ASCII RFC 2047-encoded so the header stays ASCII.
pub(crate) fn header(line: &str) -> String {
    split(line)
        .iter()
        .map(|a| match (a.is_ascii(), a.rfind('<')) {
            (true, _) => a.clone(),
            (false, Some(l)) => {
                let name = a[..l].trim().trim_matches('"');
                let b = base64::engine::general_purpose::STANDARD.encode(name);
                format!("=?UTF-8?B?{b}?= {}", &a[l..])
            }
            (false, None) => a.clone(),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Who a line goes to, for a note: `Ann Lee`, `Ann Lee and Bo`, `Ann Lee
/// and 2 others`.
pub(crate) fn names(line: &str) -> String {
    let all = split(line);
    let name = |a: &String| super::wire::sender_name(a);
    match all.as_slice() {
        [] => String::new(),
        [one] => name(one),
        [one, two] => format!("{} and {}", name(one), name(two)),
        [one, rest @ ..] => format!("{} and {} others", name(one), rest.len()),
    }
}

#[cfg(test)]
#[path = "addr_tests.rs"]
mod tests;
