//! The `/login` and `/logout` pickers: the broker's sign-in rows as a popup.
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

/// Which way the picker goes: `/login` offers sign-ins, `/logout` the
/// stored ones to remove. One palette kind carries both — the rows differ,
/// the keys and the shape do not.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Auth {
    In,
    Out,
}

impl Auth {
    /// The construct a pick submits, without the slash.
    pub(crate) fn construct(self) -> &'static str {
        match self {
            Auth::In => "login",
            Auth::Out => "logout",
        }
    }

    /// The popup card's legend.
    pub(crate) fn legend(self) -> &'static str {
        match self {
            Auth::In => "sign in",
            Auth::Out => "sign out",
        }
    }
}

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
pub(crate) fn open(palette: &mut Option<PaletteState>, auth: Auth, options: &[SignInOption]) {
    let items = match auth {
        Auth::In => {
            let keyed = crate::shellprobe::keys_now().contains(crate::oauth::OPENROUTER_KEY_VAR);
            items(options, keyed)
        }
        Auth::Out => items_out(options),
    };
    *palette = Some(PaletteState {
        kind: Kind::Auth(auth),
        sel: crate::suggest::first_selectable(&items),
        items,
        entries: Vec::new(),
        touched: false,
    });
}

#[cfg(test)]
#[path = "loginpick_tests.rs"]
mod tests;
