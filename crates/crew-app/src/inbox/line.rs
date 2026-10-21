//! One row of the inbox list, built left to right and right to left before
//! any cell exists, so the row can be written whole: every column of a
//! selected row carries the wash (`bgruns` rounds a highlight's outer corners
//! and frosts it on glass), and a chip is a capsule of its own over it.
use crew_render::CellView;

use crate::chatwidth::{char_w, clip_w, str_w};

type Rgb = (u8, u8, u8);

#[derive(Clone, Copy)]
enum Slot {
    Empty,
    /// A glyph; a wide one's second column is [`Slot::Cont`].
    Glyph {
        c: char,
        fg: Rgb,
        bold: bool,
        chip: bool,
    },
    Cont,
}

pub(crate) struct Line {
    slots: Vec<Slot>,
}

impl Line {
    /// A row `width` columns wide (the pane's, less its margin column).
    pub(crate) fn new(width: u16) -> Self {
        Self {
            slots: vec![Slot::Empty; usize::from(width)],
        }
    }

    pub(crate) fn width(&self) -> u16 {
        self.slots.len() as u16
    }

    /// `s` from `col`, cut with `…` before `end`; the column after it.
    pub(crate) fn put(&mut self, col: u16, end: u16, s: &str, fg: Rgb, bold: bool) -> u16 {
        self.write(col, end, s, fg, bold, false)
    }

    /// `s` as a chip (a capsule under its glyphs); the column after it.
    pub(crate) fn chip(&mut self, col: u16, end: u16, s: &str, fg: Rgb) -> u16 {
        self.write(col, end, s, fg, false, true)
    }

    /// `s` ending just before `end`, when it fits after `floor`: the column
    /// it starts at, or `end` when it was left out.
    pub(crate) fn right(&mut self, floor: u16, end: u16, s: &str, fg: Rgb, bold: bool) -> u16 {
        let w = str_w(s) as u16;
        match end
            .checked_sub(w)
            .filter(|&at| at >= floor && !s.is_empty())
        {
            Some(at) => {
                self.write(at, end, s, fg, bold, false);
                at
            }
            None => end,
        }
    }

    fn write(&mut self, col: u16, end: u16, s: &str, fg: Rgb, bold: bool, chip: bool) -> u16 {
        let end = end.min(self.width());
        if col >= end {
            return col;
        }
        let mut x = col;
        for c in clip_w(s, usize::from(end - col)).chars() {
            let w = char_w(c) as u16;
            if w == 0 || x + w > end {
                continue;
            }
            self.slots[usize::from(x)] = Slot::Glyph { c, fg, bold, chip };
            if w == 2 {
                self.slots[usize::from(x) + 1] = Slot::Cont;
            }
            x += w;
        }
        x
    }

    /// The row's cells on `row`: on `wash` every column, else only its glyphs.
    /// A chip wears `chip_bg` as a capsule.
    pub(crate) fn emit(self, out: &mut Vec<CellView>, row: u16, wash: Option<Rgb>, chip_bg: Rgb) {
        let page = crew_theme::theme().page_bg;
        for (x, slot) in self.slots.into_iter().enumerate() {
            let (c, fg, bold, chip) = match slot {
                Slot::Cont => continue,
                Slot::Empty if wash.is_none() => continue,
                Slot::Empty => (' ', page, false, false),
                Slot::Glyph { c, fg, bold, chip } => (c, fg, bold, chip),
            };
            out.push(CellView {
                col: x as u16,
                row,
                c,
                fg,
                bg: match chip {
                    true => chip_bg,
                    false => wash.unwrap_or(page),
                },
                bold,
                mark: chip,
                ..Default::default()
            });
        }
    }
}
