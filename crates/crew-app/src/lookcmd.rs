//! `/look` — one verb for every knob that changes how crew looks.
//!
//! ```text
//! /look                 the subjects, and what each is set to
//! /look gamma           the ladder for one subject
//! /look gamma medium    set it
//! ```
//!
//! WHY: the input bar offered sixty-seven commands, and fifteen of them were
//! appearance — `/gamma`, `/grain`, `/smooth`, `/leading`, `/shapes` and the
//! rest. Fifteen rows you scroll past to reach `/find`, fifteen names to
//! remember, and no way to see that they are one family. The knobs are good;
//! the flat list was the problem. They are subjects now, under one verb, and
//! the palette walks them in two steps: `/look ` lists the subjects, `/look
//! gamma ` lists that subject's ladder with the current value marked.
//!
//! The picker plumbing is `crate::verbs`, which owns every folded verb —
//! this file owns the `/look` family: its subjects, and the command each one
//! forwards to.
//!
//! The old spellings still run. Muscle memory is a feature, `/theme dark` is
//! in every doc and script anyone has written, and the point of the diet is
//! to shrink what you have to KNOW — which is the palette — not to punish
//! what you already typed.
use crate::app::CrewApp;

/// The subjects, in the order the picker lists them: what you reach for
/// first at the top, the fine typography knobs below it.
pub(crate) const SUBJECTS: &[(&str, &str)] = &[
    ("theme", "the palette — pick from the list"),
    ("font", "font size (<n>) or rotation (random)"),
    ("weight", "text weight — thicker or lighter"),
    ("leading", "line spacing: air between rows"),
    ("density", "how tightly the canvas packs"),
    ("motion", "how much crew moves"),
    ("contrast", "the WCAG floor every derived colour is held to"),
    ("shapes", "say it with a shape as well as a colour"),
    ("crt", "the CRT tube look"),
    ("gradient", "how far the canvas colour breathes"),
    ("opacity", "how much desktop shows through"),
    ("grain", "paper grain — newsprint texture"),
    ("smooth", "font smoothing — stem darkening"),
    ("gamma", "text gamma — ink the encoded blend eats"),
    ("invisibles", "tabs, trailing spaces and CRs in the viewer"),
];

pub(crate) fn is_subject(name: &str) -> bool {
    SUBJECTS.iter().any(|(s, _)| *s == name)
}

/// Read a typed command and argument as the picker needs them:
/// `(command to look values up under, the argument, the text a row fills)`.
///
/// `/look gamma medium` is `/gamma`'s value picker wearing `/look gamma`'s
/// name — one table of ladders, reached by two spellings, so a subject can
/// never drift from the command it stands for. Anything that is not a
/// subject yet (`/look gam`) stays on `/look`, which lists subjects.
pub(crate) fn canon(cmd: &str, arg: &str) -> (String, String, String) {
    let arg = arg.trim_start().to_lowercase();
    if cmd == "/look" {
        if let Some((head, rest)) = arg.split_once(' ') {
            if is_subject(head) {
                return (
                    format!("/{head}"),
                    rest.to_string(),
                    format!("/look {head}"),
                );
            }
        }
    }
    (cmd.to_string(), arg, cmd.to_string())
}

/// The command whose CURRENT value the bar should mark for `text` — the
/// subject's own command under `/look`, the first word otherwise.
pub(crate) fn current_key(text: &str) -> String {
    let mut words = text.split_whitespace();
    let head = words.next().unwrap_or_default();
    match (head, words.next()) {
        ("/look", Some(sub)) if is_subject(sub) => format!("/{sub}"),
        _ => head.to_string(),
    }
}

/// `/look` with no subject: what there is, in one line.
fn legend() -> String {
    let names: Vec<&str> = SUBJECTS.iter().map(|(s, _)| *s).collect();
    format!("/look <subject> [value] — {}", names.join(", "))
}

impl CrewApp {
    /// `/look`, `/look <subject>`, `/look <subject> <value>`.
    pub(crate) fn look_command(&mut self, arg: &str) {
        let arg = arg.trim();
        let (subject, value) = match arg.split_once(char::is_whitespace) {
            Some((s, v)) => (s, v.trim()),
            None => (arg, ""),
        };
        match subject {
            "" => self.set_status(legend()),
            "theme" => self.set_theme_cmd(value),
            "font" => self.set_font_cmd(value),
            "weight" => self.weight_command(value),
            "leading" => self.leading_command(value),
            "density" => self.density_command(value),
            "motion" => self.motion_command(value),
            "contrast" => self.contrast_command(value),
            "shapes" => self.shapes_command(value),
            "crt" => self.crt_command(value),
            "gradient" => self.gradient_command(value),
            "opacity" => self.opacity_command(value),
            "grain" => self.grain_command(value),
            "smooth" => self.smooth_command(value),
            "gamma" => self.gamma_command(value),
            "invisibles" => self.invisibles_command(value),
            other => self.set_status(format!(
                "/look has no \u{201c}{other}\u{201d} \u{2014} {}",
                legend()
            )),
        }
    }
}

#[cfg(test)]
#[path = "lookcmd_tests.rs"]
mod tests;
