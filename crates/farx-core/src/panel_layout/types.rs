/// What occupies a leaf node in the agent grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelLeaf {
    /// A file browser panel (Left or Right).
    FilePanel(crate::PanelSide),
    /// An embedded terminal session (stable id).
    Terminal(usize),
}
