use portable_pty::CommandBuilder;

use super::{pick, settle};

/// A command with no locale of its own, whatever the test process has.
fn bare() -> CommandBuilder {
    let mut c = CommandBuilder::new("sh");
    for k in ["LANG", "LC_ALL", "LC_CTYPE"] {
        c.env_remove(k);
    }
    c
}

#[test]
fn the_systems_language_becomes_a_utf8_locale_when_installed() {
    assert_eq!(pick(Some("en_GB"), |_| true), "en_GB.UTF-8");
    assert_eq!(
        pick(Some("de_DE@calendar=gregorian"), |l| l == "de_DE.UTF-8"),
        "de_DE.UTF-8"
    );
    assert_eq!(pick(Some("xx_YY"), |_| false), super::FALLBACK);
    assert_eq!(pick(None, |_| true), super::FALLBACK);
    assert_eq!(pick(Some(" "), |_| true), super::FALLBACK);
}

#[cfg(unix)]
#[test]
fn a_pane_with_no_locale_gets_utf8_and_a_chosen_one_is_kept() {
    let mut c = bare();
    settle(&mut c);
    let lang = c
        .get_env("LANG")
        .and_then(|v| v.to_str())
        .unwrap_or_default();
    assert!(lang.ends_with(".UTF-8"), "{lang:?}");
    for k in ["LANG", "LC_ALL", "LC_CTYPE"] {
        let mut c = bare();
        c.env(k, "fr_FR.UTF-8");
        settle(&mut c);
        let lang = c.get_env("LANG").and_then(|v| v.to_str());
        let want = (k == "LANG").then_some("fr_FR.UTF-8");
        assert_eq!(lang, want, "{k} set: LANG left as it was");
    }
}

/// Through a real pty: the shell reports a UTF-8 character map. Proves the
/// fix only when the test process itself has no locale:
/// `env -u LANG -u LC_ALL -u LC_CTYPE cargo test -p crew-term locale`.
#[cfg(unix)]
#[test]
fn a_shell_in_a_pane_reads_a_utf8_charmap() {
    use crate::model::TermModel;
    use std::time::{Duration, Instant};
    let size = crate::model::GridSize { cols: 40, rows: 5 };
    let args = ["-c".to_string(), "locale charmap; sleep 1".to_string()];
    let mut term = crate::pty::PtyTerm::spawn_args(size, "sh", &args).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut seen = String::new();
    while Instant::now() < deadline && !seen.contains("UTF-8") {
        term.try_read();
        seen = term.cells(true).iter().map(|c| c.c).collect();
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(seen.contains("UTF-8"), "{seen}");
}
