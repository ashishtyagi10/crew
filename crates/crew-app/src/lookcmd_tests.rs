use super::*;

#[test]
fn the_palette_offers_one_look_row_where_it_offered_fifteen() {
    let names: Vec<&str> = crate::cmddefs::commands().map(|c| c.name).collect();
    assert!(names.contains(&"/look"));
    for (s, _) in SUBJECTS {
        let old = format!("/{s}");
        assert!(
            !names.contains(&old.as_str()),
            "{old} is still a palette row of its own"
        );
    }
}

#[test]
fn a_subject_reads_its_values_from_the_command_it_stands_for() {
    // `/look gamma medium` and `/gamma medium` are one ladder: the picker
    // looks values up under the old name, and fills the new spelling.
    let (cmd, arg, prefix) = canon("/look", "gamma med");
    assert_eq!(
        (cmd.as_str(), arg.as_str(), prefix.as_str()),
        ("/gamma", "med", "/look gamma")
    );
    let rows = crate::suggest::menu_items("/look gamma me");
    let labels: Vec<&str> = rows.iter().map(|r| r.label.as_str()).collect();
    assert_eq!(labels, vec!["medium"], "the ladder did not come through");
    assert_eq!(rows[0].fill, "/look gamma medium");
    assert!(rows[0].submit, "picking a value should run it");
}

#[test]
fn a_half_typed_subject_still_lists_subjects() {
    let (cmd, arg, prefix) = canon("/look", "gam");
    assert_eq!(
        (cmd.as_str(), arg.as_str(), prefix.as_str()),
        ("/look", "gam", "/look")
    );
    let rows = crate::suggest::menu_items("/look gam");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].label, "gamma");
    assert_eq!(
        rows[0].fill, "/look gamma ",
        "a subject must open its own picker"
    );
    assert!(!rows[0].submit, "a subject is a step, not a command to run");
}

#[test]
fn every_subject_is_a_step_and_every_value_is_a_choice() {
    assert!(nested("/look", "grain"));
    assert!(!nested("/look", "grainy"));
    assert!(!nested("/look grain", "medium"));
    assert!(
        !nested("/grain", "medium"),
        "the old spelling has no second step"
    );
}

#[test]
fn the_bar_marks_the_value_a_subject_is_already_on() {
    assert_eq!(current_key("/look gamma medium"), "/gamma");
    assert_eq!(current_key("/look gamma"), "/gamma");
    assert_eq!(current_key("/look nonsense"), "/look");
    assert_eq!(current_key("/theme dark"), "/theme");
}

#[test]
fn every_subject_names_a_command_the_dispatcher_still_answers() {
    // The subjects forward to the old arms; a subject whose arm was renamed
    // would be a dead row in the picker, which is what this catches.
    let src = include_str!("dispatch.rs");
    let args = include_str!("dispatchargs.rs");
    for (s, _) in SUBJECTS {
        assert!(
            src.contains(&format!("\"{s}\" =>")) || args.contains(&format!("\"{s} \"")),
            "/look {s} forwards to an arm the dispatcher no longer has"
        );
    }
}

#[test]
fn a_subject_nobody_has_says_what_there_is() {
    let names: Vec<&str> = SUBJECTS.iter().map(|(s, _)| *s).collect();
    let legend = legend();
    for n in names {
        assert!(legend.contains(n), "{n} is missing from the legend");
    }
}

/// The palette's own steps, moved here from `suggest_tests` when the family
/// folded: `/look` expands rather than running, and `/font` still answers
/// though the palette no longer offers it.
#[test]
fn the_look_row_opens_the_subject_list_and_the_old_names_still_answer() {
    assert_eq!(crate::suggest::suggest("/loo", &[]).as_deref(), Some("k"));
    let names: Vec<&str> = crate::suggest::matches("/lo")
        .iter()
        .map(|c| c.name)
        .collect();
    assert!(names.contains(&"/look") && !names.contains(&"/font"));
    assert!(
        crate::cmddefs::answered("/font"),
        "a folded name still runs"
    );
    let look = crate::suggest::menu_items("/look")
        .into_iter()
        .find(|m| m.label == "/look")
        .expect("/look in palette");
    assert_eq!(look.fill, "/look ");
    assert!(!look.submit, "the verb expands, it does not run");
}
