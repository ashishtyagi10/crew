//! The SERVING card takes the rows it draws, and no more.
use super::*;
use crate::navlayout::{layout_with, Tail};
use crate::usageledger::{WindowStat, Windows};

fn metered() -> Serving {
    Serving {
        provider: Some("claude-code".into()),
        model: Some("claude-sonnet-5".into()),
        windows: Windows {
            five_h: Some(WindowStat {
                left_ms: 60_000,
                spent: 1,
                budget: 4,
            }),
            seven_d: None,
        },
    }
}

/// Every drawn row sits inside the block, with its last row left as the gap.
fn fits(s: &Serving) {
    let (cells, _) = serving_cells(s, 40);
    let last = cells.iter().map(|c| c.row).max().unwrap();
    assert_eq!(last + 2, block(s), "rows drawn vs rows taken");
}

#[test]
fn an_unmetered_card_gives_back_the_meter_rows() {
    assert_eq!(block(&Serving::default()), SERVING_BLOCK - 2);
    fits(&Serving::default());
    assert_eq!(block(&metered()), SERVING_BLOCK);
    fits(&metered());
}

/// With no provider signed in, WAITING ON YOU follows `no provider — /model`
/// after one row of air — not after the two empty rows the meters had.
#[test]
fn waiting_follows_an_unmetered_card_without_a_hole() {
    let tail = |s: &Serving| Tail::Glance {
        waiting: 1,
        weather: 0,
        serving: block(s),
    };
    let bare = layout_with(60, true, tail(&Serving::default()), 2);
    let full = layout_with(60, true, tail(&metered()), 2);
    assert_eq!(bare.waiting_top, bare.serving_top + 3);
    assert_eq!(full.waiting_top, full.serving_top + SERVING_BLOCK);
}
