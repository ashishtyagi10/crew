//! An attachment saved and opened (goal 2026-10-20-mail-reader, item 17): the
//! one thing this goal writes to disk, and only when asked — a Cmd+click on
//! the attachment's row, or its capital letter in `Cmd+E`'s hint mode.
//!
//! It lands in `~/Downloads` under the name the sender gave it, made safe:
//! no folders, no leading dot, nothing a file system refuses, and never over
//! a file already there (`report (1).pdf`).
use std::path::{Path, PathBuf};

use super::api::Gmail;
use super::worker::{Ask, Got};
use super::InboxPane;

/// The link an attachment's row carries: `crew-attachment:<message>/<n>`.
pub(crate) const ATTACHMENT: &str = "crew-attachment:";

/// `name` as a file name crew will write: its last path part, no control
/// characters or reserved ones, no leading dots, 120 characters at most.
pub(crate) fn safe_name(name: &str) -> String {
    let last = name.rsplit(['/', '\\']).next().unwrap_or("");
    let clean: String = last
        .chars()
        .filter(|c| !c.is_control() && !"<>:\"|?*".contains(*c))
        .take(120)
        .collect();
    match clean.trim().trim_start_matches('.') {
        "" => "attachment".into(),
        s => s.to_string(),
    }
}

/// `dir/name`, or `dir/name (1).ext`, `(2)`, … — the first that is free.
pub(crate) fn free_path(dir: &Path, name: &str) -> PathBuf {
    let (stem, ext) = match name.rsplit_once('.') {
        Some((s, e)) if !s.is_empty() => (s.to_string(), format!(".{e}")),
        _ => (name.to_string(), String::new()),
    };
    (0..)
        .map(|n| match n {
            0 => dir.join(name),
            n => dir.join(format!("{stem} ({n}){ext}")),
        })
        .find(|p| !p.exists())
        .expect("an unbounded range finds a free name")
}

/// The worker's answer to [`Ask::Save`]: the file written, or why not.
pub(crate) fn save(
    g: &mut Gmail,
    msg: &str,
    att: Option<String>,
    data: Option<String>,
    name: &str,
) -> Got {
    let bytes = match (data.as_deref().and_then(super::body::bytes), att) {
        (Some(b), _) => Ok(b),
        (None, Some(a)) => g.attachment(msg, &a),
        (None, None) => Err("the attachment has no bytes".into()),
    };
    let dir = dirs::download_dir().ok_or_else(|| "no Downloads folder".to_string());
    Got::Saved(bytes.and_then(|b| {
        let path = free_path(&dir?, &safe_name(name));
        std::fs::write(&path, b).map_err(|e| format!("could not write {}: {e}", path.display()))?;
        Ok(path)
    }))
}

impl InboxPane {
    /// A row's `crew-attachment:` link, acted on: the line to say.
    pub(crate) fn save_attachment(&mut self, link: &str) -> String {
        let rest = link.strip_prefix(ATTACHMENT).unwrap_or(link);
        let found = rest.rsplit_once('/').and_then(|(msg, n)| {
            let mails = self.reading.as_ref()?.mails.as_deref()?;
            let m = mails.iter().find(|m| m.id == msg)?;
            Some((
                m.id.clone(),
                m.attached.get(n.parse::<usize>().ok()?)?.clone(),
            ))
        });
        let Some((msg, part)) = found else {
            return "that attachment is not in the open thread".into();
        };
        self.ask(Ask::Save {
            msg,
            att: part.att,
            data: part.data,
            name: part.name.clone(),
        });
        self.busy = false;
        format!("saving {}\u{2026}", safe_name(&part.name))
    }
}

/// A size as people read it: `840 B`, `12 KB`, `3.4 MB`.
pub(crate) fn size(bytes: u64) -> String {
    match bytes {
        b if b < 1024 => format!("{b} B"),
        b if b < 1024 * 1024 => format!("{} KB", b / 1024),
        b => format!("{:.1} MB", b as f64 / (1024.0 * 1024.0)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_senders_name_is_made_safe_and_never_lands_on_a_file() {
        assert_eq!(safe_name("../../etc/passwd"), "passwd");
        assert_eq!(safe_name("..\\x\\.hidden"), "hidden");
        assert_eq!(safe_name("re:port?.pdf"), "report.pdf");
        assert_eq!(safe_name(""), "attachment");
        let dir = std::env::temp_dir().join(format!("crew-save-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.pdf"), b"x").unwrap();
        std::fs::write(dir.join("a (1).pdf"), b"x").unwrap();
        assert_eq!(free_path(&dir, "a.pdf"), dir.join("a (2).pdf"));
        assert_eq!(free_path(&dir, "b"), dir.join("b"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn sizes_read_as_people_say_them() {
        assert_eq!(
            [size(840), size(12_400), size(3_565_158)],
            ["840 B", "12 KB", "3.4 MB"]
        );
    }
}

#[cfg(test)]
#[path = "attach_tests.rs"]
mod attach_tests;
