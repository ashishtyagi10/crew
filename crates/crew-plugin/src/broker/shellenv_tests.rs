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

#[test]
fn credentials_fill_only_what_the_process_env_lacks() {
    // Pure helper: no process env mutation, because ~1500 tests share it.
    let store = crate::credentials::Store {
        provider: Some("anthropic".into()),
        keys: [
            ("ANTHROPIC_API_KEY".to_string(), "sk-from-store".to_string()),
            ("DASHSCOPE_API_KEY".to_string(), "sk-also-store".to_string()),
            ("OPENROUTER_API_KEY".to_string(), String::new()),
        ]
        .into_iter()
        .collect(),
    };
    // ANTHROPIC is already exported non-empty; DASHSCOPE is empty in the env.
    let current = |k: &str| match k {
        "ANTHROPIC_API_KEY" => Some("sk-from-env".to_string()),
        "DASHSCOPE_API_KEY" => Some(String::new()),
        _ => None,
    };
    let imports = super::credential_imports(&store, current);
    assert_eq!(
        imports,
        vec![("DASHSCOPE_API_KEY".to_string(), "sk-also-store".to_string())],
        "an exported var wins; an empty env var is filled; an empty stored value is skipped"
    );
}

#[test]
fn credentials_never_import_a_variable_outside_vars() {
    let store = crate::credentials::Store {
        keys: [("AWS_SECRET_ACCESS_KEY".to_string(), "nope".to_string())]
            .into_iter()
            .collect(),
        ..Default::default()
    };
    assert!(super::credential_imports(&store, |_| None).is_empty());
}

/// Same contract on the broker side: `interesting` gates what the login-shell
/// pass imports, so a provider variable missing from it can never be hydrated.
#[test]
fn every_provider_variable_is_interesting() {
    for var in crate::credentials::VARS {
        assert!(interesting(var), "{var} would never be imported");
    }
    assert!(interesting("CREW_PROVIDER"));
    assert!(!interesting("HOME"), "unrelated vars stay out");
}
