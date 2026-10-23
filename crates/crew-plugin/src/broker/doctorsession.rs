//! `/doctor`'s session line, the one `/status` used to carry: turns, tokens
//! and the relay budget. Moved out of `doctor` (over the line cap) when the
//! hooks line came in.

/// `4 turns · ~950 tok (~237/turn) · unlimited budget`. `~n/turn` only once
/// there is a turn to average over: a `0/turn` reading would be meaningless,
/// and turns == 0 would divide by zero.
pub(super) fn detail(turns: u64, tokens: u64, budget: usize) -> String {
    let per_turn = match tokens.checked_div(turns) {
        Some(avg) => format!(" (~{avg}/turn)"),
        None => String::new(),
    };
    let budget = match budget {
        0 => "unlimited budget".to_string(),
        b => format!("~{b} tok budget"),
    };
    let s = if turns == 1 { "" } else { "s" };
    format!("{turns} turn{s} · ~{tokens} tok{per_turn} · {budget}")
}
