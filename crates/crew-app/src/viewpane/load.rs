//! Fetching a file for the viewer, always on a worker thread. Detection is
//! here too rather than at the call site, because classifying needs the head
//! of the file and reading it is I/O — doing that on the winit thread would
//! freeze every pane in the grid, agents included.
//!
//! Argv construction is pure and separately tested, the same split
//! `farpane/rclone.rs` makes, so CI covers the extractor commands on a
//! machine with neither tool installed.
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};

use super::detect::{detect, Extractor, Format, Opaque, Probe, SNIFF_BYTES};
use crew_hive::childproc::no_console_window;

/// How much of a file the pane will show. Deliberately a cap on what is
/// DISPLAYED, not on what may be opened: the 40 MB log is precisely the file
/// you want to look at the top of.
pub(crate) const MAX_VIEW_BYTES: u64 = 8 * 1024 * 1024;

/// Loaded text plus, when the file was longer than the cap, its real size —
/// which the banner names so the truncation is never silent.
#[derive(Debug)]
pub(crate) struct Loaded {
    pub text: String,
    pub truncated: Option<u64>,
    /// Size and mtime, for the rungs that render no text of their own (Fix
    /// 2: the metadata card). Populated on every successful read — the
    /// `stat` in `read_capped` already has it, so this is free — but only
    /// the opaque rung draws it today.
    pub meta: Option<FileMeta>,
    /// The decoded picture, for the image rung. Decoded and downscaled HERE,
    /// on the worker: a 40-megapixel photo run through `image` on the winit
    /// thread would freeze every pane in the grid for as long as it took.
    pub image: Option<super::bitmap::Bitmap>,
}

/// A file's size and modification time, captured once on the worker
/// alongside the bytes. Copy: two small fields, cheap to pass around rather
/// than borrow.
#[derive(Debug, Clone, Copy)]
pub(crate) struct FileMeta {
    pub size: u64,
    pub modified: Option<std::time::SystemTime>,
}

/// What the worker sends back: the rung it decided on, and the text or the
/// reason there is none.
pub(crate) struct LoadDone {
    pub format: Format,
    pub result: Result<Loaded, String>,
}

/// The extractor's argv, minus the binary name. Pure.
pub(crate) fn argv(e: Extractor, p: &Path) -> Vec<String> {
    let path = p.to_string_lossy().into_owned();
    match e {
        Extractor::TextUtil => vec!["-convert".into(), "txt".into(), "-stdout".into(), path],
        Extractor::PdfToText => vec!["-layout".into(), path, "-".into()],
    }
}

/// Which extractors are on `PATH`. Probed once per process — a tool does not
/// appear mid-session, and `which` is a fork we should not repeat per open.
pub(crate) fn probe() -> Probe {
    use std::sync::OnceLock;
    static PROBE: OnceLock<Probe> = OnceLock::new();
    *PROBE.get_or_init(|| Probe {
        textutil: on_path(Extractor::TextUtil.bin()),
        pdftotext: on_path(Extractor::PdfToText.bin()),
    })
}

fn on_path(bin: &str) -> bool {
    let Some(paths) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&paths).any(|d| d.join(bin).is_file())
}

/// Read at most `MAX_VIEW_BYTES`, reporting the real size when longer, plus
/// the `FileMeta` the same `stat` already produced (Fix 2) — one syscall
/// serves both jobs rather than the render path stat-ing again later.
fn read_capped(path: &Path) -> std::io::Result<(Vec<u8>, FileMeta, Option<u64>)> {
    let md = std::fs::metadata(path)?;
    let size = md.len();
    let modified = md.modified().ok();
    let mut f = std::fs::File::open(path)?;
    let mut buf = Vec::new();
    f.by_ref().take(MAX_VIEW_BYTES).read_to_end(&mut buf)?;
    let truncated = (size > MAX_VIEW_BYTES).then_some(size);
    Ok((buf, FileMeta { size, modified }, truncated))
}

/// Cap extracted text at [`MAX_VIEW_BYTES`], the same limit `read_capped`
/// applies to a plain file — the cap is on what the pane SHOWS, and an
/// extractor's output is no exception. Unlike a raw byte slice, `String`
/// truncation must land on a `char` boundary or it panics, so this walks
/// back from the byte cap rather than indexing straight into it.
fn cap_text(mut text: String) -> (String, Option<u64>) {
    let len = text.len() as u64;
    if len <= MAX_VIEW_BYTES {
        return (text, None);
    }
    let mut boundary = MAX_VIEW_BYTES as usize;
    while boundary > 0 && !text.is_char_boundary(boundary) {
        boundary -= 1;
    }
    text.truncate(boundary);
    (text, Some(len))
}

/// Run an extractor and capture its stdout. A non-zero exit is reported with
/// the tool's own stderr — it knows why it failed and we do not.
fn extract(e: Extractor, path: &Path) -> Result<String, String> {
    let out = no_console_window(&mut std::process::Command::new(e.bin()))
        .args(argv(e, path))
        .output()
        .map_err(|err| format!("{}: {err}", e.bin()))?;
    if !out.status.success() {
        let tail = String::from_utf8_lossy(&out.stderr);
        let tail = tail.lines().last().unwrap_or("failed").to_string();
        return Err(format!("{}: {tail}", e.bin()));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// The whole job, synchronously — detection included. Called on the worker by
/// [`start`], and directly by tests.
pub(crate) fn load_now(path: &Path, probe: Probe) -> LoadDone {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned());

    let (head, meta, truncated) = match read_capped(path) {
        Ok(v) => v,
        Err(e) => {
            return LoadDone {
                // `Unreadable`, not `Binary` (Fix 2): this branch never read
                // a byte, so it has no basis to claim the file IS anything —
                // a permission error is not a binary file.
                format: Format::Opaque {
                    why: Opaque::Unreadable,
                },
                result: Err(format!("{name}: {e}")),
            };
        }
    };
    let sniff = &head[..head.len().min(SNIFF_BYTES)];
    let format = detect(path, sniff, probe);

    let result = match format {
        // The card is drawn from the path, the format, and `meta` alone;
        // there is no text to carry, and decoding these bytes would be a
        // lie.
        Format::Opaque { .. } => Ok(Loaded {
            text: String::new(),
            truncated: None,
            meta: Some(meta),
            image: None,
        }),
        // A file too big for the read cap arrives here half-read, and half a
        // PNG decodes to nothing — which is the honest answer, and the one
        // the caption gives.
        Format::Image { kind } => match super::bitmap::decode(&head) {
            Some(bm) => Ok(Loaded {
                text: String::new(),
                truncated: None,
                meta: Some(meta),
                image: Some(bm),
            }),
            None => Err(format!("{name}: not a {kind} this build can decode")),
        },
        Format::Extract { via } => extract(via, path)
            .map(|text| {
                let (text, truncated) = cap_text(text);
                Loaded {
                    text,
                    truncated,
                    meta: Some(meta),
                    image: None,
                }
            })
            .map_err(|e| format!("{name}: {e}")),
        _ => Ok(Loaded {
            text: String::from_utf8_lossy(&head).into_owned(),
            truncated,
            meta: Some(meta),
            image: None,
        }),
    };
    LoadDone { format, result }
}

/// Load `path` on a worker thread; the result arrives on the returned
/// channel. Dropping the receiver discards the result, which is what closing
/// the pane mid-load should do.
///
/// `probe()` is called *inside* the closure, not before it: on a first call
/// in the process it walks every `PATH` directory doing real `stat`s, and
/// this function is reachable from the winit thread on every file open. Any
/// work here between `mpsc::channel()` and `thread::spawn` runs on whichever
/// thread called `start` — so there must be none.
pub(crate) fn start(path: PathBuf) -> Receiver<LoadDone> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(load_now(&path, probe()));
    });
    rx
}

#[cfg(test)]
#[path = "load_tests.rs"]
mod tests;
