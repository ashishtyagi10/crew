//! Files of yours in the reply box (the user, 2026-10-21: "add bcc and an
//! attach-a-file option to the box and we might need option to see file
//! attachment"). A file comes in from the system's chooser (`pick`), opened
//! on the box's row of files, or dropped on the pane. Each is named with its
//! size on that row; Enter or a click opens it in its own app, to see it,
//! and Backspace or its `✕` takes it off. Nothing is read until it is sent.
use std::path::{Path, PathBuf};

use super::InboxPane;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Attached {
    pub path: PathBuf,
    pub name: String,
    pub size: u64,
}

/// A file's type, by its extension: what a mail reader needs to show it.
pub(crate) fn mime_of(name: &str) -> &'static str {
    let ext = name
        .rsplit_once('.')
        .map_or("", |(_, e)| e)
        .to_ascii_lowercase();
    match ext.as_str() {
        "pdf" => "application/pdf",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "heic" => "image/heic",
        "svg" => "image/svg+xml",
        "txt" | "log" => "text/plain",
        "md" => "text/markdown",
        "csv" => "text/csv",
        "html" | "htm" => "text/html",
        "json" => "application/json",
        "zip" => "application/zip",
        "doc" => "application/msword",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "xls" => "application/vnd.ms-excel",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "ppt" => "application/vnd.ms-powerpoint",
        "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        "ics" => "text/calendar",
        "mp4" => "video/mp4",
        "mov" => "video/quicktime",
        "mp3" => "audio/mpeg",
        _ => "application/octet-stream",
    }
}

impl InboxPane {
    /// `path` onto the box's row of files: the line to say.
    pub(crate) fn attach(&mut self, path: &Path) -> String {
        let Some(c) = self.compose.as_mut().filter(|c| !c.sending) else {
            return String::new();
        };
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let size = match std::fs::metadata(path) {
            Ok(m) if m.is_dir() => {
                return format!("{name} is a folder \u{2014} attach the files in it")
            }
            Ok(m) => m.len(),
            Err(_) => return format!("could not read {}", path.display()),
        };
        if c.files.iter().any(|f| f.path == path) {
            return format!("{name} is attached already");
        }
        let carried = c.forward.as_ref().map_or(0, |f| f.size());
        let total = carried + size + c.files.iter().map(|f| f.size).sum::<u64>();
        if total > super::forward::MOST {
            return format!(
                "{name} would make {} MB, more than the 25 MB Gmail sends",
                total.div_ceil(1024 * 1024)
            );
        }
        let size_s = super::save::size(size);
        c.files.push(Attached {
            path: path.to_path_buf(),
            name: name.clone(),
            size,
        });
        format!("attached {name} \u{b7} {size_s}")
    }

    /// The system's file chooser, for the box: what it picks lands on the
    /// row when it answers (`picked`).
    pub(crate) fn choose_files(&mut self) {
        if let Some(c) = self.compose.as_mut().filter(|c| c.picking.is_none()) {
            c.picking = Some(super::pick::spawn());
            self.note = "choose the files to attach\u{2026}".into();
        }
    }

    /// Whatever the chooser answered, onto the row: true when it did.
    pub(crate) fn picked(&mut self) -> bool {
        let Some(c) = self.compose.as_mut() else {
            return false;
        };
        let Some(got) = c.picking.as_ref().and_then(|rx| rx.try_recv().ok()) else {
            return false;
        };
        c.picking = None;
        match got {
            // Every file that did not go on is said, not only the last
            // file's line: a refusal mid-way went unseen.
            Ok(paths) => {
                let (mut on, mut refused) = (Vec::new(), Vec::new());
                for p in &paths {
                    let had = self.compose.as_ref().map_or(0, |c| c.files.len());
                    let said = self.attach(p);
                    match self.compose.as_ref().map_or(0, |c| c.files.len()) > had {
                        true => on.push(said),
                        false => refused.push(said),
                    }
                }
                let attached = match on.len() {
                    0 => None,
                    1 => on.pop(),
                    n => Some(format!("attached {n} files")),
                };
                self.note = attached
                    .into_iter()
                    .chain(refused)
                    .collect::<Vec<_>>()
                    .join(" \u{b7} ");
            }
            Err(e) => self.note = e,
        }
        true
    }

    /// File `i` of the box opened in its own app, to see it.
    pub(crate) fn open_attached(&mut self, i: usize) {
        if let Some(f) = self.compose.as_ref().and_then(|c| c.files.get(i)) {
            crate::linkclick::open_external(&f.path.to_string_lossy());
            self.note = format!("opening {}", f.name);
        }
    }

    /// File `i` taken off the box.
    pub(crate) fn unattach(&mut self, i: usize) {
        let Some(c) = self.compose.as_mut() else {
            return;
        };
        if i < c.files.len() {
            let f = c.files.remove(i);
            c.pick = c.pick.min(c.files.len());
            self.note = format!("took {} off", f.name);
        }
    }
}

#[cfg(test)]
#[path = "attach_box_tests.rs"]
mod tests;
