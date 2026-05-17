use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyModifiers};

use crate::action::Action;
use crate::types::PanelSide;

type Bindings = HashMap<(KeyCode, KeyModifiers), Action>;

/// Global function-key bindings (F1, F9, F10, F11, F12).
pub(super) fn build_global(global: &mut Bindings) {
    global.insert((KeyCode::F(1), KeyModifiers::NONE), Action::ShowHelp);
    global.insert((KeyCode::F(9), KeyModifiers::NONE), Action::ShowMenu);
    global.insert((KeyCode::F(10), KeyModifiers::NONE), Action::Quit);
    global.insert(
        (KeyCode::F(11), KeyModifiers::NONE),
        Action::ShowPluginCommands,
    );
    global.insert((KeyCode::F(12), KeyModifiers::NONE), Action::ShowScreenList);
}

/// Panel function keys (F2-F8) and panel switching.
pub(super) fn build_function_keys(panel: &mut Bindings) {
    panel.insert((KeyCode::F(2), KeyModifiers::NONE), Action::OpenSystemApp);
    panel.insert((KeyCode::F(3), KeyModifiers::NONE), Action::EditFile);
    panel.insert((KeyCode::F(4), KeyModifiers::NONE), Action::SwitchPanel);
    panel.insert((KeyCode::Tab, KeyModifiers::NONE), Action::SwitchPanel);
    panel.insert((KeyCode::BackTab, KeyModifiers::SHIFT), Action::SwitchPanel);
    panel.insert(
        (KeyCode::Left, KeyModifiers::CONTROL),
        Action::FocusLeftPanel,
    );
    panel.insert(
        (KeyCode::Right, KeyModifiers::CONTROL),
        Action::FocusRightPanel,
    );
    panel.insert((KeyCode::F(5), KeyModifiers::NONE), Action::CopyDialog);
    panel.insert((KeyCode::F(6), KeyModifiers::NONE), Action::MoveDialog);
    panel.insert((KeyCode::F(7), KeyModifiers::NONE), Action::MkDirDialog);
    panel.insert((KeyCode::F(8), KeyModifiers::NONE), Action::DeleteDialog);
}

/// Shift+F keys: file ops variants.
pub(super) fn build_shift_function_keys(panel: &mut Bindings) {
    panel.insert(
        (KeyCode::F(4), KeyModifiers::SHIFT),
        Action::CreateFileDialog,
    );
    panel.insert((KeyCode::F(5), KeyModifiers::SHIFT), Action::CopySameDir);
    panel.insert((KeyCode::F(6), KeyModifiers::SHIFT), Action::RenameDialog);
}

/// Alt+F keys: drive menus, search.
pub(super) fn build_alt_function_keys(panel: &mut Bindings) {
    panel.insert(
        (KeyCode::F(1), KeyModifiers::ALT),
        Action::ShowDriveMenu(PanelSide::Left),
    );
    panel.insert(
        (KeyCode::F(2), KeyModifiers::ALT),
        Action::ShowDriveMenu(PanelSide::Right),
    );
    panel.insert((KeyCode::F(7), KeyModifiers::ALT), Action::ShowSearchDialog);
}
