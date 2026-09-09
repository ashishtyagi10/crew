//! The `/login` picker: the broker's sign-in options as popup rows.
//!
//! `/login` used to answer with a numbered table and wait for a name or a
//! number typed back — a form to fill in for a choice of three. The broker
//! now sends the rows (`PluginEvent::SignIn`) and this turns them into the
//! composer popup every other pick in crew uses: arrows, Enter, Esc. A pick
//! submits `/login <name>`, and the broker runs the flow it already had.
//!
//! One row is the app's own: OpenRouter signs in through the browser
//! (`crate::oauth`), a flow the broker does not know. It rides here as the
//! same `needs` row the model picker uses, so picking it opens the browser
//! and the paste prompt beneath it.
use crew_plugin::SignInOption;

use crate::chatpalette::{Kind, PaletteState};
use crate::suggest::MenuItem;

fn header(label: &str) -> MenuItem {
    MenuItem {
        label: label.to_string(),
        header: true,
        ..Default::default()
    }
}

/// What a row says after its name.
fn state(o: &SignInOption) -> String {
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

/// The rows: the sign-ins a pick can run (the broker's device flows and
/// the browser flow), then the vendor CLIs that own theirs, dimmed with the
/// command to run. `openrouter_keyed` is whether that key is already held.
pub(crate) fn items(options: &[SignInOption], openrouter_keyed: bool) -> Vec<MenuItem> {
    let mut out = vec![header("OAuth \u{00b7} runs right here")];
    for o in options.iter().filter(|o| o.device) {
        out.push(MenuItem {
            label: o.name.clone(),
            desc: state(o),
            fill: o.name.clone(),
            submit: true,
            ..Default::default()
        });
    }
    out.push(MenuItem {
        label: "openrouter".into(),
        desc: if openrouter_keyed {
            "\u{2713} key present \u{00b7} pick to sign in again in the browser".into()
        } else {
            "sign in in the browser".into()
        },
        fill: "openrouter".into(),
        needs: Some(crate::oauth::OPENROUTER_KEY_VAR.to_string()),
        ..Default::default()
    });
    let cli: Vec<&SignInOption> = options.iter().filter(|o| !o.device).collect();
    if !cli.is_empty() {
        out.push(header("vendor CLIs \u{00b7} sign in at the terminal"));
    }
    for o in cli {
        out.push(MenuItem {
            label: o.name.clone(),
            desc: state(o),
            fill: o.name.clone(),
            submit: true,
            dim: !o.signed_in,
            ..Default::default()
        });
    }
    out
}

/// Open the picker over the composer with the broker's `options`.
pub(crate) fn open(palette: &mut Option<PaletteState>, options: &[SignInOption]) {
    let keyed = crate::shellprobe::keys_now().contains(crate::oauth::OPENROUTER_KEY_VAR);
    let items = items(options, keyed);
    *palette = Some(PaletteState {
        kind: Kind::Login,
        sel: crate::suggest::first_selectable(&items),
        items,
        entries: Vec::new(),
        touched: false,
    });
}

#[cfg(test)]
#[path = "loginpick_tests.rs"]
mod tests;
