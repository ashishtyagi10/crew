use super::*;
use std::time::Instant;

/// A throwaway executable shell script at `d/name`, `chmod +x`'d.
fn script(d: &std::path::Path, name: &str, body: &str) -> std::path::PathBuf {
    let p = d.join(name);
    std::fs::write(&p, format!("#!/bin/sh\n{body}\n")).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    p
}

#[test]
fn bounded_shell_env_captures_a_quick_probe() {
    let dir = tempfile::tempdir().unwrap();
    let sh = script(dir.path(), "sh", "printf 'ANTHROPIC_API_KEY=sk-ant-1\\n'");
    let out = bounded_shell_env(sh.to_str().unwrap(), Duration::from_secs(3))
        .expect("a fast script must produce output before the deadline");
    assert!(out.contains("ANTHROPIC_API_KEY=sk-ant-1"));
}

#[test]
fn bounded_shell_env_kills_a_hanging_probe_within_the_timeout() {
    // A blocking rc file (stray prompt, hung network mount) must not park
    // this thread for the process lifetime — `Command::output()` alone has
    // no deadline and would wait on this forever.
    let dir = tempfile::tempdir().unwrap();
    let sh = script(dir.path(), "sh", "sleep 5");
    let start = Instant::now();
    let out = bounded_shell_env(sh.to_str().unwrap(), Duration::from_millis(150));
    let elapsed = start.elapsed();
    assert!(out.is_none(), "a hung shell must produce no env to merge");
    assert!(
        elapsed < Duration::from_secs(2),
        "must be killed near the 150ms bound, not wait out the 5s sleep: {elapsed:?}"
    );
}

#[test]
fn shell_probed_pin_is_honoured() {
    let keys: HashSet<String> = ["OPENROUTER_API_KEY".to_string()].into_iter().collect();
    assert_eq!(
        resolve(&keys, Some("openrouter")),
        Some(crew_plugin::Provider::OpenRouter)
    );
}

#[test]
fn pin_absent_from_process_env_but_present_in_probe_is_still_honoured() {
    // Simulates a GUI-launched process: this process never saw
    // CREW_PROVIDER, but the login-shell probe found it in an rc file.
    let keys: HashSet<String> = ["ANTHROPIC_API_KEY".to_string()].into_iter().collect();
    // Without the pin, auto-discovery lands on the only key present.
    assert_eq!(resolve(&keys, None), Some(crew_plugin::Provider::Anthropic));
    // With the probed pin honoured, the explicit forced provider wins
    // instead — even though no OPENROUTER_API_KEY is in the key set.
    assert_eq!(
        resolve(&keys, Some("openrouter")),
        Some(crew_plugin::Provider::OpenRouter)
    );
}

#[test]
fn key_order_auto_discovery_applies_when_there_is_no_pin() {
    let keys: HashSet<String> = ["DASHSCOPE_API_KEY".to_string()].into_iter().collect();
    assert_eq!(resolve(&keys, None), Some(crew_plugin::Provider::DashScope));
    assert_eq!(resolve(&HashSet::new(), None), None);
}

#[test]
fn merge_process_pin_wins_over_shell_pin() {
    // Process already has CREW_PROVIDER=anthropic; shell output has
    // CREW_PROVIDER=openrouter. The process value must win.
    let mut probed = Probed {
        keys: HashSet::new(),
        provider_pin: Some("anthropic".to_string()),
        openrouter_key: None,
        path: None,
    };
    let shell_env = "CREW_PROVIDER=openrouter\nOPENROUTER_API_KEY=sk-or-123\n";
    merge_shell_env(&mut probed, shell_env);
    assert_eq!(
        probed.provider_pin,
        Some("anthropic".to_string()),
        "process pin must not be overwritten"
    );
    // The key should be added though.
    assert!(probed.keys.contains("OPENROUTER_API_KEY"));
}

#[test]
fn merge_adopts_shell_pin_when_process_has_none() {
    // Process has no CREW_PROVIDER; shell has CREW_PROVIDER=openrouter.
    // The shell value must be adopted.
    let mut probed = Probed {
        keys: HashSet::new(),
        provider_pin: None,
        openrouter_key: None,
        path: None,
    };
    let shell_env = "CREW_PROVIDER=openrouter\nOPENROUTER_API_KEY=sk-or-123\n";
    merge_shell_env(&mut probed, shell_env);
    assert_eq!(
        probed.provider_pin,
        Some("openrouter".to_string()),
        "shell pin must be adopted when process has none"
    );
}

#[test]
fn merge_process_openrouter_key_wins_over_shell_key() {
    // Process already has a value (e.g. this test process's own env);
    // shell output has a different one. The process value must win — same
    // precedence as `CREW_PROVIDER` above, per the broker's shellenv rule.
    let mut probed = Probed {
        keys: HashSet::new(),
        provider_pin: None,
        openrouter_key: Some("sk-or-process".to_string()),
        path: None,
    };
    let shell_env = "OPENROUTER_API_KEY=sk-or-shell\n";
    merge_shell_env(&mut probed, shell_env);
    assert_eq!(
        probed.openrouter_key,
        Some("sk-or-process".to_string()),
        "process key must not be overwritten by the shell-probed value"
    );
}

#[test]
fn merge_adopts_shell_openrouter_key_when_process_has_none() {
    // Process never saw OPENROUTER_API_KEY (the Finder-launch case this
    // probe exists for); shell has it. The shell value must be adopted so
    // `crate::modelfetch::spawn` can actually see a key.
    let mut probed = Probed {
        keys: HashSet::new(),
        provider_pin: None,
        openrouter_key: None,
        path: None,
    };
    let shell_env = "OPENROUTER_API_KEY=sk-or-shell\n";
    merge_shell_env(&mut probed, shell_env);
    assert_eq!(
        probed.openrouter_key,
        Some("sk-or-shell".to_string()),
        "shell key must be adopted when the process has none"
    );
}

#[test]
fn merge_ignores_empty_values() {
    // Shell has CREW_PROVIDER with an empty value; should not record it.
    let mut probed = Probed {
        keys: HashSet::new(),
        provider_pin: None,
        openrouter_key: None,
        path: None,
    };
    let shell_env = "CREW_PROVIDER=\nDASHSCOPE_API_KEY=sk-aak-123\n";
    merge_shell_env(&mut probed, shell_env);
    assert_eq!(
        probed.provider_pin, None,
        "empty shell value must not overwrite process None"
    );
    // Non-empty values are still recorded.
    assert!(probed.keys.contains("DASHSCOPE_API_KEY"));
}

#[test]
fn merge_ignores_keys_outside_the_interesting_set() {
    // Shell has HOME and UNKNOWN_VAR; should ignore them.
    let mut probed = Probed {
        keys: HashSet::new(),
        provider_pin: None,
        openrouter_key: None,
        path: None,
    };
    let shell_env = "HOME=/Users/ashish\nUNKNOWN_VAR=value\nANTHROPIC_API_KEY=sk-ant-123\n";
    merge_shell_env(&mut probed, shell_env);
    assert!(
        !probed.keys.contains("HOME"),
        "HOME must not be recorded (not in KEYS)"
    );
    assert!(
        !probed.keys.contains("UNKNOWN_VAR"),
        "unknown vars must not be recorded"
    );
    assert!(probed.keys.contains("ANTHROPIC_API_KEY"));
}

#[test]
fn merge_extracts_path_from_representative_env_output() {
    let mut probed = Probed {
        keys: HashSet::new(),
        provider_pin: None,
        openrouter_key: None,
        path: None,
    };
    let shell_env = "HOME=/Users/ashish\nPATH=/usr/local/bin:/usr/bin:/bin\nSHELL=/bin/zsh\n";
    merge_shell_env(&mut probed, shell_env);
    assert_eq!(
        probed.path,
        Some("/usr/local/bin:/usr/bin:/bin".to_string())
    );
}

#[test]
fn merge_path_value_containing_equals_splits_on_the_first_one_only() {
    // A PATH entry can itself contain `=`; only the line's first `=` may
    // separate the key from the value or the rest gets truncated.
    let mut probed = Probed {
        keys: HashSet::new(),
        provider_pin: None,
        openrouter_key: None,
        path: None,
    };
    let shell_env = "PATH=/usr/bin:/opt/weird=dir:/bin\n";
    merge_shell_env(&mut probed, shell_env);
    assert_eq!(
        probed.path,
        Some("/usr/bin:/opt/weird=dir:/bin".to_string())
    );
}

#[test]
fn merge_rejects_blank_path() {
    let mut probed = Probed {
        keys: HashSet::new(),
        provider_pin: None,
        openrouter_key: None,
        path: None,
    };
    merge_shell_env(&mut probed, "PATH=   \n");
    assert_eq!(
        probed.path, None,
        "whitespace-only PATH must not be adopted"
    );
}

#[test]
fn merge_leaves_path_unset_when_env_output_has_no_path_line() {
    let mut probed = Probed {
        keys: HashSet::new(),
        provider_pin: None,
        openrouter_key: None,
        path: None,
    };
    merge_shell_env(&mut probed, "ANTHROPIC_API_KEY=sk-ant-1\n");
    assert_eq!(probed.path, None, "fallback stays intact with no PATH line");
}

#[test]
fn bounded_shell_path_captures_a_quick_probe() {
    let dir = tempfile::tempdir().unwrap();
    let sh = script(dir.path(), "sh", "printf '/usr/local/bin:/usr/bin:/bin'");
    let out = bounded_shell_path(sh.to_str().unwrap(), Duration::from_secs(3))
        .expect("a fast script must produce output before the deadline");
    assert_eq!(out, "/usr/local/bin:/usr/bin:/bin");
}

#[test]
fn adopt_fallback_path_uses_the_fallback_value_when_present() {
    // The pure decision this whole mitigation rests on: once the caller has
    // already determined the primary `-ilc env` probe produced nothing (by
    // matching `bounded_shell_env`'s `None`), the fast PATH-only fallback
    // value must be adopted — no process spawn needed to verify this.
    let mut probed = Probed {
        keys: HashSet::new(),
        provider_pin: None,
        openrouter_key: None,
        path: None,
    };
    adopt_fallback_path(&mut probed, Some("/usr/local/bin:/usr/bin:/bin"));
    assert_eq!(
        probed.path,
        Some("/usr/local/bin:/usr/bin:/bin".to_string())
    );
}

#[test]
fn adopt_fallback_path_leaves_path_unset_when_fallback_also_failed() {
    // Both shells struck out (e.g. both timed out): PATH must stay unset so
    // `effective_path` falls back to the process PATH, never panicking or
    // fabricating a value.
    let mut probed = Probed {
        keys: HashSet::new(),
        provider_pin: None,
        openrouter_key: None,
        path: None,
    };
    adopt_fallback_path(&mut probed, None);
    assert_eq!(probed.path, None);
}

#[test]
fn an_entered_key_joins_the_probed_set() {
    let mut keys: HashSet<String> = ["OPENROUTER_API_KEY".to_string()].into_iter().collect();
    let entered: HashSet<String> = ["ANTHROPIC_API_KEY".to_string()].into_iter().collect();
    merge_entered(&mut keys, &entered);
    assert!(keys.contains("ANTHROPIC_API_KEY"));
    assert!(keys.contains("OPENROUTER_API_KEY"), "probed keys survive");
}

#[test]
fn noting_a_key_makes_resolve_pick_its_provider() {
    // This is what finding #4 asks for — "note_key followed by
    // provider_now() resolves to the expected provider" — but exercised
    // through the pure seam rather than the real functions: `note_key` and
    // `provider_now` both read/write the process-global `ENTERED` (and
    // `provider_now` also reads `SHELL_PROBE`, a `OnceLock` no test may set
    // more than once), and ~1170 crew-app tests run in parallel in this
    // binary. `merge_entered` + `resolve` is exactly the logic `provider_now`
    // runs over those globals, with the globals themselves replaced by
    // plain local values.
    let mut keys: HashSet<String> = HashSet::new();
    let entered: HashSet<String> = ["DASHSCOPE_API_KEY".to_string()].into_iter().collect();
    merge_entered(&mut keys, &entered);
    assert_eq!(resolve(&keys, None), Some(crew_plugin::Provider::DashScope));
}

#[test]
fn a_stored_pin_resolves_the_provider_and_the_env_pin_still_outranks_it() {
    // Finding #4, on the pure seam (`provider_now` reads the process-global
    // `SHELL_PROBE`/`PINNED`, which ~1180 parallel tests in this binary must
    // not fight over — see `noting_a_key_makes_resolve_pick_its_provider`).
    let keys: HashSet<String> = [
        "DASHSCOPE_API_KEY".to_string(),
        "OPENROUTER_API_KEY".to_string(),
    ]
    .into_iter()
    .collect();
    // No pin anywhere: the fixed discovery order takes DashScope first.
    assert_eq!(
        resolve(&keys, resolve_pin(None, None)),
        Some(crew_plugin::Provider::DashScope)
    );
    // The stored pin — what the key popup writes, and what the broker has
    // always honoured — must move the app's answer too. Without this the user
    // saves an OpenRouter key, the app keeps resolving DashScope, the row
    // stays dim, and accepting it re-opens the same prompt forever.
    assert_eq!(
        resolve(&keys, resolve_pin(None, Some("openrouter"))),
        Some(crew_plugin::Provider::OpenRouter)
    );
    // …and an explicit CREW_PROVIDER still outranks it, exactly as in
    // `crew_plugin`'s `discover::resolve_forced`.
    assert_eq!(
        resolve(&keys, resolve_pin(Some("anthropic"), Some("openrouter"))),
        Some(crew_plugin::Provider::Anthropic)
    );
    // A blank CREW_PROVIDER is not a pin.
    assert_eq!(
        resolve_pin(Some(""), Some("openrouter")),
        Some("openrouter")
    );
    assert_eq!(resolve_pin(None, Some("")), None);
}

#[test]
fn adopt_fallback_path_rejects_blank_fallback_value() {
    // Same blank/whitespace-only guard as the primary probe's PATH= line in
    // `merge_shell_env` — a shell that "succeeds" but prints nothing useful
    // must not overwrite an unset PATH with an empty string.
    let mut probed = Probed {
        keys: HashSet::new(),
        provider_pin: None,
        openrouter_key: None,
        path: None,
    };
    adopt_fallback_path(&mut probed, Some("   "));
    assert_eq!(
        probed.path, None,
        "whitespace-only fallback must not be adopted"
    );
}

/// The probe list must cover every provider variable crew knows about. A
/// hand-kept copy drifts the moment a provider row is added, and the symptom
/// is invisible: a key sitting in the user's shell config that crew never
/// imports looks exactly like having no key at all.
#[test]
fn every_provider_variable_is_probed() {
    let probed = keys();
    for var in crew_plugin::credentials::VARS {
        assert!(probed.contains(var), "{var} is never probed for");
    }
    // …plus the two knobs that are not credentials.
    assert!(probed.contains(&"CREW_PROVIDER"));
    assert!(probed.contains(&"CREW_BROKER_MOCK_REPLY"));
}
