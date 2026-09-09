//! `/lsp`: the language servers crew knows, and whether each will work.
//!
//! A server that is not installed fails quietly — the viewer skips its
//! diagnostics, an agent's `lsp:hover` comes back "not installed" — and a
//! silent skip looks exactly like a language nobody thought of. This is the
//! one place that says, per language, what crew would run and whether it
//! can, plus which servers this crew has alive right now.
//!
//! Read fresh each time, like `/integrations`: the table is the built-in
//! one with `~/.config/crew/lsp.json` merged over it, and what this shows is
//! what the next open or tool call will see.
use std::path::PathBuf;

#[cfg(test)]
#[path = "lspview_tests.rs"]
mod tests;

/// One server row: language, command, installed.
pub(crate) type Row = (String, String, bool);

/// The listing for `rows` and the `running` `(language, root)` pairs, as
/// viewer text.
pub(crate) fn listing(rows: &[Row], running: &[(String, PathBuf)]) -> String {
    let mut out = String::from("# lsp \u{b7} language servers\n");
    let installed = rows.iter().filter(|r| r.2).count();
    let lang_w = rows.iter().map(|r| r.0.chars().count()).max().unwrap_or(4);
    let cmd_w = rows.iter().map(|r| r.1.chars().count()).max().unwrap_or(7);
    out.push_str(&format!(
        "{} language(s) \u{b7} {installed} installed \u{b7} override: ~/.config/crew/lsp.json\n\n",
        rows.len()
    ));
    for (lang, cmd, ok) in rows {
        let state = if *ok { "installed" } else { "not installed" };
        out.push_str(&format!("  {lang:<lang_w$}  {cmd:<cmd_w$}  {state}\n"));
    }
    out.push('\n');
    if running.is_empty() {
        out.push_str("no server running in this crew\n");
    } else {
        out.push_str("running in this crew:\n");
        for (lang, root) in running {
            out.push_str(&format!("  {lang:<lang_w$}  {}\n", root.display()));
        }
    }
    out.push_str(
        "\nAgents call lsp:hover, lsp:definition, lsp:references and lsp:diagnostics \
         (read-only, no approval). The viewer marks a code file's diagnostics in the \
         margin when its server is installed; lsp = false in config turns that off.\n",
    );
    out
}

/// The rows for the live table: every configured language, with a PATH
/// lookup for its command.
fn rows() -> Vec<Row> {
    crew_lsp::servers::load()
        .into_iter()
        .map(|(lang, s)| {
            let ok = crew_lsp::servers::which(&s.command).is_some();
            (lang, s.command, ok)
        })
        .collect()
}

impl crate::app::CrewApp {
    /// `/lsp` — the server table, in the viewer.
    pub(crate) fn open_lsp_status(&mut self) {
        let text = listing(&rows(), &crew_lsp::running::list());
        let path = crate::lastout::temp_path(usize::MAX, "lsp");
        if let Err(e) = std::fs::write(&path, text) {
            self.set_status(format!("lsp: cannot write: {e}"));
            return;
        }
        let before = self.panes.len();
        self.open_view(&path.to_string_lossy());
        self.name_last_view("lsp");
        self.mark_last_view_ephemeral(before);
    }
}
