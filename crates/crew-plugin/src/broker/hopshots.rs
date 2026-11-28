//! The pictures a relay hop's message names (`crew_hive::provider::images_in`).
//! A pasted screenshot arrives as the path of a temp PNG: a CLI agent opens it
//! itself, so only an API model (which sees what the request carries) gets the
//! picture attached.
use crew_hive::provider::{images_in, Image};

/// `prompt` with a note for each picture left out (too big, past the limit),
/// and the pictures `ask` names, read from the working directory. The path
/// text stays in the prompt either way.
pub(super) fn attach(api: bool, ask: &str, prompt: String) -> (String, Vec<Image>) {
    let cwd = std::env::current_dir().ok().filter(|_| api);
    let Some(cwd) = cwd else {
        return (prompt, Vec::new());
    };
    let shots = images_in(ask, &cwd);
    let prompt = shots
        .notes
        .iter()
        .fold(prompt, |p, n| format!("{p}\n\n{n}"));
    (prompt, shots.images)
}
