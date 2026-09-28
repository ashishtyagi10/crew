//! The router's cost lands on the arm's turn total, once, and nowhere else.
use super::folded;
use crate::protocol::PluginEvent;
use crew_hive::Spent;

const ROUTER: Spent = Spent {
    input: 100,
    output: 10,
    micros_usd: 1000,
};

fn stat(agent: &str, tok_in: u64, tok_out: u64, cost: u64) -> PluginEvent {
    PluginEvent::Stats {
        exchanges: 1,
        tokens: tok_in + tok_out,
        agent: agent.into(),
        ms: 0,
        ctx: 0,
        tok_in,
        tok_out,
        cost_microusd: cost,
        tools: None,
    }
}

/// A stat as `(agent, exchanges, tokens, tok_in, tok_out, cost)`.
type Row = (String, u32, u64, u64, u64, u64);

fn row(ev: &PluginEvent) -> Row {
    match ev {
        PluginEvent::Stats {
            agent,
            exchanges,
            tokens,
            tok_in,
            tok_out,
            cost_microusd,
            ..
        } => (
            agent.clone(),
            *exchanges,
            *tokens,
            *tok_in,
            *tok_out,
            *cost_microusd,
        ),
        other => panic!("not a stat: {other:?}"),
    }
}

/// Every event the arm said, after the fold, as rows.
fn through(spent: Spent, said: Vec<PluginEvent>) -> Vec<Row> {
    let mut out = Vec::new();
    let mut keep = |ev: PluginEvent| -> anyhow::Result<()> {
        out.push(row(&ev));
        Ok(())
    };
    folded(spent, &mut keep, |emit| {
        for ev in said {
            emit(ev)?;
        }
        Ok(())
    })
    .unwrap();
    out
}

fn r(agent: &str, exchanges: u32, tok_in: u64, tok_out: u64, cost: u64) -> Row {
    (
        agent.into(),
        exchanges,
        tok_in + tok_out,
        tok_in,
        tok_out,
        cost,
    )
}

#[test]
fn the_first_turn_total_carries_the_router_and_a_reply_stat_does_not() {
    let out = through(
        ROUTER,
        vec![stat("coder", 5, 6, 7), stat("", 5, 6, 7), stat("", 1, 1, 1)],
    );
    assert_eq!(
        out,
        vec![
            r("coder", 1, 5, 6, 7),
            r("", 1, 105, 16, 1007),
            r("", 1, 1, 1, 1),
        ],
        "the agent's own stat untouched, the first total charged, a later one not again"
    );
}

#[test]
fn an_arm_that_says_no_total_gets_one_of_the_router_alone() {
    let out = through(ROUTER, vec![stat("coder", 5, 6, 7)]);
    assert_eq!(
        out,
        vec![r("coder", 1, 5, 6, 7), r("", 0, 100, 10, 1000)],
        "no agent was dialled for the router's own total"
    );
}

#[test]
fn nothing_spent_passes_the_arm_through_untouched() {
    let out = through(Spent::default(), vec![stat("", 5, 6, 7)]);
    assert_eq!(out, vec![r("", 1, 5, 6, 7)]);
    assert!(through(Spent::default(), Vec::new()).is_empty());
}

#[test]
fn a_failed_arm_keeps_its_error_and_the_router_is_still_said() {
    let mut out = Vec::new();
    let mut keep = |ev: PluginEvent| -> anyhow::Result<()> {
        out.push(row(&ev));
        Ok(())
    };
    let r = folded(ROUTER, &mut keep, |_| Err(anyhow::anyhow!("the arm broke")));
    assert_eq!(r.unwrap_err().to_string(), "the arm broke");
    assert_eq!(out, vec![self::r("", 0, 100, 10, 1000)]);
}
