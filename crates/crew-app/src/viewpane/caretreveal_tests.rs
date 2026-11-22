//! The block the caret is in, drawn as its markdown — as the pane holds it.
use crate::viewpane::caret::Step;
use crate::viewpane::editpane_tests::doc;

fn with(text: &str) -> crate::viewpane::ViewPane {
    let mut p = doc();
    if let crate::viewpane::LoadState::Ready { loaded, .. } = &mut p.state {
        loaded.text = text.into();
    }
    p
}

fn drawn(p: &crate::viewpane::ViewPane, cols: u16) -> Vec<String> {
    p.lines_for(cols)
        .lines
        .iter()
        .map(|l| l.iter().map(|c| c.c).collect::<String>().trim().to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

#[test]
fn the_caret_s_block_shows_its_markers_and_the_others_do_not() {
    let mut p = with("# Title\n\nSome **bold** words.\n");
    assert_eq!(
        drawn(&p, 40),
        ["Title", "Some bold words."],
        "a reader sees no markers"
    );
    p.start_editing(40);
    assert_eq!(drawn(&p, 40), ["# Title", "Some bold words."]);
    p.move_caret(Step::Down, 40, 20);
    assert_eq!(drawn(&p, 40), ["Title", "Some **bold** words."]);
    // Down keeps the column it aimed for: under the `T` of `# Title` is the `m` of `Some`.
    assert_eq!(
        p.caret_at,
        Some(11),
        "found again by its byte in the new layout"
    );
}

/// THE thing a hidden marker could never do: take a heading back to prose.
#[test]
fn a_heading_s_hashes_can_be_deleted() {
    let mut p = with("# Title\n\nbody\n");
    p.start_editing(40);
    p.move_caret(Step::Home, 40, 20);
    assert_eq!(p.caret_at, Some(0), "Home is the `#` now that it is drawn");
    p.delete(40, 20);
    p.delete(40, 20);
    assert_eq!(p.source_str(), Some("Title\n\nbody\n"));
    assert_eq!(drawn(&p, 40), ["Title", "body"]);
}

#[test]
fn a_link_s_url_is_typed_into_in_place() {
    let mut p = with("see [docs](https://a.invalid) now\n");
    p.start_editing(60);
    let url_end = "see [docs](https://a.invalid".len() as u32;
    while p.caret_at < Some(url_end) {
        p.move_caret(Step::Right, 60, 20);
    }
    p.insert("/x", 60, 20);
    assert_eq!(
        p.source_str(),
        Some("see [docs](https://a.invalid/x) now\n")
    );
}

/// A fence used to have no bytes in its render at all, so Down stepped straight over it.
#[test]
fn the_caret_steps_into_a_fence_and_enter_there_is_one_newline() {
    let mut p = with("intro\n\n```\n    let x = 1;\n```\n\nafter\n");
    p.start_editing(40);
    p.move_caret(Step::Down, 40, 20);
    let lines = drawn(&p, 40);
    assert!(
        lines.iter().any(|l| l == "```"),
        "the fence is drawn as written: {lines:?}"
    );
    p.move_caret(Step::End, 40, 20);
    p.newline(40, 20);
    p.insert("y", 40, 20);
    assert_eq!(
        p.source_str(),
        Some("intro\n\n```\n    let x = 1;\n    y\n```\n\nafter\n")
    );
}

/// The line being edited stays where the eye is when the block above it stops being revealed.
#[test]
fn leaving_a_revealed_block_keeps_the_caret_s_row_on_screen() {
    // Two rows rendered, six revealed: every `w` is four bytes of markers on screen.
    let long = "**w** ".repeat(30);
    // Enough above it that the view is scrolled: with the top of the document on screen there
    // is nothing to scroll and every row below a collapsing block has to move up.
    let above = "para\n\n".repeat(20);
    let mut p = with(&format!("{above}{long}\n\nnext\n"));
    p.start_editing(30);
    p.move_caret(Step::Bottom, 30, 20);
    p.move_caret(Step::Up, 30, 20);
    // Where `next` is on screen with the long paragraph above it revealed…
    let row = p
        .lines_for(30)
        .lines
        .iter()
        .position(|l| l.iter().map(|c| c.c).collect::<String>().contains("next"));
    let on_screen = row.unwrap() - p.scroll;
    assert!(p.scroll > 0);
    p.move_caret(Step::Down, 30, 20);
    // …is where the caret is once it has moved there and the paragraph has rendered again.
    assert_eq!(p.caret.unwrap().row - p.scroll, on_screen);
}

/// Shot-caught: the fence lines were marker ink, which the code field reads as a quote's bar
/// and starts itself after — so it padded every code line eight columns in, mid-word.
#[test]
fn a_revealed_fence_s_code_is_drawn_whole() {
    let mut p = with("```rust\nfn main() {\n    println!(\"hi\");\n}\n```\n");
    p.start_editing(40);
    assert_eq!(
        drawn(&p, 40),
        ["```rust", "fn main() {", "println!(\"hi\");", "}", "```"]
    );
}

/// A fence inside a list item is drawn AFTER the whole list, out of byte order. Revealing the
/// item must not take the rows between with it, and the caret must still find its byte.
#[test]
fn a_fence_hoisted_out_of_a_list_keeps_the_items_around_it() {
    let src = "1. one\n   ```\n   code\n   ```\n2. two\n";
    let mut p = with(src);
    p.start_editing(40);
    let lines = drawn(&p, 40);
    assert!(lines.iter().any(|l| l.contains("two")), "{lines:?}");
    // Into the second item, by byte, and type there twice: the caret follows its byte.
    p.caret_at = Some(src.find("two").unwrap() as u32);
    p.relayout_caret(40, 20);
    p.insert("X", 40, 20);
    p.insert("Y", 40, 20);
    assert_eq!(
        p.source_str(),
        Some("1. one\n   ```\n   code\n   ```\n2. XYtwo\n")
    );
}

/// What a caret crossing blocks costs on the frame thread: each crossing re-lays the whole
/// document. Printed, not asserted — a debug build on a shared machine is no benchmark.
#[test]
#[ignore = "timing; run by hand"]
fn time_downs_through_the_manual() {
    let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/CREW.md"));
    let mut p = with(&text.unwrap());
    p.start_editing(100);
    let t = std::time::Instant::now();
    for _ in 0..200 {
        p.move_caret(Step::Down, 100, 40);
    }
    eprintln!(
        "200 downs: {:?} ({:?} each)",
        t.elapsed(),
        t.elapsed() / 200
    );
}
