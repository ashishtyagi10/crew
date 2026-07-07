//! Frame-to-frame reuse of shaped pane buffers. Shaping (cosmic-text layout)
//! dominates `set_scene` cost; a pane whose content signature is unchanged
//! reuses last frame's `Buffer` untouched, so e.g. the sidebar's once-a-second
//! tick reshapes one pane instead of every pane on screen.
use std::hash::{Hash, Hasher};

use crate::celltext::FontParams;
use crate::scene::{PaneBuffer, PaneScene};

/// The previous frame's shaped buffers + their signatures, consumed for reuse.
pub(crate) type PrevPass = (Vec<u64>, Vec<PaneBuffer>);

/// Everything that affects a pane's SHAPED buffer (not its position): the
/// cells, grid dims, pixel size, and font parameters. Same signature = the
/// previous frame's buffer can be reused as-is (a moved pane reuses too).
pub(crate) fn pane_sig(pane: &PaneScene, cols: usize, rows: usize, params: &FontParams) -> u64 {
    let mut h = std::hash::DefaultHasher::new();
    pane.cells.hash(&mut h);
    (cols, rows).hash(&mut h);
    (pane.w.to_bits(), pane.h.to_bits()).hash(&mut h);
    (
        params.font_size.to_bits(),
        params.line_height.to_bits(),
        params.cell_w.to_bits(),
        params.weight,
    )
        .hash(&mut h);
    params.family.hash(&mut h);
    h.finish()
}

/// One render pass's retained buffers and their signatures.
#[derive(Default)]
pub(crate) struct SceneSlots {
    sigs: Vec<u64>,
    bufs: Vec<PaneBuffer>,
}

impl SceneSlots {
    /// Hand the previous frame's state to `build_scene`, leaving this empty.
    pub fn take_prev(&mut self) -> PrevPass {
        (
            std::mem::take(&mut self.sigs),
            std::mem::take(&mut self.bufs),
        )
    }

    /// Store this frame's results for the next frame.
    pub fn set(&mut self, sigs: Vec<u64>, bufs: Vec<PaneBuffer>) {
        self.sigs = sigs;
        self.bufs = bufs;
    }

    /// The retained buffers, for the draw path.
    pub fn bufs(&self) -> &[PaneBuffer] {
        &self.bufs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn take_prev_empties_the_slots() {
        let mut s = SceneSlots::default();
        s.set(vec![1, 2], Vec::new());
        let (sigs, bufs) = s.take_prev();
        assert_eq!(sigs, vec![1, 2]);
        assert!(bufs.is_empty());
        assert!(s.bufs().is_empty());
        assert!(s.take_prev().0.is_empty(), "second take sees empty state");
    }
}
