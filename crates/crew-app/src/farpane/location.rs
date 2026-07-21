//! Where a Far panel is rooted: the local filesystem, or an `rclone` remote
//! (e.g. Google Drive). Local panels keep a real `PathBuf`; remote panels
//! carry the remote name plus a `/`-joined sub-path. `rclone_addr` produces
//! the `remote:sub/path` string every `rclone` subcommand takes.
use std::path::{Path, PathBuf};

/// The storage backend a panel is browsing.
#[derive(Clone, PartialEq, Eq)]
pub(crate) enum Backend {
    Local,
    Rclone { remote: String },
}

/// A resolved location: a backend plus a path within it. For `Local` the path
/// is an absolute filesystem path; for `Rclone` it is the remote-relative
/// sub-path (empty at the remote root).
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct Location {
    pub backend: Backend,
    pub path: String,
}

impl Location {
    pub(crate) fn local(p: &Path) -> Self {
        Self {
            backend: Backend::Local,
            path: p.to_string_lossy().into_owned(),
        }
    }

    pub(crate) fn is_remote(&self) -> bool {
        matches!(self.backend, Backend::Rclone { .. })
    }

    pub(crate) fn local_path(&self) -> Option<PathBuf> {
        match self.backend {
            Backend::Local => Some(PathBuf::from(&self.path)),
            Backend::Rclone { .. } => None,
        }
    }

    /// The `rclone`-addressable string: `remote:sub/path` for a remote, or the
    /// plain filesystem path for local.
    pub(crate) fn rclone_addr(&self) -> String {
        match &self.backend {
            Backend::Local => self.path.clone(),
            Backend::Rclone { remote } => format!("{remote}:{}", self.path),
        }
    }

    /// What the panel legend shows.
    pub(crate) fn display(&self) -> String {
        match &self.backend {
            Backend::Local => self.path.clone(),
            Backend::Rclone { .. } => self.rclone_addr(),
        }
    }

    /// Descend into `name`.
    pub(crate) fn child(&self, name: &str) -> Self {
        match &self.backend {
            Backend::Local => Self::local(&PathBuf::from(&self.path).join(name)),
            Backend::Rclone { remote } => {
                let path = if self.path.is_empty() {
                    name.to_string()
                } else {
                    format!("{}/{name}", self.path)
                };
                Self {
                    backend: Backend::Rclone {
                        remote: remote.clone(),
                    },
                    path,
                }
            }
        }
    }

    pub(crate) fn has_parent(&self) -> bool {
        match &self.backend {
            Backend::Local => PathBuf::from(&self.path).parent().is_some(),
            Backend::Rclone { .. } => !self.path.is_empty(),
        }
    }

    /// Ascend one level; `None` at a root (filesystem root or remote root).
    pub(crate) fn parent(&self) -> Option<Self> {
        match &self.backend {
            Backend::Local => PathBuf::from(&self.path).parent().map(|p| Self::local(p)),
            Backend::Rclone { remote } => {
                if self.path.is_empty() {
                    return None;
                }
                let parent = match self.path.rsplit_once('/') {
                    Some((head, _)) => head.to_string(),
                    None => String::new(),
                };
                Some(Self {
                    backend: Backend::Rclone {
                        remote: remote.clone(),
                    },
                    path: parent,
                })
            }
        }
    }
}

#[cfg(test)]
#[path = "location_tests.rs"]
mod tests;
