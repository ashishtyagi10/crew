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
    /// A recognized theme name: switch to it.
    Switch(crew_theme::ThemeId),
    /// An unrecognized name, kept verbatim for the error echo.
    Unknown(String),
}

/// Parse the text after `/theme` (already trimmed of the leading command).
fn parse_theme_cmd(arg: &str) -> ThemeCmd {
    let arg = arg.trim();
    if arg.is_empty() {
        return ThemeCmd::List;
    }
    match crew_theme::ThemeId::from_name(arg) {
        Some(id) => ThemeCmd::Switch(id),
        None => ThemeCmd::Unknown(arg.to_string()),
    }
}

/// The `/theme` (no-arg) listing: every theme's name and description, the
/// current one marked with `\u{25cf}`.
fn theme_list_line(current: crew_theme::ThemeId) -> String {
    let items: Vec<String> = crew_theme::ALL_THEMES
        .iter()
        .map(|&id| {
            let mark = if id == current { "\u{25cf} " } else { "" };
            format!("{mark}{} ({})", id.as_str(), id.describe())
        })
        .collect();
    format!(
        "themes: {} \u{2014} /theme <name> to switch",
        items.join(", ")
    )
}

/// The comma-joined list of valid theme names, for the "unknown theme" echo.
fn theme_names() -> String {
    crew_theme::ALL_THEMES
        .iter()
        .map(|id| id.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

/// Intercept composer submissions the pane answers locally. Returns `true`
/// when `text` was consumed (nothing should be sent to the broker).
pub(crate) fn intercept(pane: &mut ChatPane, text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed != "/theme" && !trimmed.starts_with("/theme ") {
        return false;
    }
    let arg = trimmed.strip_prefix("/theme").unwrap_or("");
    let note = match parse_theme_cmd(arg) {
        ThemeCmd::List => theme_list_line(crew_theme::current_id()),
        ThemeCmd::Switch(id) => {
            crew_theme::set_theme(id);
            format!("theme \u{2192} {}", id.as_str())
        }
        ThemeCmd::Unknown(name) => {
            format!("unknown theme '{name}' \u{2014} try: {}", theme_names())
        }
    };
    let ts = chrono::Local::now().timestamp_millis().to_string();
    pane.messages.push(Message {
        sender: "crew".into(),
        text: note,
        ts,
        meta: String::new(),
    });
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crew_theme::ThemeId;

    #[test]
    fn parse_no_arg_lists() {
        assert_eq!(parse_theme_cmd(""), ThemeCmd::List);
        assert_eq!(parse_theme_cmd("   "), ThemeCmd::List);
    }

    #[test]
    fn parse_known_name_switches() {
        assert_eq!(
            parse_theme_cmd("paper-light"),
            ThemeCmd::Switch(ThemeId::PaperLight)
        );
        assert_eq!(
            parse_theme_cmd(" crt-green "),
            ThemeCmd::Switch(ThemeId::CrtGreen)
        );
    }

    #[test]
    fn parse_unknown_name_is_unknown() {
        assert_eq!(parse_theme_cmd("nope"), ThemeCmd::Unknown("nope".into()));
    }

    #[test]
    fn list_line_names_every_theme_and_marks_the_current_one() {
        let line = theme_list_line(ThemeId::PaperLight);
        for id in crew_theme::ALL_THEMES {
            assert!(
                line.contains(id.as_str()),
                "missing {}: {line}",
                id.as_str()
            );
        }
        assert!(
            line.contains("\u{25cf} paper-light"),
            "current theme not marked: {line}"
        );
        assert!(
            !line.contains("\u{25cf} paper-dark"),
            "wrong theme marked: {line}"
        );
    }
}
