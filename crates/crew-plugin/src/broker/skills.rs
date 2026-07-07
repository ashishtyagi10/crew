//! Skills: reusable prompt playbooks, one markdown file each, with an optional
//! `---` frontmatter header (`name:` / `description:`). Loaded from the user
//! dir (`~/.config/crew/skills/`) and the project dir (`./.crew/skills/`);
//! a project skill overrides a user skill with the same name. `/skill <name>
//! <task>` runs the normal relay with the playbook prepended to the task.
use std::path::{Path, PathBuf};

use crate::PluginEvent;

use super::relay::{msg, relay_turn, split_target};
use super::session::Session;
use super::stdio::roster;

/// One loaded playbook.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Skill {
    pub name: String,
    pub description: String,
    pub body: String,
    /// Where it came from: `"user"` or `"project"`.
    pub origin: &'static str,
    /// The markdown source: the flat file, or the directory's `SKILL.md`.
    pub path: PathBuf,
    /// The skill's root directory — `Some` only for directory skills.
    pub dir: Option<PathBuf>,
}

/// Parse one skill file. Frontmatter (`---` … `---`) may set `name` and
/// `description`; otherwise the name is the file stem and the description the
/// body's first non-empty line (clipped).
pub(crate) fn parse(text: &str, stem: &str, origin: &'static str) -> Skill {
    let mut name = normalize_name(stem);
    let mut description = String::new();
    let mut body = text.trim();
    if let Some(rest) = text.trim_start().strip_prefix("---") {
        if let Some(end) = rest.find("\n---") {
            for line in rest[..end].lines() {
                match line.split_once(':') {
                    Some((k, v)) if k.trim() == "name" => name = normalize_name(v),
                    Some((k, v)) if k.trim() == "description" => description = v.trim().into(),
                    _ => {}
                }
            }
            let after = &rest[end + 1..]; // starts at the closing "---" line
            body = after.split_once('\n').map_or("", |(_, b)| b).trim();
        }
    }
    if description.is_empty() {
        description = super::route::clip(
            body.lines().find(|l| !l.trim().is_empty()).unwrap_or(""),
            80,
        );
    }
    Skill {
        name,
        description,
        body: body.to_string(),
        origin,
        path: PathBuf::new(),
        dir: None,
    }
}

/// Lowercase, whitespace → `-`, so `/skill` names are easy to type.
fn normalize_name(s: &str) -> String {
    s.trim()
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-")
}

/// All `.md` skills in `dir`, plus subdirectories containing a `SKILL.md` (empty
/// when the dir doesn't exist), sorted by name so loading order is stable.
pub(crate) fn load_dir(dir: &Path, origin: &'static str) -> Vec<Skill> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    // (markdown source, name stem, skill root for directory skills)
    let mut sources: Vec<(PathBuf, String, Option<PathBuf>)> = Vec::new();
    for p in entries.flatten().map(|e| e.path()) {
        if p.extension().is_some_and(|e| e == "md") {
            let Some(stem) = p.file_stem() else { continue };
            sources.push((p.clone(), stem.to_string_lossy().into_owned(), None));
        } else if p.is_dir() && p.join("SKILL.md").is_file() {
            let Some(name) = p.file_name() else { continue };
            let name = name.to_string_lossy().into_owned();
            sources.push((p.join("SKILL.md"), name, Some(p)));
        }
    }
    sources.sort_by(|a, b| a.1.cmp(&b.1));
    sources
        .into_iter()
        .filter_map(|(path, stem, root)| {
            let text = std::fs::read_to_string(&path).ok()?;
            let mut s = parse(&text, &stem, origin);
            s.path = path;
            s.dir = root;
            Some(s)
        })
        .collect()
}
/// User + project skills merged: a project skill replaces a user skill with
/// the same name.
pub(crate) fn merge(user: Vec<Skill>, project: Vec<Skill>) -> Vec<Skill> {
    let mut all = user;
    for s in project {
        match all.iter_mut().find(|u| u.name == s.name) {
            Some(slot) => *slot = s,
            None => all.push(s),
        }
    }
    all.sort_by(|a, b| a.name.cmp(&b.name));
    all
}

/// Load every skill visible from the broker's cwd.
pub(crate) fn load() -> Vec<Skill> {
    let user = dirs::config_dir()
        .map(|d| load_dir(&d.join("crew").join("skills"), "user"))
        .unwrap_or_default();
    let project = load_dir(Path::new(".crew/skills"), "project");
    merge(user, project)
}

/// `/skill <name> <task>` — run one relay turn with the playbook prepended.
pub(crate) fn skill_cmd(
    session: &mut Session,
    rest: &str,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let (name, task) = rest
        .trim()
        .split_once(char::is_whitespace)
        .unwrap_or((rest.trim(), ""));
    let (name, task) = (normalize_name(name), task.trim());
    if name.is_empty() || task.is_empty() {
        return emit(msg(
            "crew",
            "usage: /skill <name> <task> \u{2014} /skills lists them",
        ));
    }
    let skills = load();
    let Some(skill) = skills.iter().find(|s| s.name == name) else {
        let known: Vec<&str> = skills.iter().map(|s| s.name.as_str()).collect();
        let hint = if known.is_empty() {
            "none loaded \u{2014} see /skills".to_string()
        } else {
            known.join(", ")
        };
        return emit(msg(
            "crew",
            format!("unknown skill \u{201c}{name}\u{201d} \u{2014} skills: {hint}"),
        ));
    };
    let reg = session.registry();
    if reg.is_empty() {
        return emit(msg("crew", roster(&reg)));
    }
    let (start, task) = split_target(task, &reg);
    emit(msg(
        "crew",
        format!("skill \u{201c}{name}\u{201d} \u{2014} starting with {start}"),
    ))?;
    let broker = session.broker(reg);
    let sys_on = super::systools::enabled();
    relay_turn(
        &broker,
        &start,
        &super::skillframe::framed(skill, &task, sys_on),
        "skill-1",
        emit,
    )
    .map(|_| ())
}

#[cfg(test)]
#[path = "skills_tests.rs"]
mod tests;
