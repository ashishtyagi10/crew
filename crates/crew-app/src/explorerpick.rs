//! The Explorer's `+`: a button at the end of its rule that adds a folder beside the pane's —
//! VS Code's "Add Folder to Workspace". It opens the system's own folder chooser, run beside
//! the app as the mail's file chooser is (`inbox::pick`): AppleScript's `choose folder` on a
//! Mac, a FolderBrowserDialog through PowerShell on Windows, zenity or kdialog on Linux. Every
//! folder chosen is pinned as `/nav explorer add` pins one (`explorerpin`); `/nav explorer add`
//! with no path opens the chooser too.
use std::path::PathBuf;
use std::process::Command;
use std::sync::mpsc::{self, Receiver, TryRecvError};

use crew_hive::childproc::no_console_window;
use crew_render::CellView;

use crate::app::CrewApp;
use crate::navsection::NavSection;

const TITLE: &str = "Add a folder to the Explorer";

/// What the chooser answers: the folders chosen (none when cancelled), or why there is none.
pub(crate) type Chosen = Receiver<Result<Vec<PathBuf>, String>>;

impl CrewApp {
    /// Whether the pointer is on the rule's `+ ADD`: the rule's row, the columns it covers.
    pub(crate) fn explorer_add_at_cursor(&self) -> bool {
        let on_rule = matches!(self.section_row(NavSection::Explorer), Some((0, _)));
        let span =
            |(c, cols): (u16, u16)| add_span(cols).is_some_and(|(a, b, _)| (a..=b).contains(&c));
        on_rule && self.nav_cursor_col().is_some_and(span)
    }

    /// Open the folder chooser, unless it is open already.
    pub(crate) fn explorer_choose_folder(&mut self) {
        if self.sidebar.explorer.choosing.is_some() {
            return;
        }
        self.sidebar.explorer.choosing = Some(spawn());
        self.set_status("explorer: choose a folder to add\u{2026}".to_string());
    }

    /// Pin whatever the chooser answered. `true` when it answered.
    pub(crate) fn poll_explorer_chooser(&mut self) -> bool {
        let Some(rx) = self.sidebar.explorer.choosing.as_ref() else {
            return false;
        };
        let got = match rx.try_recv() {
            Ok(got) => got,
            Err(TryRecvError::Empty) => return false,
            Err(TryRecvError::Disconnected) => Err("the folder chooser closed".into()),
        };
        self.sidebar.explorer.choosing = None;
        match got {
            Ok(dirs) if dirs.is_empty() => self.set_status("explorer: no folder added".to_string()),
            Ok(dirs) => dirs
                .into_iter()
                .for_each(|d| self.explorer_pin_path(crate::pathexpand::normalized(&d))),
            Err(e) => self.set_status_err(format!("explorer: {e}")),
        }
        self.redraw();
        true
    }
}

/// The rule's button: `+ ADD` ending on the `×` column, or a lone `+` on a panel too narrow
/// for the word. The columns it covers, first and last — the draw and the click read these.
pub(crate) fn add_span(cols: u16) -> Option<(u16, u16, &'static str)> {
    let end = crate::navfarrow::x_col(cols);
    match () {
        _ if cols >= 22 => Some((end - 4, end, ADD)),
        _ if end >= 4 => Some((end, end, "+")),
        _ => None,
    }
}

const ADD: &str = "+ ADD";

/// The button drawn over the rule, a space either side so it reads apart from it: always
/// there, so the way to add a folder is in sight.
pub(crate) fn add_button(out: &mut Vec<CellView>, cols: u16, lit: bool) {
    let Some((from, to, label)) = add_span(cols) else {
        return;
    };
    out.retain(|v| !(v.row == 0 && (from - 1..=to).contains(&v.col)));
    let n = out.len();
    let ink = crate::accentink::accent_ink();
    crate::navtext::put_at(out, &format!(" {label} "), from - 1, 0, cols, ink);
    // Under the pointer the words are a chip: the accent filled behind them, as the keyboard's
    // row is (`explorerdraw::bar`).
    let (on, fill) = crate::segment::inked(crate::palette::accent());
    for v in &mut out[n..] {
        v.bold = true;
        if lit && (from..=to).contains(&v.col) {
            (v.fg, v.bg) = (on, fill);
        }
    }
}

/// Open the chooser on a thread of its own.
pub(crate) fn spawn() -> Chosen {
    let (tx, rx) = mpsc::channel();
    #[cfg(not(test))]
    std::thread::spawn(move || {
        let _ = tx.send(run());
    });
    // Tests choose without a dialog: whatever `CHOSEN` holds.
    #[cfg(test)]
    let _ = tx.send(Ok(CHOSEN.with(|c| std::mem::take(&mut *c.borrow_mut()))));
    rx
}

#[cfg(test)]
thread_local! {
    pub(crate) static CHOSEN: std::cell::RefCell<Vec<PathBuf>> = const { std::cell::RefCell::new(Vec::new()) };
}

#[cfg_attr(test, allow(dead_code))]
fn run() -> Result<Vec<PathBuf>, String> {
    for mut cmd in commands() {
        match cmd.output() {
            Ok(out) => {
                let dirs = crate::inbox::pick::paths(&String::from_utf8_lossy(&out.stdout));
                return Ok(dirs.into_iter().filter(|d| d.is_dir()).collect());
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => return Err(format!("the folder chooser did not open: {e}")),
        }
    }
    Err("no folder chooser here \u{2014} type /nav explorer add <folder>".into())
}

/// The choosers to try, in order.
#[cfg_attr(test, allow(dead_code))]
fn commands() -> Vec<Command> {
    if cfg!(target_os = "macos") {
        let script = [
            "activate".to_string(),
            format!(
                "set fs to choose folder with prompt \"{TITLE}\" with multiple selections allowed"
            ),
            "set out to \"\"".into(),
            "repeat with f in fs".into(),
            "set out to out & POSIX path of f & linefeed".into(),
            "end repeat".into(),
            "return out".into(),
        ];
        let mut c = Command::new("osascript");
        no_console_window(&mut c);
        for line in script {
            c.args(["-e", &line]);
        }
        vec![c]
    } else if cfg!(windows) {
        let ps = format!(
            "Add-Type -AssemblyName System.Windows.Forms; $d = New-Object System.Windows.Forms.FolderBrowserDialog; $d.Description = '{TITLE}'; if ($d.ShowDialog() -eq 'OK') {{ $d.SelectedPath }}"
        );
        let mut c = Command::new("powershell");
        no_console_window(&mut c);
        c.args(["-NoProfile", "-STA", "-Command", &ps]);
        vec![c]
    } else {
        let mut z = Command::new("zenity");
        no_console_window(&mut z);
        z.args([
            "--file-selection",
            "--directory",
            "--multiple",
            "--separator=\n",
            "--title",
            TITLE,
        ]);
        let mut k = Command::new("kdialog");
        no_console_window(&mut k);
        k.args(["--getexistingdirectory", ".", "--title", TITLE]);
        vec![z, k]
    }
}

#[cfg(test)]
#[path = "explorerpick_tests.rs"]
mod tests;
