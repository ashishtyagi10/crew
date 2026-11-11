use super::*;

fn tmp(name: &str) -> Option<PathBuf> {
    Some(std::env::temp_dir().join(format!("crew-restart-{name}-{}.toml", std::process::id())))
}

/// A shell in a directory that exists (`load` drops the ones that do not).
fn shell() -> SavedPane {
    SavedPane::shell(std::env::temp_dir().to_string_lossy().into_owned())
}

fn gone() -> SavedPane {
    SavedPane::shell("/no/such/crew/dir".into())
}

/// The reason this is not the quit snapshot: two shells in one directory are
/// two panes, and a restart that handed back one would have lost a pane.
#[test]
fn two_panes_in_the_same_directory_both_come_back() {
    let p = tmp("twins");
    let win = Win {
        focus: Some(1),
        panes: vec![shell(), shell(), SavedPane::todo()],
    };
    assert!(save_at(p.clone(), vec![win.clone()]));
    assert_eq!(take_at(p), vec![win]);
}

#[test]
fn the_snapshot_resumes_exactly_one_launch() {
    let p = tmp("once");
    save_at(
        p.clone(),
        vec![Win {
            focus: None,
            panes: vec![shell()],
        }],
    );
    assert_eq!(take_at(p.clone()).len(), 1);
    assert!(take_at(p).is_empty(), "a second launch must start clean");
}

/// A pane whose directory went away is dropped, and the focus follows the
/// pane it was on rather than the index it had.
#[test]
fn focus_follows_its_pane_past_a_dropped_one() {
    let p = tmp("focus");
    save_at(
        p.clone(),
        vec![Win {
            focus: Some(2),
            panes: vec![shell(), gone(), SavedPane::crew()],
        }],
    );
    let back = take_at(p.clone());
    assert_eq!(back[0].panes.len(), 2);
    assert_eq!(back[0].focus, Some(1));
    // …and focus on the dropped pane itself is simply no focus.
    save_at(
        p.clone(),
        vec![Win {
            focus: Some(0),
            panes: vec![gone(), shell()],
        }],
    );
    assert_eq!(take_at(p)[0].focus, None);
}

/// Each window keeps its own panes; one left empty is not reopened — but the
/// first stays, empty or not, because the launch already made that window.
#[test]
fn windows_stay_separate_and_empty_extras_are_dropped() {
    let p = tmp("windows");
    let one = Win {
        focus: None,
        panes: vec![],
    };
    let two = Win {
        focus: Some(0),
        panes: vec![shell()],
    };
    let three = Win {
        focus: None,
        panes: vec![gone()],
    };
    save_at(p.clone(), vec![one.clone(), two.clone(), three]);
    assert_eq!(take_at(p), vec![one, two]);
}

#[test]
fn a_fat_file_is_capped() {
    let p = tmp("fat");
    save_at(
        p.clone(),
        vec![Win {
            focus: None,
            panes: vec![shell(); 200],
        }],
    );
    assert_eq!(take_at(p)[0].panes.len(), MAX_PANES);
}

#[test]
fn a_discarded_or_missing_snapshot_resumes_nothing() {
    let p = tmp("discard");
    save_at(
        p.clone(),
        vec![Win {
            focus: None,
            panes: vec![shell()],
        }],
    );
    discard_at(p.clone());
    assert!(take_at(p).is_empty());
    assert!(take_at(None).is_empty());
}
