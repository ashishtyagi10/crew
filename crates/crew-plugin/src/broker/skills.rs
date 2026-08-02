//! Skills: reusable prompt playbooks, one markdown file each, with an optional
//! `---` frontmatter header (`name:` / `description:`). Loaded from the user
//! dir (`~/.config/crew/skills/`) and the project dir (`./.crew/skills/`);
//! a project skill overrides a user skill with the same name. There is no
//! command any more: a relay/swarm task that names a skill picks its playbook
//! up automatically (see `skillframe::with_skills`).
use std::path::{Path, PathBuf};

/// One loaded playbook.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Skill {
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

/// The project dir skills load from. Mirrors `sessionlog::base_dir` exactly,
/// and for the same reason: `CREW_PROJECT_DIR` overrides the process CWD —
/// the seam tests use, since lib tests share one CWD and cannot each chdir.
/// Production never sets it: the broker's CWD *is* the project.
fn base_dir() -> PathBuf {
    std::env::var("CREW_PROJECT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

/// Load every skill visible to this broker.
pub(crate) fn load() -> Vec<Skill> {
    list(&base_dir())
}

/// User + project skills, the project dir rooted explicitly at
/// `project_root` — the GUI's attach picker lists skills for a pane whose
/// cwd is not the process cwd.
pub fn list(project_root: &Path) -> Vec<Skill> {
    let user = dirs::config_dir()
        .map(|d| load_dir(&d.join("crew").join("skills"), "user"))
        .unwrap_or_default();
    let project = load_dir(&project_root.join(".crew/skills"), "project");
    merge(user, project)
}

#[cfg(test)]
#[path = "skills_tests.rs"]
mod tests;
