use super::*;

#[test]
fn a_solo_run_is_the_old_cap_exactly() {
    let b = ToolBudget::solo();
    assert_eq!(b.total(), MAX_TOOL_ROUNDS);
    for used in 0..MAX_TOOL_ROUNDS {
        assert_eq!(
            b.take(used),
            Some(MAX_TOOL_ROUNDS - used - 1),
            "round {used}"
        );
    }
    assert_eq!(b.take(MAX_TOOL_ROUNDS), None, "the fifth is refused");
    assert_eq!(b.left(), 0);
}

#[test]
fn a_run_pools_its_tasks_rounds_and_one_agent_may_take_twice_its_share() {
    let b = ToolBudget::for_run(3);
    assert_eq!(b.total(), 12);
    let mut used = 0;
    while b.take(used).is_some() {
        used += 1;
    }
    assert_eq!(used, 2 * MAX_TOOL_ROUNDS, "the ceiling, not the pool");
    assert_eq!(b.left(), 4, "and the rest is still there for another task");
    let other = b.clone();
    assert_eq!(
        other.take(0),
        Some(3),
        "a second agent draws from the same pool"
    );
    assert_eq!(b.left(), 3, "the clone IS the pool");
}

#[test]
fn what_is_promised_is_the_smaller_of_the_pool_and_the_ceiling() {
    let b = ToolBudget::for_run(2);
    // Pool 8, ceiling 8: the first take promises 7 — the pool.
    assert_eq!(b.take(0), Some(7));
    // Drain six more elsewhere; a fresh agent's promise is now the pool's 1.
    for _ in 0..6 {
        b.take(0).unwrap();
    }
    assert_eq!(b.left(), 1);
    assert_eq!(b.take(0), Some(0), "this one, and no more");
    assert_eq!(b.take(0), None);
    assert_eq!(b.left(), 0, "a refusal takes nothing");
}
