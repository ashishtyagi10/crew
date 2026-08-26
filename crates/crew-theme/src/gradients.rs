//! Named gradients — the ready-made pairs `/gradient <name>` offers.
//!
//! `poleshift` can already wear any two colours the user types. This is the
//! shelf of ones worth typing: eight pairs chosen for their *interval* rather
//! than their brightness, because brightness is not what a custom gradient
//! carries. Only the hue and chroma of these survive — every pair is re-lit to
//! the active theme's own pole lightness at draw time (see
//! [`crate::poleshift::poles`]) — so a preset is really a pair of HUES with a
//! saturation, and it lands correctly on a near-black page and a paper-white
//! one without either being tuned for.
//!
//! That is also why the swatches below are written at a mid lightness: they
//! are never drawn as written, and picking them light or dark would only
//! invite tuning a number nothing reads.
//!
//! Eight, not eighty. A list you can hold in your head is a list you pick
//! from; the freeform `#rrggbb #rrggbb` form is there for everything else.
use crate::poleshift::Poles;

/// One named pair: the name typed at `/gradient`, the two poles, and the
/// one-line description the value picker shows.
pub struct Gradient {
    pub name: &'static str,
    pub poles: Poles,
    pub about: &'static str,
}

/// The shelf, in the order the picker offers it: the cool half first (they
/// are the ones that read as "an app"), the warm half after, and the
/// colourless one last, since it is the odd one out rather than a favourite.
pub static GRADIENTS: &[Gradient] = &[
    Gradient {
        name: "aurora",
        poles: ((64, 196, 180), (150, 122, 240)),
        about: "teal into violet — cold light over a dark page",
    },
    Gradient {
        name: "tide",
        poles: ((70, 190, 220), (80, 120, 230)),
        about: "cyan into deep blue — the quietest of the eight",
    },
    Gradient {
        name: "orchid",
        poles: ((190, 120, 235), (240, 130, 175)),
        about: "violet into rose — crew's own aurora look",
    },
    Gradient {
        name: "moss",
        poles: ((130, 195, 100), (70, 185, 160)),
        about: "green into teal — a page that reads as paper",
    },
    Gradient {
        name: "ember",
        poles: ((240, 165, 70), (225, 95, 85)),
        about: "amber into red — warm, and the most awake",
    },
    Gradient {
        name: "sand",
        poles: ((225, 180, 120), (205, 135, 110)),
        about: "sand into clay — warm and nearly neutral",
    },
    Gradient {
        name: "dusk",
        poles: ((120, 130, 235), (215, 110, 190)),
        about: "indigo into magenta — evening light",
    },
    Gradient {
        name: "mono",
        poles: ((150, 150, 150), (150, 150, 150)),
        about: "no colour at all — the wash in the page's own grey",
    },
];

/// The pair a name stands for, case-insensitively. `None` for anything not on
/// the shelf, which is what sends `/gradient` on to the `#rrggbb` parser.
pub fn by_name(name: &str) -> Option<Poles> {
    let n = name.trim();
    GRADIENTS
        .iter()
        .find(|g| g.name.eq_ignore_ascii_case(n))
        .map(|g| g.poles)
}

/// The name a stored pair came from, if it is one of ours — so `/gradient`
/// can report "ember" rather than two hex codes for a gradient the user
/// picked by name.
pub fn name_of(poles: Poles) -> Option<&'static str> {
    GRADIENTS.iter().find(|g| g.poles == poles).map(|g| g.name)
}

#[cfg(test)]
#[path = "gradients_tests.rs"]
mod tests;
