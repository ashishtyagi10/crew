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

/// The `/theme` (no-arg) listing: every theme's name and description, plus the
/// three rotation modes, the active one marked with `\u{25cf}`. When a mode is
/// on no fixed theme is marked (rotation owns the pick).
fn theme_list_line(current: crew_theme::ThemeId, mode: Option<crew_theme::RandomMode>) -> String {
    // The two rotation modes lead the list (then auto), ahead of the fixed
    // themes — the rotations are the headline choices.
    let modes: [(crew_theme::RandomMode, &str); 3] = [
        (
            crew_theme::RandomMode::Dark,
            "rotates dark themes every 10 min",
        ),
        (
            crew_theme::RandomMode::Light,
            "rotates light themes every 10 min",
        ),
        (
            crew_theme::RandomMode::Auto,
            "light by day, dark by night \u{2014} follows the OS",
        ),
    ];
    let mut items: Vec<String> = modes
        .into_iter()
        .map(|(m, desc)| {
            let mark = if mode == Some(m) { "\u{25cf} " } else { "" };
            format!("{mark}{} ({desc})", m.as_str())
        })
        .collect();
    items.extend(crew_theme::ALL_THEMES.iter().map(|&id| {
        let mark = if mode.is_none() && id == current {
            "\u{25cf} "
        } else {
            ""
        };
        format!("{mark}{} ({})", id.as_str(), id.describe())
    }));
    format!(
        "themes: {} \u{2014} /theme <name> to switch",
        items.join(", ")
    )
}

/// The comma-joined list of valid theme names, for the "unknown theme" echo.
/// The two rotation modes lead, then the fixed themes. The bare `random`
/// alias still parses but isn't advertised (random-dark is canonical).
fn theme_names() -> String {
    ["random-dark", "random-light", "auto"]
        .into_iter()
        .chain(crew_theme::ALL_THEMES.iter().map(|id| id.as_str()))
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
        ThemeCmd::List => theme_list_line(crew_theme::current_id(), crew_theme::mode()),
        ThemeCmd::Select(sel) => {
            crew_theme::apply_selection(sel, now_ms);
            outcome = ThemeIntercept::Switched;
            format!("theme \u{2192} {}", crew_theme::selection_label())
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
    outcome
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
            ThemeCmd::Select(crew_theme::Selection::Fixed(ThemeId::PaperLight))
        );
        assert_eq!(
            parse_theme_cmd(" crt-green "),
            ThemeCmd::Select(crew_theme::Selection::Fixed(ThemeId::CrtGreen))
        );
    }

    #[test]
    fn parse_unknown_name_is_unknown() {
        assert_eq!(parse_theme_cmd("nope"), ThemeCmd::Unknown("nope".into()));
    }

    #[test]
    fn parse_modes_and_alias() {
        assert_eq!(
            parse_theme_cmd("random"),
            ThemeCmd::Select(crew_theme::Selection::Mode(crew_theme::RandomMode::Dark))
        );
        assert_eq!(
            parse_theme_cmd("random-light"),
            ThemeCmd::Select(crew_theme::Selection::Mode(crew_theme::RandomMode::Light))
        );
        assert_eq!(
            parse_theme_cmd(" AUTO "),
            ThemeCmd::Select(crew_theme::Selection::Mode(crew_theme::RandomMode::Auto))
        );
    }

    #[test]
    fn list_line_names_every_theme_and_marks_the_current_one() {
        let line = theme_list_line(ThemeId::PaperLight, None);
        for id in crew_theme::ALL_THEMES {
            assert!(
                line.contains(id.as_str()),
                "missing {}: {line}",
                id.as_str()
            );
        }
        assert!(
            line.contains("random-dark (rotates dark themes every 10 min)"),
            "random-dark not listed: {line}"
        );
        assert!(
            line.contains("random-light (rotates light themes every 10 min)"),
            "random-light not listed: {line}"
        );
        assert!(
            line.contains("auto (light by day, dark by night \u{2014} follows the OS)"),
            "auto not listed: {line}"
        );
        assert!(
            line.contains("\u{25cf} paper-light"),
            "current theme not marked: {line}"
        );
        assert!(
            !line.contains("\u{25cf} paper-dark"),
            "wrong theme marked: {line}"
        );
        assert!(
            !line.contains("\u{25cf} random-dark"),
            "random-dark marked while off: {line}"
        );
    }

    #[test]
    fn list_line_marks_mode_and_no_fixed_theme_when_mode_is_on() {
        let line = theme_list_line(ThemeId::PaperDark, Some(crew_theme::RandomMode::Light));
        assert!(
            line.contains("\u{25cf} random-light"),
            "random-light not marked: {line}"
        );
        assert!(
            !line.contains("\u{25cf} paper-dark"),
            "fixed theme marked while a mode is on: {line}"
        );
    }

    #[test]
    fn theme_names_lists_modes_but_not_the_bare_random_alias() {
        let names = theme_names();
        assert!(names.contains("random-dark"), "{names}");
        assert!(names.contains("random-light"), "{names}");
        assert!(names.contains("auto"), "{names}");
        // The bare `random` alias is not advertised (it still parses).
        assert!(
            !names.split(", ").any(|n| n == "random"),
            "bare `random` must not be listed: {names}"
        );
    }

    #[test]
    fn intercept_distinguishes_switch_list_and_foreign_text() {
        let _g = crate::app::theme_test_guard();
        let plugin =
            crew_plugin::Plugin::spawn("sh", &["-c".to_string(), "cat >/dev/null".to_string()])
                .unwrap();
        let mut p = ChatPane::new(plugin, "crew".into());
        assert_eq!(
            intercept(&mut p, "/theme paper-dark"),
            ThemeIntercept::Switched
        );
        assert_eq!(intercept(&mut p, "/theme"), ThemeIntercept::Handled);
        assert_eq!(intercept(&mut p, "/theme nope"), ThemeIntercept::Handled);
        assert_eq!(intercept(&mut p, "hello"), ThemeIntercept::NotTheme);
    }

    #[test]
    fn switch_after_random_clears_random_mode() {
        let _g = crate::app::theme_test_guard();
        crew_theme::apply_selection(
            crew_theme::Selection::Mode(crew_theme::RandomMode::Dark),
            1_000,
        );
        assert!(crew_theme::is_random());
        crew_theme::apply_selection(
            crew_theme::Selection::Fixed(crew_theme::ThemeId::PaperDark),
            2_000,
        );
        assert!(!crew_theme::is_random());
        assert_eq!(crew_theme::current_id(), ThemeId::PaperDark);
    }
}
