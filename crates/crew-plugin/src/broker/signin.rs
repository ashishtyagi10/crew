//! The in-pane device sign-in behind `/model <n>` on a signed-out
//! device-flow provider (condition 7 meeting condition 4): the code card
//! streams as a normal chat message, the poll runs on the task's worker
//! thread with sliced sleeps so `/stop` lands within a beat, and success
//! pins the provider. Split from `modelcmd.rs` for the line cap.
use crate::PluginEvent;

use super::auth::device::{self, Outcome, SignInCard};
use super::relay::msg;
use super::session::Session;

/// Run a device-flow sign-in IN THE PANE: the code card streams as a normal
/// chat message, the poll runs right here on the task's worker thread (the
/// stdio loop keeps draining; `/stop` cancels between polls), and success
/// pins the provider so the choice survives restarts. Condition 7's
/// affordance meeting condition 4's engine.
pub(crate) fn signin_cmd(
    session: &Session,
    name: &str,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let Some(e) = device::endpoints_for(name) else {
        return emit(msg(
            "agent smith",
            format!("{name} declares no sign-in flow"),
        ));
    };
    let outcome = device::run_flow(
        name,
        &e,
        &mut |card| {
            let _ = emit(msg("agent smith", card_text(name, &card)));
        },
        &mut |d| sliced_sleep(d, session),
        &mut |p, t| {
            let _ = super::auth::tokens::store(p, t);
        },
        &|| session.cancelled(),
    );
    let note = match outcome {
        Outcome::SignedIn => {
            let _ = crate::credentials::save_pin(name);
            format!(
                "\u{2713} signed in \u{2014} {name} now serves smith work \
                 (persists across restarts)"
            )
        }
        Outcome::Expired => {
            "the sign-in code expired \u{2014} pick it in /model to try again".into()
        }
        Outcome::Denied => "sign-in denied at the provider \u{2014} nothing stored".into(),
        Outcome::TimedOut => "sign-in timed out \u{2014} pick it in /model to try again".into(),
        Outcome::Stopped => "sign-in stopped".into(),
        Outcome::Failed(e) => format!("sign-in failed: {e}"),
    };
    emit(PluginEvent::Roster {
        agents: session.registry().infos(),
    })?;
    emit(msg("agent smith", note))
}

/// The code card: the two things the human needs (code, URL), framed as a
/// plain text block — in-pane, no overlays, ASCII-narrow like the nameplate.
fn card_text(name: &str, c: &SignInCard) -> String {
    let url = c
        .verification_uri_complete
        .as_deref()
        .unwrap_or(&c.verification_uri);
    let bar = "\u{2500}".repeat(31);
    format!(
        "\u{250c}{bar}\u{2510}\n\
         \u{2502} sign in to {name}\n\
         \u{2502} code:  {}\n\
         \u{2502} visit: {}\n\
         \u{2514}{bar}\u{2518}\n\
         waiting for approval\u{2026} (/stop cancels)",
        c.user_code, url
    )
}

/// Sleep `d` in 100ms slices so a `/stop` lands within a beat rather than
/// after a full poll interval.
fn sliced_sleep(d: std::time::Duration, session: &Session) {
    let end = std::time::Instant::now() + d;
    loop {
        let now = std::time::Instant::now();
        if now >= end || session.cancelled() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(100).min(end - now));
    }
}
