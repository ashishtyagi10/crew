//! The judges' one-shot: the standard model, held at the task's spend cap.
//!
//! WHY apart from `classify`: it is the one `live_bounded` caller that spends
//! the task's money, and it grew that file past its size.
use crew_hive::Spent;

use super::classify::{live_bounded, ONESHOT_TIMEOUT};

/// A judge's call: the standard model, held at the task's spend cap. Cheap
/// routing calls are not (they must answer at the cap, to say why the task
/// stopped); a verdict is a full-price call like the work it grades. Call it
/// on the task's thread, where the meter is.
pub(crate) fn live_judge_call(
    max_tokens: u32,
) -> Option<impl Fn(&str) -> Result<(String, Spent), String>> {
    live_bounded(
        max_tokens,
        crew_hive::ModelTier::Standard,
        ONESHOT_TIMEOUT,
        true,
    )
}
