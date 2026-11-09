use crate::model::{GridSize, TermCore};

fn text(core: &TermCore) -> String {
    let mut cs = core.cells(true);
    cs.sort_by_key(|c| (c.row, c.col));
    cs.iter().map(|c| c.c).collect()
}

fn alt(core: &TermCore) -> bool {
    core.input_modes().alt_screen
}

/// A program on the alternate screen dies (no `1049 l`) and the shell's
/// next prompt arrives in the same chunk: the pane is on the primary screen
/// again, the prompt is drawn THERE, and the dead frame is gone.
#[test]
fn a_prompt_on_a_dead_programs_screen_lands_on_the_primary_one() {
    let mut core = TermCore::new(GridSize { cols: 30, rows: 4 });
    core.feed(b"$ vim\r\n\x1b[?1049h\x1b[HVIMFRAME");
    assert!(alt(&core));
    core.feed(b"killed\r\n\x1b]133;A\x07back$ ");
    assert!(!alt(&core), "the prompt left the alternate screen");
    let t = text(&core);
    assert!(
        t.contains("back$"),
        "the prompt is drawn on the primary: {t}"
    );
    assert!(t.contains("$vim"), "the primary's history is back: {t}");
    assert!(!t.contains("VIMFRAME"), "{t}");
}

/// The mark split across reads is still the cut.
#[test]
fn a_prompt_mark_split_across_reads_still_leaves() {
    let mut core = TermCore::new(GridSize { cols: 30, rows: 4 });
    core.feed(b"\x1b[?1049hFRAME\x1b]13");
    core.feed(b"3;A\x1b\\back$ ");
    assert!(!alt(&core));
    assert!(text(&core).contains("back$"));
}

/// A program that lives keeps its screen: other marks (`C`, `D`), or a
/// prompt drawn on the primary screen, change nothing.
#[test]
fn only_a_prompt_mark_leaves_and_only_the_alternate_screen() {
    let mut core = TermCore::new(GridSize { cols: 30, rows: 4 });
    core.feed(b"\x1b[?1049h\x1b]133;C\x07\x1b]133;D;0\x07FRAME");
    assert!(alt(&core), "C and D are not a prompt");
    let mut primary = TermCore::new(GridSize { cols: 30, rows: 4 });
    primary.feed(b"one\r\n\x1b]133;A\x07two$ ");
    assert!(!alt(&primary));
    assert!(text(&primary).contains("one"), "nothing restored or wiped");
}
