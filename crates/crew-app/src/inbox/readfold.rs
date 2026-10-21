//! A long thread read as Gmail reads it (the reader loop, round 2): in a
//! thread of three mails or more, every mail but the newest folds to one
//! line, who wrote it, how it starts and when, so the conversation reads at
//! a glance and the newest is where the eye lands. A click on a folded mail
//! opens it; `x` opens them all, or folds them again.
use super::reader::{row, Key};
use super::readmail::Lay;
use super::wire::sender_name;
use super::{InboxPane, Reading};
use crate::chatbody::{plain, CardLine};
use crate::chatwidth::{clip_w, str_w};

/// A thread folds when it has this many mails.
const FOLDS_AT: usize = 3;
/// The widest a folded mail's name runs.
const NAME: usize = 20;

impl Reading {
    /// Whether mail `i` of `n` is folded to its line.
    pub(crate) fn folded(&self, i: usize, n: usize) -> bool {
        n >= FOLDS_AT && i + 1 < n && !self.all && !self.open.contains(&i)
    }

    /// `x`: every mail open, or the older ones folded again. The line to say.
    pub(crate) fn fold_all(&mut self) -> &'static str {
        let n = self.mails.as_ref().map_or(0, Vec::len);
        if n < FOLDS_AT {
            return "every mail in this thread is open";
        }
        (self.all, self.folds) = (!self.all, self.folds + 1);
        self.open.clear();
        match self.all {
            true => "every mail open \u{b7} x folds the older ones",
            false => "",
        }
    }
}

/// The column a folded mail's words start after its name: the widest name
/// folded in `mails`, so every line's words start together.
pub(crate) fn names_w(mails: &[super::api::Mail]) -> usize {
    let w = |m: &super::api::Mail| str_w(&clip_w(&sender_name(&m.from), NAME));
    mails.iter().map(w).max().unwrap_or(0)
}

/// How a mail starts, in words: its markdown's marks, a table's rules and
/// bars, and a picture's mark gone.
fn said(md: &str) -> String {
    let words = md.lines().map(|l| {
        let l = l
            .trim()
            .trim_start_matches(['#', '>', '-', '*', ' ', '\u{25a3}']);
        match l.chars().all(|c| "|-: ".contains(c)) {
            true => String::new(),
            false => super::html::plain_of(l).replace('|', " "),
        }
    });
    let all: Vec<String> = words.collect();
    all.join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Mail `i`, folded: its line in the thread, a click away from open, its
/// name padded to `name_w` columns.
pub(crate) fn line(lay: &mut Lay, m: &super::api::Mail, k: &Key, i: usize, name_w: usize) {
    let name = clip_w(&sender_name(&m.from), NAME).to_string();
    let name = format!("{name}{}", " ".repeat(name_w.saturating_sub(str_w(&name))));
    let date = super::reader::when(&m.date);
    let said = said(&m.rich.body);
    let room = k
        .width
        .saturating_sub(1 + str_w(&name) + 2 + 2 + str_w(&date));
    let said = clip_w(&said, room).to_string();
    let pad = k
        .width
        .saturating_sub(1 + str_w(&name) + 2 + str_w(&said) + str_w(&date));
    let mut cells: CardLine = row(&name, k.ink, true);
    cells.extend(
        "  ".chars()
            .chain(said.chars())
            .map(|c| plain(c, k.muted, false)),
    );
    cells.extend(
        " ".repeat(pad)
            .chars()
            .chain(date.chars())
            .map(|c| plain(c, k.muted, false)),
    );
    lay.folded.push((lay.lines.len(), i));
    lay.lines.push(cells);
}

impl InboxPane {
    /// The folded mail on row `row` of a reader `cols` wide, if one is.
    pub(crate) fn folded_at(&self, cols: u16, row: u16) -> Option<usize> {
        let r = self.reading.as_ref()?;
        let at = r.scroll + usize::from(row);
        let laid = r.laid(super::reader::text_width(cols));
        let found = laid.folded.iter().find(|&&(l, _)| l == at);
        found.map(|&(_, i)| i)
    }

    /// A click there: the folded mail opens. Whether one did.
    pub(crate) fn unfold_at(&mut self, cols: u16, row: u16) -> bool {
        let Some(i) = self.folded_at(cols, row) else {
            return false;
        };
        if let Some(r) = self.reading.as_mut() {
            r.open.insert(i);
            r.folds += 1;
        }
        true
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_folded_mail_starts_in_plain_words() {
        let md = "## Your order\n\n| Item | Qty |\n| --- | --- |\n| **Tea** | 2 |\n\n\u{25a3} logo";
        assert_eq!(super::said(md), "Your order Item Qty Tea 2 logo");
    }
}
