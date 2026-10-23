//! What may sit around a `@tool` call and still leave it a call: the fence it
//! was wrapped in, the relay's routing line, and a short aside under it.
//!
//! Split out of `parse.rs` once the parser had to say WHERE a call sits as
//! well as what it is. Both questions are about the same wrapping, so the
//! rules for it live in one place: what `ends_the_reply` lets through below a
//! call is exactly what `split_tool_call` cuts off with it.

/// Prose lines a call may be followed by and still be a call.
///
/// A model that calls a tool often says one more thing on its way out. A
/// model EXPLAINING the syntax — answering a question about crew, or quoting
/// a call it made earlier — writes a paragraph after it. Past two short lines
/// the call is an example, and running it would act on something the model
/// was only showing.
const TRAILING_PROSE: usize = 2;

/// Longest line that still counts as an aside rather than an explanation.
const SHORT: usize = 160;

/// Phrases that take a call back. A model that writes a call and then "never
/// mind" has changed its mind; running the tool anyway does what it just
/// said not to.
const RETRACTS: [&str; 6] = [
    "never mind",
    "nevermind",
    "scratch that",
    "disregard",
    "on second thought",
    "ignore that",
];

/// Whether `s` is a call written without its `@tool`: a `server:tool` name,
/// lowercase words either side of the colon, and JSON that opens right after
/// it and closes on the line. Strict, as the least sure shape: a log line
/// (`error[E0308]: …`), a URL or a call cut off mid-JSON never reads as one.
pub(super) fn bare_call(s: &str) -> bool {
    let Some((target, rest)) = s.split_once(' ') else {
        return false;
    };
    let word = |w: &str| {
        w.starts_with(|c: char| c.is_ascii_lowercase())
            && w.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || "_-".contains(c))
    };
    let named = target
        .split_once(':')
        .is_some_and(|(s, t)| word(s) && word(t));
    let json = rest.trim();
    named && json.starts_with('{') && json.ends_with('}')
}

/// Whether what follows a call leaves it the reply's last word: blank lines,
/// the fence it sat in, the relay's routing line, and at most
/// [`TRAILING_PROSE`] short asides that do not take it back.
pub(super) fn ends_the_reply(after: &[&str]) -> bool {
    let mut prose = 0;
    for line in after {
        let t = line.trim();
        if t.is_empty() || is_fence(t) || opens_shell(t) || is_routing(t) {
            continue;
        }
        let lower = t.to_lowercase();
        if RETRACTS.iter().any(|r| lower.contains(r)) {
            return false;
        }
        prose += 1;
        if prose > TRAILING_PROSE || t.chars().count() > SHORT {
            return false;
        }
    }
    true
}

/// The line the fence around a call opens on, when the call sits in one.
///
/// A call is inside a code block when an odd number of fence lines come
/// before it. The opening fence goes with the call only when nothing but
/// blank lines lies between them: text inside the block above a call is
/// something the model wrote, and cutting from the fence would drop it along
/// with the call. Left without its fence, a stripped reply would open a code
/// block that never closes, and everything under it would render as code.
pub(super) fn fence_above(above: &[&str]) -> Option<usize> {
    let fences = above.iter().filter(|l| opens_fence(l.trim())).count();
    if fences % 2 == 0 {
        return None;
    }
    let at = above.iter().rposition(|l| !l.trim().is_empty())?;
    opens_fence(above[at].trim()).then_some(at)
}

/// A fence line of either kind, bare or naming a language: models fence a
/// call as `json` or `bash` as often as they leave it plain.
fn opens_fence(t: &str) -> bool {
    t.starts_with("```") || t.starts_with("~~~")
}

/// A bare code fence: the one a call was wrapped in closing behind it.
fn is_fence(t: &str) -> bool {
    t.len() >= 3 && t.chars().all(|c| c == '`' || c == '~')
}

/// A fence opening a shell block under the calls. Models end a turn of
/// calls with the command they mean to run once the calls are done (`After
/// these edits I will run the tests:` and a ```` ```sh ```` block of `cargo
/// test`), and read as two more lines of prose the fence pushed the reply
/// past [`TRAILING_PROSE`]: on the 2026-10-22 probe both of a relay's turns
/// ended on four edits, unrun. The fence is wrapping; the command inside it
/// still counts as an aside, so a block of several commands, or one in any
/// other language, still makes the call an example. The command itself is
/// not run: the agent sees its calls' results and runs it next round.
fn opens_shell(t: &str) -> bool {
    let lang = t.trim_start_matches(['`', '~']);
    t.len() - lang.len() >= 3
        && matches!(
            lang.trim().to_ascii_lowercase().as_str(),
            "sh" | "bash" | "shell" | "zsh" | "console"
        )
}

/// `@done` / `@next <agent>`, read as tolerantly as the relay's own routing
/// parser reads them. A relay agent ends every reply with one, and a call
/// above it is still the reply's business before routing is.
fn is_routing(t: &str) -> bool {
    let bare = t
        .trim_matches(|c: char| matches!(c, '*' | '`' | '_' | ' ' | '.'))
        .to_ascii_lowercase();
    bare.starts_with("@done") || bare.starts_with("@next")
}

#[cfg(test)]
#[path = "barecall_tests.rs"]
mod barecall_tests;

#[cfg(test)]
#[path = "shellfence_tests.rs"]
mod shellfence_tests;
