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
    /// Enter random-rotation mode (rotates every 10 minutes).
    Random,
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
    if arg.eq_ignore_ascii_case("random") {
        return ThemeCmd::Random;
    }
    match crew_theme::ThemeId::from_name(arg) {
        Some(id) => ThemeCmd::Switch(id),
        None => ThemeCmd::Unknown(arg.to_string()),
    }
}

/// The `/theme` (no-arg) listing: every theme's name and description, plus the
/// `random` rotation mode, the active one marked with `\u{25cf}`. When
/// `random` is true no fixed theme is marked (rotation owns the pick).
fn theme_list_line(current: crew_theme::ThemeId, random: bool) -> String {
    let mut items: Vec<String> = crew_theme::ALL_THEMES
        .iter()
        .map(|&id| {
            let mark = if !random && id == current {
                "\u{25cf} "
            } else {
                ""
            };
            format!("{mark}{} ({})", id.as_str(), id.describe())
        })
        .collect();
    let random_mark = if random { "\u{25cf} " } else { "" };
    items.push(format!("{random_mark}random (rotates every 10 min)"));
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
        .chain(std::iter::once("random"))
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
    let now_ms = chrono::Local::now().timestamp_millis() as u64;
    let note = match parse_theme_cmd(arg) {
        ThemeCmd::List => theme_list_line(crew_theme::current_id(), crew_theme::is_random()),
        ThemeCmd::Random => {
            crew_theme::set_random(true, now_ms);
            "theme \u{2192} random (rotates every 10 min)".to_string()
        }
        ThemeCmd::Switch(id) => {
            crew_theme::set_random(false, now_ms);
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
    fn parse_random_is_case_insensitive() {
        assert_eq!(parse_theme_cmd("random"), ThemeCmd::Random);
        assert_eq!(parse_theme_cmd("RANDOM"), ThemeCmd::Random);
        assert_eq!(parse_theme_cmd(" Random "), ThemeCmd::Random);
    }

    #[test]
    fn list_line_names_every_theme_and_marks_the_current_one() {
        let line = theme_list_line(ThemeId::PaperLight, false);
        for id in crew_theme::ALL_THEMES {
            assert!(
                line.contains(id.as_str()),
                "missing {}: {line}",
                id.as_str()
            );
        }
        assert!(line.contains("random"), "random not listed: {line}");
        assert!(
            line.contains("\u{25cf} paper-light"),
            "current theme not marked: {line}"
        );
        assert!(
            !line.contains("\u{25cf} paper-dark"),
            "wrong theme marked: {line}"
        );
        assert!(
            !line.contains("\u{25cf} random"),
            "random marked while off: {line}"
        );
    }

    #[test]
    fn list_line_marks_random_and_no_fixed_theme_when_random_is_on() {
        let line = theme_list_line(ThemeId::PaperDark, true);
        assert!(
            line.contains("\u{25cf} random"),
            "random not marked: {line}"
        );
        assert!(
            !line.contains("\u{25cf} paper-dark"),
            "fixed theme marked while random is on: {line}"
        );
    }

    #[test]
    fn theme_names_includes_random() {
        assert!(theme_names().contains("random"));
    }

    #[test]
    fn switch_after_random_clears_random_mode() {
        let _g = crate::app::theme_test_guard();
        crew_theme::set_random(true, 1_000);
        assert!(crew_theme::is_random());
        crew_theme::set_random(false, 2_000);
        crew_theme::set_theme(crew_theme::ThemeId::PaperDark);
        assert!(!crew_theme::is_random());
        assert_eq!(crew_theme::current_id(), ThemeId::PaperDark);
    }
}
