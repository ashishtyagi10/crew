//! Leading-token pop-ups in the crew composer: a `/` command palette and a
//! leading `@` attach picker (agents, skills, files). Distinct from the
//! mid-line `@file` mention (chatmention): this handles ONLY the leading
//! token, that only non-leading ones, so at most one is open. Pure string
//! logic + popup state.
use crate::chatcomplete::{describe, CONSTRUCTS};
use crate::chatkeys::ChatInput;
use crate::suggest::MenuItem;

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
}

pub(crate) enum PaletteKey {
    Consumed,
    /// The accepted row is a command to RUN: `input` now holds it, and the
    /// caller must submit as if Enter had been pressed on the composer.
    Submit,
    Forward,
}

/// The leading token being typed, if it's a `/command` or `@agent` selector
/// (nothing before it — no whitespace yet). For a multi-target `@a+b`, the
/// query is the segment after the last `+` (matching chatcomplete's Tab).
pub(crate) fn pending_palette(input: &str) -> Option<(Kind, &str)> {
    // `/model <arg>` is the one command with a value picker in the composer:
    // one whitespace-free argument token opens it. A second token means the
    // freeform `/model <agent> <slug>` (or explicit `all`) form — leave it be.
    if let Some(rest) = input.strip_prefix("/model ") {
        let arg = rest.trim_start();
        return (!arg.contains(char::is_whitespace)).then_some((Kind::Model, arg));
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
            })
        }
    }
}

fn slash_items(query: &str) -> Vec<MenuItem> {
    CONSTRUCTS
        .iter()
        .filter(|c| c[1..].starts_with(query))
        .map(|c| MenuItem {
            label: c.to_string(),
            desc: describe(c).to_string(),
            fill: c.to_string(),
            submit: false,
            header: false,
            dim: false,
        })
        .collect()
}

/// Rows for the leading `@`: the full attach picker (agents, skills, files
/// — `chatmention::filter`'s section order), agents only once the token has
/// a `+` (multi-target selectors route, they don't attach).
fn attach_items(
    query: &str,
    entries: &[crate::chatmention::MentionEntry],
    multi: bool,
) -> Vec<MenuItem> {
    use crate::chatmention::MentionEntry;
    crate::chatmention::filter(entries, query)
        .into_iter()
        .filter(|e| !multi || matches!(e, MentionEntry::Agent { .. }))
        .map(|e| MenuItem {
            label: format!("@{}", e.token()),
            desc: e.desc(),
            fill: e.token(),
            submit: false,
            header: false,
            dim: false,
        })
        .collect()
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
        ChatInput::Up => p.sel = crate::suggest::step_sel(&p.items, p.sel, false),
        ChatInput::Down => p.sel = crate::suggest::step_sel(&p.items, p.sel, true),
        ChatInput::Complete | ChatInput::Enter => {
            let mut submit = false;
            if let Some(item) = p.items.get(p.sel) {
                submit = item.submit && matches!(key, ChatInput::Enter);
                *input = accept(input, p.kind, &item.fill);
            }
            *palette = None;
            if submit {
                return PaletteKey::Submit;
            }
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
