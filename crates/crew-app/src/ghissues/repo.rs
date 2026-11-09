//! Which GitHub repo a todo's `@project` is: the project resolved to its
//! checkout the way `r` resolves it (`projectdir`, without pane hints — the
//! sync runs on its own thread), then the checkout's `origin` remote read as
//! `owner/name`. A project that is not a checkout, or pushes somewhere other
//! than github.com, has no repo, and its todos stay crew's alone.
use std::collections::HashMap;
use std::process::Command;

use crew_hive::childproc::no_console_window;

use crate::todopane::projectdir;

/// `owner/name` of a github.com remote URL, in any of git's spellings.
pub(crate) fn slug(url: &str) -> Option<String> {
    let url = url.trim();
    let rest = [
        "git@github.com:",
        "ssh://git@github.com/",
        "https://github.com/",
        "http://github.com/",
        "git://github.com/",
    ]
    .iter()
    .find_map(|p| url.strip_prefix(p))?;
    let rest = rest.trim_end_matches('/').trim_end_matches(".git");
    let mut parts = rest.split('/');
    let (owner, name) = (parts.next()?, parts.next()?);
    let ok = |s: &str| {
        !s.is_empty()
            && s.chars()
                .all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c))
    };
    (ok(owner) && ok(name) && parts.next().is_none()).then(|| format!("{owner}/{name}"))
}

/// Project names to repos, looked up once per pass.
#[derive(Default)]
pub(crate) struct Repos(HashMap<String, Option<String>>);

impl Repos {
    pub(crate) fn of(&mut self, project: &str) -> Option<String> {
        let key = project.to_lowercase();
        self.0
            .entry(key)
            .or_insert_with(|| look_up(project))
            .clone()
    }
}

fn look_up(project: &str) -> Option<String> {
    let dir = projectdir::resolve(project, &projectdir::bound(), &[])?;
    let root = projectdir::git_root(&dir)?;
    let out = no_console_window(&mut Command::new("git"))
        .arg("-C")
        .arg(root)
        .args(["remote", "get-url", "origin"])
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| slug(&String::from_utf8_lossy(&out.stdout)))
        .flatten()
}

#[cfg(test)]
mod tests {
    use super::slug;

    #[test]
    fn every_spelling_of_a_github_remote_reads_as_owner_name() {
        for url in [
            "git@github.com:me/crew.git",
            "https://github.com/me/crew",
            "https://github.com/me/crew.git\n",
            "ssh://git@github.com/me/crew.git",
        ] {
            assert_eq!(slug(url).as_deref(), Some("me/crew"), "{url}");
        }
    }

    #[test]
    fn other_hosts_and_odd_paths_are_not_github() {
        assert_eq!(slug("git@gitlab.com:me/crew.git"), None);
        assert_eq!(slug("https://github.com/me"), None);
        assert_eq!(slug("https://github.com/me/crew/tree/main"), None);
        assert_eq!(slug("https://github.com/me/cr ew"), None);
    }
}
