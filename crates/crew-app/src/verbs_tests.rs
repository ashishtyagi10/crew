use super::*;

use crate::suggest::menu_items;

#[test]
fn every_folded_spelling_left_the_palette_and_still_answers() {
    let names: Vec<&str> = crate::cmddefs::commands().map(|c| c.name).collect();
    for folded in FOLDED {
        let old = format!("/{folded}");
        assert!(
            !names.contains(&old.as_str()),
            "{old} is still a palette row"
        );
        assert!(crate::cmddefs::answered(&old), "{old} stopped answering");
    }
}

#[test]
fn a_verb_lists_its_subjects_and_each_row_runs() {
    let rows = menu_items("/clear ");
    let labels: Vec<&str> = rows.iter().map(|r| r.label.as_str()).collect();
    assert_eq!(labels, vec!["all", "log"]);
    assert_eq!(rows[0].fill, "/clear all");
    assert!(rows[0].submit, "a one-step subject IS the answer");
}

#[test]
fn only_the_two_step_verb_inserts_instead_of_running() {
    assert!(nested("/look", "gamma"), "a look subject opens its ladder");
    assert!(!nested("/clear", "all"), "clear all is the whole answer");
    assert_eq!(fill("/look", "gamma"), "/look gamma ");
    assert_eq!(fill("/clear", "all"), "/clear all");
}

#[test]
fn the_palette_lost_six_rows_and_gained_one() {
    let names: Vec<&str> = crate::cmddefs::commands().map(|c| c.name).collect();
    assert!(names.contains(&"/close"), "the new verb is missing");
    for verb in ["/clear", "/find", "/errors", "/look"] {
        assert!(names.contains(&verb), "{verb} must stay a row");
    }
}

#[test]
fn every_verb_offers_subjects_and_nothing_else_does() {
    for v in VERBS {
        let rows = options(v.name).unwrap_or_default();
        assert_eq!(rows.len(), v.subjects.len(), "{} lost subjects", v.name);
        assert!(rows.iter().all(|(s, d)| !s.is_empty() && !d.is_empty()));
    }
    assert!(options("/settings").is_none());
}

/// The completion side, moved here from `suggest_tests` when the family
/// folded: the verb completes, the folded spellings do not, and both run.
#[test]
fn the_verbs_complete_and_the_folded_spellings_still_answer() {
    assert_eq!(crate::suggest::suggest("/clos", &[]).as_deref(), Some("e"));
    let names: Vec<&str> = crate::suggest::matches("/cl")
        .iter()
        .map(|c| c.name)
        .collect();
    assert!(
        names.contains(&"/close") && names.contains(&"/clear"),
        "{names:?}"
    );
    for folded in ["/only", "/clearlog", "/clearall", "/closeall"] {
        assert!(!names.contains(&folded), "{folded} is still offered");
        assert!(crate::cmddefs::answered(folded), "{folded} stopped running");
    }
}

#[test]
fn the_left_column_is_one_verb_with_the_weather_under_it() {
    let rows = menu_items("/nav ");
    let labels: Vec<&str> = rows.iter().map(|r| r.label.as_str()).collect();
    assert_eq!(labels, vec!["glance", "log", "weather"]);
    assert!(
        crate::cmddefs::answered("/weather"),
        "the old spelling runs on"
    );
    let names: Vec<&str> = crate::cmddefs::commands().map(|c| c.name).collect();
    assert!(
        !names.contains(&"/weather"),
        "and is no longer a row of its own"
    );
}

#[test]
fn the_command_block_list_is_a_subject_of_the_output_it_lists() {
    let rows = menu_items("/out ");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].fill, "/out list");
    assert!(crate::cmddefs::answered("/blocks"));
}

#[test]
fn the_document_window_has_one_name_and_answers_to_two() {
    let names: Vec<&str> = crate::cmddefs::commands().map(|c| c.name).collect();
    assert!(names.contains(&"/doc") && !names.contains(&"/md"));
    assert!(
        crate::cmddefs::answered("/md"),
        "/md still opens a document"
    );
}

#[test]
fn the_viewer_owns_its_own_contents() {
    // `/diff`, `/blame` and `/log` each described themselves as opening the
    // file viewer, which is `/view`'s whole job — three rows for one command.
    let rows = menu_items("/view ");
    let labels: Vec<&str> = rows.iter().map(|r| r.label.as_str()).collect();
    assert_eq!(labels, vec!["diff", "blame", "log"]);
    assert!(rows[0].submit, "a subject IS the answer");
    assert_eq!(rows[0].fill, "/view diff");
    let names: Vec<&str> = crate::cmddefs::commands().map(|c| c.name).collect();
    assert!(names.contains(&"/view"), "the verb itself must stay a row");
    for gone in ["/diff", "/blame", "/log"] {
        assert!(!names.contains(&gone), "{gone} is still a palette row");
        assert!(crate::cmddefs::answered(gone), "{gone} stopped answering");
    }
}

/// `/find all` is where the search goes, not the search: the row fills the
/// bar and waits for the words rather than running a search for "all".
#[test]
fn find_all_waits_for_its_words() {
    assert!(nested("/find", "all"));
    assert_eq!(fill("/find", "all"), "/find all ");
}

/// The aliases the dispatcher runs are answered names: the bar must not ink
/// `/help` or `/crew` as a typo while they work.
#[test]
fn the_aliases_that_run_are_answered() {
    for name in ["/help", "/crew", "/shell", "/run"] {
        assert!(crate::cmddefs::answered(name), "{name}");
    }
}
