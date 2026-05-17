use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// What occupies a leaf node in the panel tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelLeaf {
    /// A file browser panel (Left or Right).
    FilePanel(super::PanelSide),
    /// An embedded terminal session (index into App's terminals vec).
    Terminal(usize),
}

/// Recursive layout tree for panel splitting.
#[derive(Debug, Clone)]
pub enum LayoutNode {
    /// A single panel (file browser or terminal).
    Leaf(PanelLeaf),
    /// Two panels split in a direction.
    Split {
        direction: Direction,
        first: Box<LayoutNode>,
        second: Box<LayoutNode>,
    },
}

impl LayoutNode {
    /// Create the default two-panel layout.
    pub fn default_layout() -> Self {
        LayoutNode::Split {
            direction: Direction::Horizontal,
            first: Box::new(LayoutNode::Leaf(PanelLeaf::FilePanel(
                super::PanelSide::Left,
            ))),
            second: Box::new(LayoutNode::Leaf(PanelLeaf::FilePanel(
                super::PanelSide::Right,
            ))),
        }
    }

    /// Collect all leaf nodes in order (left-to-right, top-to-bottom).
    pub fn leaves(&self) -> Vec<PanelLeaf> {
        let mut result = Vec::new();
        self.collect_leaves(&mut result);
        result
    }

    fn collect_leaves(&self, out: &mut Vec<PanelLeaf>) {
        match self {
            LayoutNode::Leaf(leaf) => out.push(*leaf),
            LayoutNode::Split { first, second, .. } => {
                first.collect_leaves(out);
                second.collect_leaves(out);
            }
        }
    }

    /// Split the leaf at the given index, adding a new terminal.
    /// The new split alternates direction: H → V → H → V...
    /// Returns true if the split was performed.
    pub fn split_leaf(&mut self, leaf_index: usize, terminal_id: usize) -> bool {
        let mut counter = 0usize;
        // The root split is Horizontal, so first child split should be Vertical
        let parent_dir = match self {
            LayoutNode::Split { direction, .. } => Some(*direction),
            LayoutNode::Leaf(_) => None,
        };
        self.split_leaf_inner(leaf_index, terminal_id, &mut counter, parent_dir)
    }

    fn split_leaf_inner(
        &mut self,
        target: usize,
        terminal_id: usize,
        counter: &mut usize,
        parent_dir: Option<Direction>,
    ) -> bool {
        match self {
            LayoutNode::Leaf(_) => {
                if *counter == target {
                    // Alternate direction from parent
                    let new_dir = match parent_dir {
                        Some(Direction::Horizontal) => Direction::Vertical,
                        _ => Direction::Horizontal,
                    };

                    let original = std::mem::replace(
                        self,
                        LayoutNode::Leaf(PanelLeaf::Terminal(0)), // placeholder
                    );
                    *self = LayoutNode::Split {
                        direction: new_dir,
                        first: Box::new(original),
                        second: Box::new(LayoutNode::Leaf(PanelLeaf::Terminal(terminal_id))),
                    };
                    true
                } else {
                    *counter += 1;
                    false
                }
            }
            LayoutNode::Split {
                direction,
                first,
                second,
            } => {
                let dir = Some(*direction);
                if first.split_leaf_inner(target, terminal_id, counter, dir) {
                    return true;
                }
                second.split_leaf_inner(target, terminal_id, counter, dir)
            }
        }
    }

    /// Remove a terminal leaf from the tree, collapsing the split.
    /// Returns true if the terminal was found and removed.
    pub fn remove_terminal(&mut self, terminal_id: usize) -> bool {
        self.remove_terminal_inner(terminal_id)
    }

    fn remove_terminal_inner(&mut self, terminal_id: usize) -> bool {
        match self {
            LayoutNode::Leaf(_) => false,
            LayoutNode::Split { first, second, .. } => {
                // Check if first child is the target terminal
                if matches!(first.as_ref(), LayoutNode::Leaf(PanelLeaf::Terminal(id)) if *id == terminal_id)
                {
                    *self = *second.clone();
                    return true;
                }
                // Check if second child is the target terminal
                if matches!(second.as_ref(), LayoutNode::Leaf(PanelLeaf::Terminal(id)) if *id == terminal_id)
                {
                    *self = *first.clone();
                    return true;
                }
                // Recurse
                if first.remove_terminal_inner(terminal_id) {
                    return true;
                }
                second.remove_terminal_inner(terminal_id)
            }
        }
    }

    /// Compute the Rect for each leaf node given the total available area.
    pub fn compute_rects(&self, area: Rect) -> Vec<(PanelLeaf, Rect)> {
        let mut result = Vec::new();
        self.compute_rects_inner(area, &mut result);
        result
    }

    fn compute_rects_inner(&self, area: Rect, out: &mut Vec<(PanelLeaf, Rect)>) {
        match self {
            LayoutNode::Leaf(leaf) => {
                out.push((*leaf, area));
            }
            LayoutNode::Split {
                direction,
                first,
                second,
            } => {
                let chunks = Layout::default()
                    .direction(*direction)
                    .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .split(area);
                first.compute_rects_inner(chunks[0], out);
                second.compute_rects_inner(chunks[1], out);
            }
        }
    }

    /// Update terminal IDs after a terminal is removed (shift IDs down).
    pub fn adjust_terminal_ids(&mut self, removed_id: usize) {
        match self {
            LayoutNode::Leaf(PanelLeaf::Terminal(id)) => {
                if *id > removed_id {
                    *id -= 1;
                }
            }
            LayoutNode::Leaf(_) => {}
            LayoutNode::Split { first, second, .. } => {
                first.adjust_terminal_ids(removed_id);
                second.adjust_terminal_ids(removed_id);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PanelSide;

    #[test]
    fn default_layout_has_two_file_panels() {
        let layout = LayoutNode::default_layout();
        let leaves = layout.leaves();
        assert_eq!(
            leaves,
            vec![
                PanelLeaf::FilePanel(PanelSide::Left),
                PanelLeaf::FilePanel(PanelSide::Right)
            ]
        );
    }

    #[test]
    fn split_leaf_adds_terminal_and_alternates_direction() {
        let mut layout = LayoutNode::default_layout();
        assert!(layout.split_leaf(0, 7));

        let leaves = layout.leaves();
        assert_eq!(
            leaves,
            vec![
                PanelLeaf::FilePanel(PanelSide::Left),
                PanelLeaf::Terminal(7),
                PanelLeaf::FilePanel(PanelSide::Right)
            ]
        );

        match &layout {
            LayoutNode::Split { first, .. } => match first.as_ref() {
                LayoutNode::Split { direction, .. } => {
                    assert_eq!(*direction, Direction::Vertical);
                }
                _ => panic!("expected first child to be split after splitting first leaf"),
            },
            _ => panic!("expected root split"),
        }
    }

    #[test]
    fn remove_terminal_collapses_split() {
        let mut layout = LayoutNode::default_layout();
        assert!(layout.split_leaf(0, 2));
        assert!(layout.remove_terminal(2));

        let leaves = layout.leaves();
        assert_eq!(
            leaves,
            vec![
                PanelLeaf::FilePanel(PanelSide::Left),
                PanelLeaf::FilePanel(PanelSide::Right)
            ]
        );
    }

    #[test]
    fn adjust_terminal_ids_shifts_higher_ids() {
        let mut layout = LayoutNode::default_layout();
        assert!(layout.split_leaf(0, 1));
        assert!(layout.split_leaf(1, 3));
        layout.adjust_terminal_ids(1);

        let leaves = layout.leaves();
        assert!(leaves.contains(&PanelLeaf::Terminal(2)));
        assert!(!leaves.contains(&PanelLeaf::Terminal(3)));
    }

    #[test]
    fn compute_rects_returns_area_for_each_leaf() {
        let mut layout = LayoutNode::default_layout();
        assert!(layout.split_leaf(1, 5));

        let rects = layout.compute_rects(Rect::new(0, 0, 120, 40));
        assert_eq!(rects.len(), layout.leaves().len());
        assert!(rects.iter().all(|(_, r)| r.width > 0 && r.height > 0));
    }
}
