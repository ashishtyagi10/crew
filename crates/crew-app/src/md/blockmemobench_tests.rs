//! The heavy proofs, for a release build: `cargo test --release -p crew-app --bin crew
//! blockmemo -- --ignored --nocapture`.
use super::*;
use std::time::Instant;

/// Every `.md` in the repository, at three widths: the block render equals the whole one.
#[test]
#[ignore = "release sweep of every .md in the repo"]
fn every_repo_md_renders_alike() {
    let _f = crate::glyphs::force(false);
    let (mut block, mut whole) = (0, 0);
    for p in repo_md() {
        let text = std::fs::read_to_string(&p).unwrap_or_default();
        match memo::walk(&text).tops {
            Some(_) => block += 1,
            None => whole += 1,
        }
        for w in WIDTHS {
            assert_eq!(render(&text, w), full(&text, w), "{} at {w}", p.display());
        }
    }
    println!("{block} files by block, {whole} rendered whole");
    assert!(block > whole);
}

/// Hundreds of random edits to the manual, checked after every one.
#[test]
#[ignore = "release: 300 random edits to the 327 KB manual"]
fn random_edits_to_the_manual_render_alike() {
    let _f = crate::glyphs::force(false);
    let mut text = manual();
    let mut rng = Lcg(0xc0ffee);
    for i in 0..300 {
        rng.edit(&mut text);
        let w = WIDTHS[i % WIDTHS.len()];
        assert_eq!(render(&text, w), full(&text, w), "edit {i}, width {w}");
    }
}

/// One keystroke in the middle of the manual: the whole render it cost before, against the
/// block render now. Both pay the walk the caret's block lookup needs.
#[test]
#[ignore = "release timing"]
fn time_an_insert_in_the_manual() {
    let _f = crate::glyphs::force(false);
    let mut text = manual();
    let mid = text.len() / 2;
    let mid = (mid..text.len())
        .find(|&i| text.is_char_boundary(i))
        .unwrap_or(mid);
    render(&text, 100);
    let n = 40u32;
    let (mut whole, mut block, mut walk) = (0f64, 0f64, 0f64);
    for _ in 0..n {
        text.insert(mid, 'x');
        let t = Instant::now();
        let w = memo::walk(&text);
        walk += t.elapsed().as_secs_f64();
        assert!(w.tops.is_some());
        let t = Instant::now();
        let a = render(&text, 100);
        block += t.elapsed().as_secs_f64();
        let t = Instant::now();
        let b = full(&text, 100);
        whole += t.elapsed().as_secs_f64();
        assert_eq!(a.len(), b.len());
    }
    let ms = |s: f64| s * 1000.0 / f64::from(n);
    println!(
        "per insert: walk {:.2} ms; whole render {:.2} ms -> block render {:.2} ms",
        ms(walk),
        ms(whole),
        ms(block)
    );
}

/// Many seeds, each from the mixed document afresh: the shapes one seed never reaches.
#[test]
#[ignore = "release: 500 seeds x 120 edits"]
fn many_seeds_render_alike() {
    let _f = crate::glyphs::force(false);
    for seed in 0..500u64 {
        let mut text = MIXED.to_string();
        let mut rng = Lcg(seed);
        for i in 0..120 {
            rng.edit(&mut text);
            let w = WIDTHS[(i + seed as usize) % WIDTHS.len()];
            assert_eq!(
                render(&text, w),
                full(&text, w),
                "seed {seed} edit {i}: {text:?}"
            );
        }
    }
}
