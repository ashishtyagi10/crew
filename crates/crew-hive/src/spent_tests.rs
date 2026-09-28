use super::Spent;
use crate::graph::ModelTier;
use crate::provider::Completion;

fn reply(input: u32, output: u32, cost: u64) -> Completion {
    Completion {
        input_tokens: input,
        output_tokens: output,
        cost_microusd: cost,
        ..Default::default()
    }
}

#[test]
fn a_reported_cost_is_billed_as_given() {
    let s = Spent::billed("claude-haiku-4-5", ModelTier::Cheap, &reply(100, 10, 7));
    assert_eq!(
        s,
        Spent {
            input: 100,
            output: 10,
            micros_usd: 7
        }
    );
}

#[test]
fn an_unreported_cost_is_priced_the_way_a_workers_round_is() {
    let c = reply(100, 10, 0);
    let s = Spent::billed("mock", ModelTier::Cheap, &c);
    assert_eq!(
        s.micros_usd,
        crate::apiagent::billed("mock", ModelTier::Cheap, &c)
    );
    assert_eq!(s.micros_usd, 150, "the tier's own model, not zero");
}

#[test]
fn spends_add_field_by_field() {
    let mut s = Spent {
        input: 1,
        output: 2,
        micros_usd: 3,
    };
    s += Spent {
        input: 10,
        output: 20,
        micros_usd: 30,
    };
    assert_eq!((s.input, s.output, s.micros_usd), (11, 22, 33));
    assert_eq!(s.tokens(), 33);
    assert!(!s.is_zero());
    assert!(Spent::default().is_zero());
}
