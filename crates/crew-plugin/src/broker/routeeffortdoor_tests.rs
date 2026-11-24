//! The router's verdict, entered where production enters it: `routed()`.
use std::cell::Cell;

use super::*;
use crate::broker::intent::decision::Decision;

/// The production door: a message the router calls a plan runs its arm at
/// `high`, and the message after it, routed a reply, at nothing. (A plan,
/// not a review: `shapeguard` lets a review stand only on a message that
/// asks for one, and such a message is the review gate's before the router.)
#[test]
fn a_message_routed_as_a_plan_runs_its_arm_at_high() {
    let _env = crate::broker::testenv::mock("ok\n@done");
    let mut session = crate::broker::session::Session::new();
    let at = |said: &'static str, session: &mut crate::broker::session::Session| {
        let call = move |_: &str| Ok(said.to_string());
        let counted = crate::broker::intent::uncounted(&call);
        let seen = Cell::new(None);
        let mut arm =
            |_: &Decision,
             _: &mut crate::broker::session::Session,
             _: &mut dyn FnMut(crate::PluginEvent) -> anyhow::Result<()>| {
                seen.set(Some(ROUTE.with(Cell::get)));
                Ok(())
            };
        crate::broker::intent::routed(
            "tidy the parser module",
            Some(&counted),
            session,
            &mut arm,
            &mut |_| Ok(()),
        )
        .unwrap();
        seen.get().expect("the arm ran")
    };
    assert_eq!(at("SHAPE: plan\nWHY: w", &mut session), Some(Effort::High));
    assert_eq!(ROUTE.with(Cell::get), None, "gone once the message ends");
    assert_eq!(at("SHAPE: reply\nWHY: w", &mut session), None);
}
