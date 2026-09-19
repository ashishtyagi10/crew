//! The LOG's clock column: how wide it is, how a stamp is found in a
//! buffered line, and the one question the section asks before drawing any of
//! them — whether a nav this narrow can afford it at all.
//!
//! Split from [`super`] (child module) when that question arrived, along the
//! line between what the section DRAWS and what it knows about the six
//! columns down its left.

/// Column the entry text starts on, under the `LOG` rule's own indent.
pub(super) const TEXT_COL: u16 = 2;
/// Columns a message needs before the clock beside it is worth its own six.
///
/// The stamp is fixed furniture — six columns on every row — and on the
/// collapsed rail it was taking a third of the section: every line read
/// `crew v0…`, `restored…`, `shell pr…`, which is a column of timestamps
/// beside a column of nothing. The rail's own rule (see `navtext`) is that
/// prose ellipsizes and a row of VALUES drops whole values, and at this width
/// the clock is the value that goes: the log is a live tail read from the
/// bottom, and the app bar's clock is two cards up.
///
/// Fourteen is where a message stops being a phrase: the cut falls at 23
/// columns, so a docked nav (27 by default) keeps its clock and the collapsed
/// rail gives the words the width.
const MSG_MIN: usize = 14;

/// Whether a LOG `cols` wide can carry its stamps and still say anything.
pub(super) fn stamps_fit(cols: u16) -> bool {
    let room = cols.saturating_sub(TEXT_COL + 1) as usize;
    room.saturating_sub(STAMP_W) >= MSG_MIN
}

/// Columns a stamp and its trailing space take.
const STAMP_W: usize = 6;
/// What a repeated stamp leaves behind: the same six columns, empty, so the
/// messages beside it stay in one column.
pub(super) const BLANK_STAMP: &str = "      ";

/// Split a buffered entry into its `HH:MM ` stamp and the message. The stamp
/// is prepended when the line is buffered, so it is a prefix of the text
/// rather than a field — recognised by shape, and absent (`""`) on any line
/// that does not carry one.
pub(super) fn split_stamp(s: &str) -> (&str, &str) {
    let b = s.as_bytes();
    let stamped = b.len() > 6
        && b[0].is_ascii_digit()
        && b[1].is_ascii_digit()
        && b[2] == b':'
        && b[3].is_ascii_digit()
        && b[4].is_ascii_digit()
        && b[5] == b' ';
    if stamped {
        s.split_at(6)
    } else {
        ("", s)
    }
}

#[cfg(test)]
#[path = "navlogstamp_tests.rs"]
mod tests;
