//! The `/font` pool's membership rule, tested on its own: which installed
//! families crew will auto-select from. Lives outside `fontcmd_tests.rs`,
//! which is at its line cap.
use crate::fontcmd::allowed_pool;

#[test]
fn the_pool_keeps_a_face_in_whatever_spelling_the_machine_has_it() {
    // The allowlist names `Comic Mono`; this machine has `ComicMono Nerd Font
    // Mono`. Matched by name, the face was in the pool only because somebody
    // had written that spelling down too.
    let installed: Vec<String> = [
        "ComicMono Nerd Font Mono",
        "JetBrainsMono NFM",
        "Lilex Nerd Font",
        "Hactor",
    ]
    .map(String::from)
    .to_vec();
    let pool = allowed_pool(installed);
    assert_eq!(
        pool,
        vec![
            "ComicMono Nerd Font Mono".to_string(),
            "JetBrainsMono NFM".to_string(),
            "Lilex Nerd Font".to_string(),
        ],
        "Hactor is not allowlisted; the other three are, under other names"
    );
}

#[test]
fn a_face_nobody_allowlisted_is_still_kept_out() {
    let pool = allowed_pool(["IntoneMono Nerd Font Mono".to_string(), "MonoLisa".into()].to_vec());
    assert_eq!(pool, vec!["MonoLisa".to_string()], "Intel One Mono is out");
}
