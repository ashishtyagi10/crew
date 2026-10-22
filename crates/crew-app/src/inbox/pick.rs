//! The system's own file chooser, run beside the app: AppleScript's
//! `choose file` on a Mac, Windows Forms' open dialog through PowerShell,
//! zenity or kdialog on Linux. Each prints the chosen paths a line each;
//! Cancel prints nothing. No dialog crate, so nothing new to build or link.
use std::path::PathBuf;
use std::process::Command;
use std::sync::mpsc::{self, Receiver};

use crew_hive::childproc::no_console_window;

const TITLE: &str = "Attach to your mail";

/// Open the chooser on a thread of its own: the paths come back on the
/// channel, none when it was cancelled, or why there is no chooser.
pub(crate) fn spawn() -> Receiver<Result<Vec<PathBuf>, String>> {
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
            Ok(out) => return Ok(paths(&String::from_utf8_lossy(&out.stdout))),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => return Err(format!("the file chooser did not open: {e}")),
        }
    }
    Err("no file chooser here \u{2014} drop the file on the pane instead".into())
}

/// The chooser's printout as paths: a line each, blanks and non-files out.
pub(crate) fn paths(out: &str) -> Vec<PathBuf> {
    out.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .collect()
}

/// The choosers to try, in order.
pub(crate) fn commands() -> Vec<Command> {
    if cfg!(target_os = "macos") {
        let script = [
            "activate".to_string(),
            format!(
                "set fs to choose file with prompt \"{TITLE}\" with multiple selections allowed"
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
            "Add-Type -AssemblyName System.Windows.Forms; $d = New-Object System.Windows.Forms.OpenFileDialog; $d.Multiselect = $true; $d.Title = '{TITLE}'; if ($d.ShowDialog() -eq 'OK') {{ $d.FileNames }}"
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
            "--multiple",
            "--separator=\n",
            "--title",
            TITLE,
        ]);
        let mut k = Command::new("kdialog");
        no_console_window(&mut k);
        k.args([
            "--getopenfilename",
            "--multiple",
            "--separate-output",
            "--title",
            TITLE,
        ]);
        vec![z, k]
    }
}
