//! `PtyTerm` constructors: login-shell/command spawn with cwd + env plumbing
//! (child module of `pty` — split for the 200-line cap).
use super::*;

impl PtyTerm {
    /// Spawn a shell (no extra args).  Delegates to `spawn_args`.
    pub fn spawn(size: GridSize, shell: &str) -> anyhow::Result<Self> {
        Self::spawn_args(size, shell, &[])
    }

    /// Spawn `command` with `args` in a new PTY of the given size.
    pub fn spawn_args(size: GridSize, command: &str, args: &[String]) -> anyhow::Result<Self> {
        Self::spawn_in(size, command, args, None)
    }

    /// Spawn `command` with `args` in a new PTY, starting in `cwd` when given
    /// (otherwise the child inherits the host process's working directory).
    pub fn spawn_in(
        size: GridSize,
        command: &str,
        args: &[String],
        cwd: Option<&Path>,
    ) -> anyhow::Result<Self> {
        Self::spawn_with_env(size, command, args, cwd, &[])
    }

    /// As [`Self::spawn_in`], additionally setting `env` vars on the child —
    /// the host's env is inherited otherwise. Crew uses this to hand run panes
    /// the user's login-shell PATH: a Dock-launched app only inherits launchd's
    /// minimal one, under which almost no user command resolves.
    pub fn spawn_with_env(
        size: GridSize,
        command: &str,
        args: &[String],
        cwd: Option<&Path>,
        env: &[(&str, &str)],
    ) -> anyhow::Result<Self> {
        let pty = native_pty_system();
        let pair = pty.openpty(PtySize {
            rows: size.rows,
            cols: size.cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;
        let mut cmd = CommandBuilder::new(command);
        cmd.args(args);
        if let Some(dir) = cwd {
            cmd.cwd(dir);
        }
        // Advertise a capable terminal so TUI programs behave (env is otherwise
        // inherited from the host process, so $HOME/$PATH etc. are present).
        cmd.env("TERM", "xterm-256color");
        // Light/dark hint for programs that read $COLORFGBG instead of (or as
        // a fallback to) querying OSC 11 — agent CLIs pick their palette from
        // it, so it must match the theme active at spawn time.
        cmd.env(
            "COLORFGBG",
            crate::contrast::colorfgbg_for(crew_theme::theme().term_bg),
        );
        for (k, v) in env {
            cmd.env(k, v);
        }
        let child = pair.slave.spawn_command(cmd)?;
        // Drop the slave end so EOF propagates when the child exits.
        drop(pair.slave);

        // Spawn a reader thread: portable-pty reads are blocking. The channel is
        // bounded so a flooding child can't pile up unbounded output in memory —
        // a full queue blocks the reader (and in turn the child) until the main
        // thread drains it.
        let mut reader = pair.master.try_clone_reader()?;
        let (tx, rx) = sync_channel::<Vec<u8>>(CHANNEL_CAP);
        std::thread::spawn(move || {
            let mut buf = [0u8; 8192];
            loop {
                match std::io::Read::read(&mut reader, &mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        if tx.send(buf[..n].to_vec()).is_err() {
                            break;
                        }
                    }
                }
            }
        });

        let input = pair.master.take_writer()?;
        Ok(Self {
            core: TermCore::new(size),
            master: pair.master,
            input: std::sync::Arc::new(std::sync::Mutex::new(input)),
            rx,
            exited: false,
            pending: false,
            watch: Vec::new(),
            scan_tail: String::new(),
            hits: Vec::new(),
            child,
        })
    }
}
