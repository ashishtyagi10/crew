//! `/theme`: a pane-local composer command (opencode-style) that lists or
//! switches crew's color theme without leaving the crew pane. Handled
//! app-side, like `/export` — the broker never sees it. Reuses crew-theme's
//! live-switchable global (also bound to `Ctrl+Shift+L`), so the effect is
//! immediate and visible in every pane, not just this one.
use crate::chat::ChatPane;
use crate::chatlayout::Message;

/// What a `/theme <arg>` invocation resolves to, before any side effect runs
/// — kept separate from `intercept` so it's trivially testable without a
/// `ChatPane`.
#[derive(Debug, PartialEq, Eq)]
enum ThemeCmd {
    /// No argument: list every theme, marking the current one.
    List,
    /// A recognized theme name or rotation mode: apply it.
    Select(crew_theme::Selection),
    /// An unrecognized name, kept verbatim for the error echo.
    Unknown(String),
}

/// Parse the text after `/theme` (already trimmed of the leading command).
fn parse_theme_cmd(arg: &str) -> ThemeCmd {
    let arg = arg.trim();
    if arg.is_empty() {
        return ThemeCmd::List;
    }
    match crew_theme::parse_selection(arg) {
        Some(sel) => ThemeCmd::Select(sel),
        None => ThemeCmd::Unknown(arg.to_string()),
    }
}

/// The `/theme` (no-arg) listing: the four themes (`dark`, `light`, `crt`,
/// `auto`), each a rotation over its own palette pool — auto's follows the OS
/// appearance — the active one marked with `\u{25cf}`. The individual
/// palettes are pool members, not list entries.
fn theme_list_line(mode: Option<crew_theme::RandomMode>) -> String {
    let items: Vec<String> = crew_theme::THEME_MODES
        .iter()
        .map(|&m| {
            let mark = if mode == Some(m) { "\u{25cf} " } else { "" };
            format!("{mark}{} ({})", m.as_str(), m.describe())
        })
        .collect();
    let auto = if mode == Some(crew_theme::RandomMode::Auto) {
        format!("\n{}", crate::themereport::live_report())
    } else {
        String::new()
    };
    format!(
        "themes: {} \u{2014} /theme <name> to switch{auto}",
        items.join(", ")
    )
}

/// The comma-joined list of valid theme names, for the "unknown theme" echo:
/// the four canonical modes. Legacy names (`random-*`, the palette names)
/// still parse but aren't advertised.
fn theme_names() -> String {
    crew_theme::THEME_MODES
        .iter()
        .map(|m| m.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

/// What `intercept` did with a composer submission: not a `/theme` command
/// at all (send it to the broker), answered locally with no theme change, or
/// switched the live theme — the app must persist that switch to config, or
/// it silently reverts on restart (and a fixed pick would kill a saved
/// rotation mode until then).
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ThemeIntercept {
    NotTheme,
    Handled,
    Switched,
}

/// Intercept composer submissions the pane answers locally. Anything but
/// `NotTheme` means `text` was consumed (nothing should be sent to the
/// broker); `Switched` additionally asks the app to persist the selection.
pub(crate) fn intercept(pane: &mut ChatPane, text: &str) -> ThemeIntercept {
    let trimmed = text.trim();
    if trimmed != "/theme" && !trimmed.starts_with("/theme ") {
        return ThemeIntercept::NotTheme;
    }
    let arg = trimmed.strip_prefix("/theme").unwrap_or("");
    let now_ms = chrono::Local::now().timestamp_millis() as u64;
    let mut outcome = ThemeIntercept::Handled;
    let note = match parse_theme_cmd(arg) {
        ThemeCmd::List => theme_list_line(crew_theme::mode()),
        ThemeCmd::Select(sel) => {
            crew_theme::apply_selection(sel, now_ms);
            outcome = ThemeIntercept::Switched;
            match crew_theme::mode() {
                // `auto` names the half it just served (and the dormant one);
                // every other selection is its own whole story.
                Some(crew_theme::RandomMode::Auto) => crate::themereport::live_report(),
                _ => format!("theme \u{2192} {}", crew_theme::selection_label()),
            }
        }
        ThemeCmd::Unknown(name) => {
            format!("unknown theme '{name}' \u{2014} try: {}", theme_names())
        }
    };
    let ts = chrono::Local::now().timestamp_millis().to_string();
    pane.messages.push(Message {
        sender: "agent smith".into(),
        text: note,
        ts,
        meta: String::new(),
        usage: None,
        expanded: false,
    });
    outcome
}

#[cfg(test)]
#[path = "chattheme_tests.rs"]
mod tests;
