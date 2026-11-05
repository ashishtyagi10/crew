//! Which sequences count as an erase or a screen switch, and where they end.
use super::*;

fn hits(chunks: &[&[u8]]) -> Vec<Hit> {
    let mut s = WipeScan::default();
    chunks.iter().flat_map(|c| s.feed(c)).map(|h| h.1).collect()
}

#[test]
fn clear_is_an_erase_of_the_screen_then_of_the_scrollback() {
    assert_eq!(
        hits(&[b"\x1b[H\x1b[2J\x1b[3J"]),
        [Hit::EraseScreen, Hit::EraseSaved]
    );
    // The index is the final byte, so the parser can stop just short of it.
    assert_eq!(
        WipeScan::default().feed(b"ab\x1b[2J"),
        [(5, Hit::EraseScreen)]
    );
}

#[test]
fn the_alternate_screen_in_all_its_spellings() {
    assert_eq!(hits(&[b"\x1b[?1049h"]), [Hit::AltScreen(true)]);
    assert_eq!(hits(&[b"\x1b[?1047l"]), [Hit::AltScreen(false)]);
    assert_eq!(hits(&[b"\x1b[?1006;47h"]), [Hit::AltScreen(true)]);
    assert_eq!(hits(&[b"\x1bc"]), [Hit::Reset]);
}

#[test]
fn a_sequence_split_across_reads_still_counts() {
    assert_eq!(hits(&[b"\x1b", b"[", b"3", b"J"]), [Hit::EraseSaved]);
    assert_eq!(hits(&[b"\x1b[?10", b"49l"]), [Hit::AltScreen(false)]);
}

#[test]
fn near_misses_drop_nothing() {
    let quiet: &[&[u8]] = &[
        b"\x1b[J",      // erase below the cursor
        b"\x1b[1J",     // erase above it
        b"\x1b[?2J",    // selective erase
        b"\x1b[2 J",    // an intermediate makes it something else
        b"\x1b[?1006h", // a mouse mode
        b"\x1b[1049h",  // not private: an ANSI mode, not the screen
        b"2J [3J ?1049h",
    ];
    for q in quiet {
        assert_eq!(hits(&[q]), [], "{:?}", String::from_utf8_lossy(q));
    }
}
