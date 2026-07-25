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
mod tests {
    use super::*;
    use crate::chatkeys::ChatInput;
    use crew_plugin::AgentInfo;

    fn agents() -> Vec<AgentInfo> {
        // AgentInfo does NOT derive Default — construct all three fields.
        ["planner", "coder"]
            .iter()
            .map(|n| AgentInfo {
                name: n.to_string(),
                role: "role".into(),
                model: String::new(),
            })
            .collect()
    }

    #[test]
    fn pending_palette_detects_leading_slash_and_agent() {
        assert_eq!(pending_palette("/mod"), Some((Kind::Slash, "mod")));
        assert_eq!(pending_palette("@co"), Some((Kind::Agent, "co")));
        assert_eq!(pending_palette("@a+co"), Some((Kind::Agent, "co"))); // segment after '+'
        assert_eq!(pending_palette("@planner"), Some((Kind::Agent, "planner")));
        assert_eq!(pending_palette("hey @co"), None); // non-leading → file mention's job
        assert_eq!(pending_palette("/theme x"), None); // token ended (no arg picker)
        assert_eq!(pending_palette("plain"), None);
        assert_eq!(pending_palette(""), None);
    }

    #[test]
    fn accept_replaces_leading_token_preserving_multi_target() {
        assert_eq!(accept("/mod", Kind::Slash, "/model"), "/model ");
        assert_eq!(accept("@co", Kind::Agent, "coder"), "@coder ");
        assert_eq!(accept("@a+co", Kind::Agent, "coder"), "@a+coder ");
    }

    fn agent_entries() -> Vec<crate::chatmention::MentionEntry> {
        agents()
            .iter()
            .map(|a| crate::chatmention::MentionEntry::Agent {
                name: a.name.clone(),
                role: a.role.clone(),
            })
            .collect()
    }

    #[test]
    fn after_edit_opens_refilters_and_closes() {
        let mut p = None;
        after_edit(&mut p, "@", None, agent_entries);
        assert_eq!(p.as_ref().unwrap().items.len(), 2);
        assert_eq!(p.as_ref().unwrap().kind, Kind::Agent);
        after_edit(&mut p, "@co", None, agent_entries);
        assert_eq!(p.as_ref().unwrap().items.len(), 1); // only coder
        after_edit(&mut p, "@zzz", None, agent_entries);
        assert!(p.is_none()); // no match closes
        after_edit(&mut p, "/mo", None, Vec::new);
        assert_eq!(p.as_ref().unwrap().kind, Kind::Slash);
        assert!(p.as_ref().unwrap().items.iter().any(|i| i.fill == "/model"));
        after_edit(&mut p, "hey", None, agent_entries);
        assert!(p.is_none()); // no leading selector
    }

    #[test]
    fn leading_at_offers_agents_skills_and_files_in_order() {
        let mut p = None;
        let mut entries: Vec<crate::chatmention::MentionEntry> = vec![
            crate::chatmention::MentionEntry::Agent {
                name: "reviewer".into(),
                role: "r".into(),
            },
            crate::chatmention::MentionEntry::Skill {
                name: "review".into(),
                desc: "d".into(),
            },
            crate::chatmention::MentionEntry::File("review.md".into()),
        ];
        after_edit(&mut p, "@rev", None, || entries.clone());
        let items = &p.as_ref().unwrap().items;
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert_eq!(labels, vec!["@reviewer", "@skill:review", "@review.md"]);
        assert_eq!(items[1].fill, "skill:review");
        assert_eq!(items[2].fill, "review.md");
        // narrowing refilters without rescanning
        entries.clear();
        after_edit(&mut p, "@revi", None, || {
            unreachable!("no rescan while open")
        });
        assert!(p.is_some());
    }

    #[test]
    fn multi_target_plus_offers_agents_only() {
        let mut p = None;
        let entries = vec![
            crate::chatmention::MentionEntry::Agent {
                name: "coder".into(),
                role: "c".into(),
            },
            crate::chatmention::MentionEntry::File("coder.md".into()),
        ];
        after_edit(&mut p, "@planner+co", None, || entries.clone());
        let labels: Vec<&str> = p
            .as_ref()
            .unwrap()
            .items
            .iter()
            .map(|i| i.label.as_str())
            .collect();
        assert_eq!(labels, vec!["@coder"]);
    }

    #[test]
    fn pending_palette_detects_the_model_arg_phase() {
        assert_eq!(pending_palette("/model "), Some((Kind::Model, "")));
        assert_eq!(pending_palette("/model son"), Some((Kind::Model, "son")));
        assert_eq!(pending_palette("/model  son"), Some((Kind::Model, "son")));
        // Two argument tokens = the freeform per-agent / explicit-all form.
        assert_eq!(pending_palette("/model all qwen-max"), None);
        assert_eq!(pending_palette("/model coder qwen"), None);
        // Still the plain slash palette before the space.
        assert_eq!(pending_palette("/model"), Some((Kind::Slash, "model")));
        // Other commands keep their old behaviour.
        assert_eq!(pending_palette("/theme dark"), None);
    }

    #[test]
    fn model_rows_accept_into_a_full_broker_command_and_submit() {
        let mut p = None;
        after_edit(&mut p, "/model son", None, Vec::new);
        let open = p.as_ref().expect("model picker opens");
        assert_eq!(open.kind, Kind::Model);
        assert!(open.items.iter().any(|i| i.header)); // grouped
        assert!(!open.items[open.sel].header); // selection never starts on a header

        let mut input = "/model son".to_string();
        let key = popup_key(&mut p, &mut input, &ChatInput::Enter);
        assert!(matches!(key, PaletteKey::Submit));
        assert!(input.starts_with("/model all "), "{input}");
        assert!(p.is_none()); // accepting closes
    }

    #[test]
    fn popup_key_navigates_accepts_and_closes() {
        let mut p = None;
        after_edit(&mut p, "@", None, agent_entries);
        let mut input = "@".to_string();
        assert!(matches!(
            popup_key(&mut p, &mut input, &ChatInput::Down),
            PaletteKey::Consumed
        ));
        assert!(matches!(
            popup_key(&mut p, &mut input, &ChatInput::Enter),
            PaletteKey::Consumed
        ));
        assert!(input.starts_with('@') && input.ends_with(' '));
        assert!(p.is_none());
        // Esc closes the popup, not the pane.
        after_edit(&mut p, "/", None, Vec::new);
        assert!(matches!(
            popup_key(&mut p, &mut input, &ChatInput::Close),
            PaletteKey::Consumed
        ));
        assert!(p.is_none());
        // Closed popup forwards.
        let mut none: Option<PaletteState> = None;
        assert!(matches!(
            popup_key(&mut none, &mut input, &ChatInput::Enter),
            PaletteKey::Forward
        ));
    }
}
