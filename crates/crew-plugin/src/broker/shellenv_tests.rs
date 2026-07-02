use super::*;

#[test]
fn interesting_matches_provider_keys_and_crew_knobs() {
    assert!(interesting("DASHSCOPE_API_KEY"));
    assert!(interesting("OPENROUTER_API_KEY"));
    assert!(interesting("ANTHROPIC_API_KEY"));
    assert!(interesting("CREW_PROVIDER"));
    assert!(interesting("CREW_DASHSCOPE_MODEL"));
    assert!(!interesting("PATH"));
    assert!(!interesting("HOME"));
}

#[test]
fn merge_imports_only_missing_interesting_vars() {
    let out = "PATH=/usr/bin\nDASHSCOPE_API_KEY=sk-new\n\
               OPENROUTER_API_KEY=sk-old\nCREW_PROVIDER=dashscope\n";
    // The process already has OPENROUTER_API_KEY — it must not be replaced.
    let got = merge(out, |k| k != "OPENROUTER_API_KEY");
    assert_eq!(
        got,
        vec![
            ("DASHSCOPE_API_KEY".to_string(), "sk-new".to_string()),
            ("CREW_PROVIDER".to_string(), "dashscope".to_string()),
        ]
    );
}

#[test]
fn merge_skips_empty_values_and_malformed_lines() {
    let out = "DASHSCOPE_API_KEY=\nnot a var line\nCREW_PROVIDER=dashscope";
    let got = merge(out, |_| true);
    assert_eq!(
        got,
        vec![("CREW_PROVIDER".to_string(), "dashscope".to_string())]
    );
}
