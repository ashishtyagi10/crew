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

/// The `/theme` (no-arg) listing: the three themes (`dark`, `light`, `crt`),
/// each a rotation over its own palette pool, the active one marked with
/// `\u{25cf}`. The individual palettes are pool members, not list entries.
fn theme_list_line(mode: Option<crew_theme::RandomMode>) -> String {
    let items: Vec<String> = crew_theme::THEME_MODES
        .iter()
        .map(|&m| {
            let mark = if mode == Some(m) { "\u{25cf} " } else { "" };
            format!("{mark}{} ({})", m.as_str(), m.describe())
        })
        .collect();
    format!(
        "themes: {} \u{2014} /theme <name> to switch",
        items.join(", ")
    )
}

/// The comma-joined list of valid theme names, for the "unknown theme" echo:
/// the three canonical modes. Legacy names (`random-*`, `auto`, the palette
/// names) still parse but aren't advertised.
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
            format!("theme \u{2192} {}", crew_theme::selection_label())
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
        // The three canonical names.
        assert_eq!(
            parse_theme_cmd("dark"),
            ThemeCmd::Select(crew_theme::Selection::Mode(crew_theme::RandomMode::Dark))
        );
        assert_eq!(
            parse_theme_cmd("light"),
            ThemeCmd::Select(crew_theme::Selection::Mode(crew_theme::RandomMode::Light))
        );
        assert_eq!(
            parse_theme_cmd(" CRT "),
            ThemeCmd::Select(crew_theme::Selection::Mode(crew_theme::RandomMode::Crt))
        );
        // Pre-consolidation names still parse.
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
    fn list_line_names_the_three_modes_and_marks_the_active_one() {
        let line = theme_list_line(None);
        for m in crew_theme::THEME_MODES {
            assert!(line.contains(m.as_str()), "missing {}: {line}", m.as_str());
            assert!(line.contains(m.describe()), "missing desc: {line}");
        }
        // Nothing is marked while no mode is on.
        assert!(
            !line.contains("\u{25cf}"),
            "nothing should be marked: {line}"
        );
        // The pooled palettes are not listed as entries.
        assert!(
            !line.contains("paper-dark") && !line.contains("crt-green"),
            "individual palettes must not be listed: {line}"
        );
    }

    #[test]
    fn list_line_marks_the_active_mode() {
        let line = theme_list_line(Some(crew_theme::RandomMode::Light));
        assert!(
            line.contains("\u{25cf} light"),
            "light mode not marked: {line}"
        );
        assert!(!line.contains("\u{25cf} dark"), "wrong mode marked: {line}");
    }

    #[test]
    fn theme_names_lists_the_three_modes() {
        let names = theme_names();
        for m in crew_theme::THEME_MODES {
            assert!(
                names.contains(m.as_str()),
                "missing {}: {names}",
                m.as_str()
            );
        }
        // Legacy/pooled names are not advertised (they still parse).
        assert!(
            !names.contains("random-dark") && !names.contains("paper-dark"),
            "legacy names must not be listed: {names}"
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
