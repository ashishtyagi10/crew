//! `sys:edit`'s arguments, read into the swaps `sysedit` makes. Moved out of
//! `systools` (over the line cap) when the edit learned `line`.
use super::sysedit::{edit_all, Swap};
use super::systools::str_arg;
use serde_json::Value;

/// `sys:edit`, in either shape: one `old`/`new` pair, or an `edits` array of
/// them. Both end in the same all-or-nothing call, so the batch is not a
/// second code path that can drift from the single edit's contract.
pub(super) fn edit(v: &Value) -> Result<String, String> {
    let path = str_arg(v, "path")?;
    let Some(edits) = v.get("edits").and_then(|e| e.as_array()) else {
        let (old, new) = (str_arg(v, "old")?, str_arg(v, "new")?);
        let swap = Swap {
            old,
            new,
            all: all(v),
            line: line(v),
        };
        return edit_all(path, &[swap]);
    };
    // `line` or `all` beside the list, not in an edit (a probe sent
    // `{"edits": [{…}], "line": 15}`): one edit's own, or no one's.
    let one = edits.len() == 1;
    if !one && (line(v).is_some() || all(v)) {
        return Err("edit: `line` and `all` go inside the edit they are for: \
                    {\"edits\": [{\"old\": \u{2026}, \"new\": \u{2026}, \"line\": N}, \u{2026}]}"
            .into());
    }
    let swaps: Result<Vec<_>, String> = edits
        .iter()
        .enumerate()
        .map(|(i, e)| {
            let at = |k: &str| {
                e.get(k)
                    .and_then(|x| x.as_str())
                    .ok_or_else(|| format!("edits[{i}] has no string \u{201c}{k}\u{201d}"))
            };
            Ok(Swap {
                old: at("old")?,
                new: at("new")?,
                all: all(e) || (one && all(v)),
                line: line(e).or(line(v).filter(|_| one)),
            })
        })
        .collect();
    edit_all(path, &swaps?)
}

fn all(e: &Value) -> bool {
    e.get("all").and_then(|a| a.as_bool()) == Some(true)
}

/// `"line": 131`, or `"131"` as a model sometimes quotes it.
fn line(e: &Value) -> Option<usize> {
    let v = e.get("line")?;
    let n = v.as_u64().or_else(|| v.as_str()?.trim().parse().ok())?;
    usize::try_from(n).ok()
}

#[cfg(test)]
#[path = "editargs_tests.rs"]
mod tests;
