//! The `/logout` picker: the broker's signed-in rows as a popup — and the
//! sign-in row wording the `/model` picker shares (`state`).
//!
//! Bare `/logout` used to answer with a text list and wait for a name typed
//! back. The broker now sends the rows (`PluginEvent::SignOut`) and this
//! turns them into the composer popup every other pick in crew uses:
//! arrows, Enter, Esc. A pick submits `/logout <name>`, and the broker
//! removes the grant it already could. (Its twin, the `/login` picker,
//! retired: the sign-in rows lead the `/model` picker — `modelsignin` —
//! so there is one front door for "who serves".)
use crew_plugin::SignInOption;

use crate::chatpalette::{Kind, PaletteState};
use crate::suggest::MenuItem;

/// The popup card's legend.
pub(crate) const LEGEND: &str = "sign out";

fn header(label: &str) -> MenuItem {
    MenuItem {
        label: label.to_string(),
        header: true,
        ..Default::default()
    }
}

/// What a sign-in row says after its name (the model picker's sign-in
/// section reads it).
pub(crate) fn state(o: &SignInOption) -> String {
    match (o.signed_in, o.key_present, &o.install) {
        (true, _, _) if o.device => "\u{2713} signed in \u{00b7} pick to sign in again".into(),
        (true, _, _) => "\u{2713} signed in through its CLI".into(),
        (false, _, Some(install)) => format!("not installed \u{00b7} `{install}`"),
        (false, true, None) if o.device => "key present \u{00b7} sign in with OAuth instead".into(),
        (false, _, None) => match &o.login {
            Some(login) => format!("signs in through its CLI \u{00b7} run `{login}`"),
            None => "sign in with OAuth".into(),
        },
    }
}

/// The `/logout` rows: the grants crew holds (a pick removes one; a key,
/// if present, serves again), then the CLI-owned sign-ins, dimmed — crew
/// never touches a vendor's store, so those say what to run instead.
pub(crate) fn items_out(options: &[SignInOption]) -> Vec<MenuItem> {
    let mut out = vec![header("stored grants \u{00b7} pick to remove")];
    for o in options.iter().filter(|o| o.device) {
        out.push(MenuItem {
            label: o.name.clone(),
            desc: if o.key_present {
                "\u{2713} signed in \u{00b7} the key serves once removed".into()
            } else {
                "\u{2713} signed in".into()
            },
            fill: o.name.clone(),
            submit: true,
            ..Default::default()
        });
    }
    let cli: Vec<&SignInOption> = options.iter().filter(|o| !o.device).collect();
    if !cli.is_empty() {
        out.push(header("vendor CLIs \u{00b7} sign out at the terminal"));
    }
    for o in cli {
        out.push(MenuItem {
            label: o.name.clone(),
            desc: match &o.logout {
                Some(cmd) => format!("signed in through its CLI \u{00b7} run `{cmd}`"),
                None => "signed in through its CLI \u{00b7} it owns the sign-out".into(),
            },
            fill: o.name.clone(),
            submit: true,
            dim: true,
            ..Default::default()
        });
    }
    out
}

/// Open the picker over the composer with the broker's `options`.
pub(crate) fn open(palette: &mut Option<PaletteState>, options: &[SignInOption]) {
    let items = items_out(options);
    *palette = Some(PaletteState {
        kind: Kind::SignOut,
        sel: crate::suggest::first_selectable(&items),
        items,
        entries: Vec::new(),
        touched: false,
    });
}

impl crate::chat::ChatPane {
    /// The broker answered a bare `/logout` with its rows: open the picker,
    /// and count that as the reply. A bare construct goes out like any line
    /// and latches `awaiting`; the old answer was a `Message`, which clears
    /// it, but the picker event is the WHOLE answer now — no message
    /// follows. Left latched, the pane stayed busy, so the pick itself was
    /// queued behind a reply that was never coming: the popup opened, and
    /// then nothing happened.
    pub(crate) fn open_sign_out_picker(&mut self, options: &[SignInOption]) {
        self.awaiting = false;
        open(&mut self.palette, options);
    }
}

#[cfg(test)]
#[path = "signoutpick_tests.rs"]
mod tests;
