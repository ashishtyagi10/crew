//! How the pane words the broker's spend-cap question, which rides the
//! approval channel (id `cap{N}`) but is not a tool call.
//!
//! WHY apart from `chatapprove`: it drew "allow this?" and a tool's yes/no
//! over "spent $10 of the $10 cap", which asks the user to allow a tool that
//! does not exist. Money has its own words; the broker's question already
//! carries its keys, so none are added here.

/// Whether approval `id` is the spend cap's (`spendcap::put` mints `cap{N}`;
/// tool approvals are `a{N}`-style ids that never start so).
pub(crate) fn is_spend(id: &str) -> bool {
    id.starts_with("cap")
}

/// The transcript line when a question arrives.
pub(crate) fn asked(id: &str, question: &str) -> String {
    match is_spend(id) {
        true => format!("$ {question}"),
        false => {
            format!("\u{26a0} allow this? {question} \u{2014} enter allows \u{00b7} esc refuses")
        }
    }
}

/// The transcript line once it is answered.
pub(crate) fn answered(id: &str, question: &str, granted: bool) -> String {
    match (is_spend(id), granted) {
        (true, true) => "continuing past the spend cap".to_string(),
        (true, false) => "stopping at the spend cap".to_string(),
        (false, true) => format!("allowed \u{2014} {question}"),
        (false, false) => format!("refused \u{2014} {question}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_spend_question_is_worded_as_money_not_as_a_tool() {
        let q = "spent $10.40 of the $10 cap \u{2014} Enter/y continues (+$10) \u{b7} Esc/n stops";
        let line = asked("cap3", q);
        assert_eq!(line, format!("$ {q}"));
        assert!(!line.contains("allow this?"), "{line}");
        assert_eq!(answered("cap3", q, true), "continuing past the spend cap");
        assert_eq!(answered("cap3", q, false), "stopping at the spend cap");
    }

    #[test]
    fn a_tool_question_keeps_its_words() {
        assert!(asked("a1", "edit a.rs").starts_with("\u{26a0} allow this? edit a.rs"));
        assert_eq!(
            answered("a1", "edit a.rs", true),
            "allowed \u{2014} edit a.rs"
        );
        assert!(!is_spend("a1"));
    }
}
