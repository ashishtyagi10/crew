use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyModifiers};

use crate::action::Action;

type Bindings = HashMap<(KeyCode, KeyModifiers), Action>;

/// Tools and dialogs: dir size, undo, batch rename, fuzzy finder, AI panel,
/// quick actions, duplicates, touch, stats, treemap, checksums, archives,
/// chmod, symlink.
pub(super) fn build_tools(panel: &mut Bindings) {
    // Directory size
    panel.insert(
        (KeyCode::Char('s'), KeyModifiers::ALT),
        Action::CalculateDirSize,
    );

    // Undo
    panel.insert((KeyCode::Char('z'), KeyModifiers::CONTROL), Action::Undo);

    // Batch rename
    panel.insert(
        (KeyCode::Char('m'), KeyModifiers::CONTROL),
        Action::BatchRename,
    );

    // Fuzzy finder
    panel.insert(
        (KeyCode::Char('p'), KeyModifiers::CONTROL),
        Action::ShowFuzzyFinder,
    );

    // AI tools panel
    panel.insert(
        (KeyCode::Char('e'), KeyModifiers::CONTROL),
        Action::ShowAiPanel,
    );

    // Quick actions
    panel.insert(
        (KeyCode::Enter, KeyModifiers::ALT),
        Action::ShowQuickActions,
    );

    // Duplicate finder
    panel.insert(
        (KeyCode::Char('d'), KeyModifiers::ALT),
        Action::FindDuplicates,
    );

    // Touch file
    panel.insert((KeyCode::Char('w'), KeyModifiers::ALT), Action::TouchFile);

    // File statistics
    panel.insert(
        (KeyCode::Char('i'), KeyModifiers::ALT),
        Action::ShowFileStats,
    );

    // Disk usage treemap
    panel.insert((KeyCode::Char('t'), KeyModifiers::ALT), Action::ShowTreemap);

    // Checksums
    panel.insert(
        (KeyCode::Char('k'), KeyModifiers::ALT),
        Action::ShowChecksums,
    );

    // Archives
    panel.insert(
        (KeyCode::Char('e'), KeyModifiers::ALT),
        Action::ExtractArchive,
    );
    panel.insert(
        (KeyCode::Char('c'), KeyModifiers::ALT),
        Action::CompressSelection,
    );

    // File permissions (chmod)
    panel.insert((KeyCode::Char('a'), KeyModifiers::ALT), Action::ChmodDialog);

    // Symlink
    panel.insert(
        (KeyCode::Char('l'), KeyModifiers::ALT),
        Action::CreateSymlinkDialog,
    );
}

/// Invert selection and select/deselect by mask.
pub(super) fn build_mask_selection(panel: &mut Bindings) {
    panel.insert(
        (KeyCode::Char('*'), KeyModifiers::ALT),
        Action::InvertSelection,
    );
    panel.insert(
        (
            KeyCode::Char('*'),
            KeyModifiers::ALT.union(KeyModifiers::SHIFT),
        ),
        Action::InvertSelection,
    );

    // Alt+= (easier to press) and Alt++ (Shift+= on most keyboards)
    panel.insert(
        (KeyCode::Char('+'), KeyModifiers::ALT),
        Action::SelectByMaskDialog,
    );
    panel.insert(
        (
            KeyCode::Char('+'),
            KeyModifiers::ALT.union(KeyModifiers::SHIFT),
        ),
        Action::SelectByMaskDialog,
    );
    panel.insert(
        (KeyCode::Char('='), KeyModifiers::ALT),
        Action::SelectByMaskDialog,
    );
    panel.insert(
        (KeyCode::Char('-'), KeyModifiers::ALT),
        Action::DeselectByMaskDialog,
    );
}
