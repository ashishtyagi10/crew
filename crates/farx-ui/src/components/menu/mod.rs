mod action;
mod definitions;
mod render;
mod state;

pub use action::MenuAction;
pub use render::render_menu;
pub use state::MenuState;

pub(super) struct MenuItem {
    pub label: &'static str,
    pub action: MenuAction,
    pub hotkey: &'static str,
}

pub(super) struct MenuColumn {
    pub title: &'static str,
    pub items: Vec<MenuItem>,
}
