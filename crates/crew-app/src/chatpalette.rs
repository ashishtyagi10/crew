//! Leading-token pop-ups in the crew composer: a `/` command palette and a
//! leading `@` attach picker (agents, skills, files). Distinct from the
//! mid-line `@file` mention (chatmention): this handles ONLY the leading
//! token, that only non-leading ones, so at most one is open. Pure string
//! logic + popup state.
use crate::chatkeys::ChatInput;
use crate::suggest::MenuItem;

#[path = "chatpaletteitems.rs"]
mod chatpaletteitems;
use chatpaletteitems::{attach_items, slash_items};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Kind {
    Slash,
    Agent,
    /// The `/model ` argument phase — the grouped model picker.
    Model,
}

/// The open leading-token palette: already-filtered rows + selection, plus
/// the scanned entries (agents/skills/files) so narrowing the query doesn't
/// rescan — like `chatmention::MentionState`.
pub(crate) struct PaletteState {
    pub kind: Kind,
    pub items: Vec<MenuItem>,
    pub sel: usize,
    pub entries: Vec<crate::chatmention::MentionEntry>,
    /// Whether Up/Down has moved the selection since the palette opened.
    /// Only meaningful for `Kind::Model` — see `popup_key`'s Enter arm.
    pub touched: bool,
}

pub(crate) enum PaletteKey {
    Consumed,
    /// A row was accepted without running: `input` now holds the filled text
    /// and the palette is closed. The caller MUST re-sync it against the new
    /// input (`after_edit`) — accepting `/model` fills `/model `, which is
    /// itself a palette token, and the model picker only opens if something
    /// re-runs the sync. Without this the picker never appeared for the most
    /// natural flow of all: type `/mod`, press Enter on the row.
    Accepted,
    /// The accepted row is a command to RUN: `input` now holds it, and the
    /// caller must submit as if Enter had been pressed on the composer.
    Submit,
    Forward,
    /// The accepted row can't run until a provider key exists; the payload is
    /// the variable it needs. The palette is closed and `input` is UNCHANGED —
    /// the model is not chosen until it can actually be served.
    NeedsKey(String),
}

/// The leading token being typed, if it's a `/command` or `@agent` selector
/// (nothing before it — no whitespace yet). For a multi-target `@a+b`, the
/// query is the segment after the last `+` (matching chatcomplete's Tab).
pub(crate) fn pending_palette(input: &str) -> Option<(Kind, &str)> {
    // `/model <arg>` (and its broker alias `/m`, see `expand_alias` in
    // commands.rs) is the one command with a value picker in the composer:
    // one whitespace-free argument token opens it. A second token means the
    // freeform `/model <agent> <slug>` (or explicit `all`) form — leave it be.
    for prefix in ["/model ", "/m "] {
        if let Some(rest) = input.strip_prefix(prefix) {
            let arg = rest.trim_start();
            // `/model 2` is the broker's numbered provider pick — it can
            // START A SIGN-IN, and the listing that taught the user the
            // number came from the broker. Opening the catalog popup here
            // hijacked Enter into accepting some catalog row (or its key
            // prompt), which made the advertised OAuth path unreachable.
            if !arg.is_empty() && arg.chars().all(|c| c.is_ascii_digit()) {
                return None;
            }
            return (!arg.contains(char::is_whitespace)).then_some((Kind::Model, arg));
        }
    }
    if input.contains(char::is_whitespace) {
        return None;
    }
    if let Some(rest) = input.strip_prefix('/') {
        return Some((Kind::Slash, rest));
    }
    if let Some(rest) = input.strip_prefix('@') {
        return Some((Kind::Agent, rest.rsplit('+').next().unwrap_or(rest)));
    }
    None
}

/// Sync the palette to the input after an edit: open on a leading `/`/`@`
/// token, refilter as it narrows, close when it ends or nothing matches.
pub(crate) fn after_edit(
    palette: &mut Option<PaletteState>,
    input: &str,
    current_model: Option<&str>,
    scan: impl FnOnce() -> Vec<crate::chatmention::MentionEntry>,
) {
    let Some((kind, query)) = pending_palette(input) else {
        *palette = None;
        return;
    };
    // Reuse the open palette's scan; (re)scan when opening or kind changed.
    let entries = match palette {
        Some(p) if p.kind == kind => std::mem::take(&mut p.entries),
        _ => match kind {
            Kind::Agent => scan(),
            Kind::Slash | Kind::Model => Vec::new(),
        },
    };
    let items = match kind {
        Kind::Slash => slash_items(query),
        Kind::Agent => attach_items(query, &entries, input.contains('+')),
        Kind::Model => crate::modelpick::rows(query, current_model),
    };
    if items.is_empty() {
        *palette = None;
        return;
    }
    match palette {
        Some(p) if p.kind == kind => {
            p.sel = p.sel.min(items.len() - 1);
            if items[p.sel].header {
                p.sel = crate::suggest::first_selectable(&items);
            }
            p.items = items;
            p.entries = entries;
        }
        _ => {
            let sel = crate::suggest::first_selectable(&items);
            *palette = Some(PaletteState {
                kind,
                items,
                sel,
                entries,
                touched: false,
            })
        }
    }
}

/// Popup-first key routing: arrows move, Tab/Enter accept, Esc closes the
/// popup (not the pane).
pub(crate) fn popup_key(
    palette: &mut Option<PaletteState>,
    input: &mut String,
    key: &ChatInput,
) -> PaletteKey {
    let Some(p) = palette else {
        return PaletteKey::Forward;
    };
    match key {
        ChatInput::Up => {
            p.sel = crate::suggest::step_sel(&p.items, p.sel, false);
            p.touched = true;
        }
        ChatInput::Down => {
            p.sel = crate::suggest::step_sel(&p.items, p.sel, true);
            p.touched = true;
        }
        ChatInput::Complete | ChatInput::Enter => {
            // Bare `/model `/`/m ` + untouched Enter: submit the input as-is
            // (the broker's read-only listing) instead of accepting the
            // highlighted `default` row's destructive `/model all default`.
            if p.kind == Kind::Model
                && !p.touched
                && matches!(key, ChatInput::Enter)
                && matches!(pending_palette(input), Some((Kind::Model, q)) if q.trim().is_empty())
            {
                *palette = None;
                return PaletteKey::Submit;
            }
            let mut submit = false;
            if let Some(item) = p.items.get(p.sel) {
                if let Some(var) = item.needs.clone() {
                    *palette = None;
                    return PaletteKey::NeedsKey(var);
                }
                submit = item.submit && matches!(key, ChatInput::Enter);
                *input = accept(input, p.kind, &item.fill);
            }
            *palette = None;
            return if submit {
                PaletteKey::Submit
            } else {
                PaletteKey::Accepted
            };
        }
        ChatInput::Close => *palette = None,
        _ => return PaletteKey::Forward,
    }
    PaletteKey::Consumed
}

/// Replace the leading token's active segment with `fill`: a slash construct
/// becomes `/cmd `; an agent becomes `@name `, preserving any `@a+` prefix.
pub(crate) fn accept(input: &str, kind: Kind, fill: &str) -> String {
    match kind {
        Kind::Slash => format!("{fill} "),
        // The broker reads `/model <agent> <slug>`; the picker applies the
        // pick to the whole roster, so it must send the `all` target.
        Kind::Model => format!("/model all {fill}"),
        Kind::Agent => match input.rfind('+') {
            Some(plus) => format!("{}{fill} ", &input[..=plus]),
            None => format!("@{fill} "),
        },
    }
}

/// The model every agent runs, or `None` when the roster disagrees (mixed
/// pins) or reports nothing — only an unambiguous answer earns the `●` mark.
pub(crate) fn shared_model(agents: &[crew_plugin::AgentInfo]) -> Option<String> {
    let first = agents.iter().find(|a| !a.model.is_empty())?;
    agents
        .iter()
        .all(|a| a.model.is_empty() || a.model == first.model)
        .then(|| first.model.clone())
}

#[cfg(test)]
#[path = "chatpalette_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "chatpalettemodel_tests.rs"]
mod model_tests;
