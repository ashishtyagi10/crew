use super::types::{GitFileStatus, TreeState};
use std::path::PathBuf;

impl TreeState {
    /// Refresh git status for the current root directory.
    /// Runs `git status --porcelain` and parses per-file status.
    pub fn refresh_git_status(&mut self) {
        self.git_status.clear();
        self.in_git_repo = false;

        // Check if we're in a git repo by finding the git root
        let git_root = match std::process::Command::new("git")
            .args(["rev-parse", "--show-toplevel"])
            .current_dir(&self.root)
            .output()
        {
            Ok(out) if out.status.success() => {
                let root = String::from_utf8_lossy(&out.stdout).trim().to_string();
                PathBuf::from(root)
            }
            _ => return,
        };

        self.in_git_repo = true;

        let output = match std::process::Command::new("git")
            .args(["status", "--porcelain", "-uall"])
            .current_dir(&git_root)
            .output()
        {
            Ok(out) if out.status.success() => out.stdout,
            _ => return,
        };

        let text = String::from_utf8_lossy(&output);
        for line in text.lines() {
            if line.len() < 4 {
                continue;
            }
            let xy = &line[..2];
            let path_str = &line[3..];
            // Handle renames: "R  old -> new"
            let file_path = if let Some(arrow) = path_str.find(" -> ") {
                &path_str[arrow + 4..]
            } else {
                path_str
            };
            let abs_path = git_root.join(file_path);
            let status = match xy {
                "??" => GitFileStatus::Untracked,
                "!!" => GitFileStatus::Ignored,
                "UU" | "AA" | "DD" => GitFileStatus::Conflict,
                _ => {
                    let index = xy.as_bytes()[0];
                    let worktree = xy.as_bytes()[1];
                    if index == b'R' || worktree == b'R' {
                        GitFileStatus::Renamed
                    } else if index == b'D' || worktree == b'D' {
                        GitFileStatus::Deleted
                    } else if index != b' ' && index != b'?' {
                        GitFileStatus::Staged
                    } else if worktree == b'M' || worktree == b'A' {
                        GitFileStatus::Modified
                    } else {
                        continue;
                    }
                }
            };
            self.git_status.insert(abs_path, status);
        }
    }

    /// Get the git status for a given absolute path.
    pub fn git_status_for(&self, path: &PathBuf) -> Option<GitFileStatus> {
        self.git_status.get(path).copied()
    }
}
