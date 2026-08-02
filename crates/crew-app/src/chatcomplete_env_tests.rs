//! The env-knob half of the doc-drift guard: every `CREW_*` variable the
//! shipped source reads is either in the manual or declared internal here.
//! Split from `chatcomplete.rs`'s `doc_drift` module to keep that file
//! inside the line cap as the internal-seam list grew; the construct and
//! anchor guards stay there (`DOCS` is shared).
use super::doc_drift::DOCS;

/// `CREW_*` names in the source that are NOT user-facing knobs, and so are
/// deliberately absent from the manual. Being a declared list is the point:
/// an internal seam is a decision someone made, an undocumented knob is an
/// oversight, and without this list the two are indistinguishable.
const NOT_USER_FACING: &[&str] = &[
    // Set by the app on itself, never by a user.
    "CREW_DETACHED",
    // A Rust static in `shellenv`, not an environment variable at all.
    "CREW_INJECTED",
    // Test seams: they exist so a test can redirect a real-disk singleton
    // (or, for the OAuth device flow, a live endpoint onto a stub server).
    "CREW_PROJECT_DIR",
    "CREW_CREDENTIALS_PATH",
    "CREW_OAUTH_BASE",
    "CREW_SECURITY_BIN",
    "CREW_RESOLVE_DIR",
    "CREW_PE_DIR",
    "CREW_EE_A",
    "CREW_EE_UNSET",
    // The `crew-render` screenshot example's output directory.
    "CREW_SHOT_DIR",
];

/// Every `CREW_*` knob the shipped source reads is either in the manual or
/// declared internal above. Six were neither — `CREW_HTTP_TIMEOUT_MS`,
/// `CREW_STREAM_TEXT`, the three plugin-path overrides and `CREW_PANE` —
/// which is the provider-table gap again in a different shape: a knob
/// nobody is told about is a knob nobody turns.
#[test]
fn every_env_knob_is_documented_or_declared_internal() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut docs = String::new();
    for rel in DOCS {
        if let Ok(s) = std::fs::read_to_string(root.join("crates/crew-app").join(rel)) {
            docs.push_str(&s);
        }
    }
    if docs.is_empty() {
        return; // docs not shipped in this build context
    }
    for name in env_names(&root.join("crates")) {
        assert!(
            docs.contains(&name) || NOT_USER_FACING.contains(&name.as_str()),
            "{name} is read by the source but appears in no doc and is not \
             declared internal"
        );
    }
}

/// Every `CREW_[A-Z0-9_]+` token in the crates' `src/` trees, skipping
/// `*_tests.rs` — a fixture variable is not a shipped knob.
fn env_names(crates: &std::path::Path) -> Vec<String> {
    let mut out = Vec::new();
    let mut stack = vec![crates.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in entries.filter_map(Result::ok) {
            let p = e.path();
            let name = e.file_name().to_string_lossy().into_owned();
            if p.is_dir() {
                if name != "target" && name != "tests" {
                    stack.push(p);
                }
            } else if name.ends_with(".rs") && !name.ends_with("_tests.rs") {
                if let Ok(src) = std::fs::read_to_string(&p) {
                    scan_env_names(&src, &mut out);
                }
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

fn scan_env_names(src: &str, out: &mut Vec<String>) {
    let chars: Vec<char> = src.chars().collect();
    let mut i = 0;
    while i + 5 <= chars.len() {
        if chars[i..].starts_with(&['C', 'R', 'E', 'W', '_'])
            && (i == 0 || !chars[i - 1].is_ascii_alphanumeric() && chars[i - 1] != '_')
        {
            let name: String = chars[i..]
                .iter()
                .take_while(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || **c == '_')
                .collect();
            // A bare `CREW_` prefix constant is not a variable name.
            if name.len() > 5 && !name.ends_with('_') {
                out.push(name);
            }
        }
        i += 1;
    }
}
