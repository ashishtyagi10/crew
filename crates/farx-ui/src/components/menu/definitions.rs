use super::{MenuAction, MenuColumn, MenuItem};

pub(super) fn build_menus() -> Vec<MenuColumn> {
    vec![
        MenuColumn {
            title: " Left ",
            items: vec![
                MenuItem {
                    label: "Sort by Name",
                    action: MenuAction::SortByName,
                    hotkey: "Ctrl+F3",
                },
                MenuItem {
                    label: "Sort by Extension",
                    action: MenuAction::SortByExtension,
                    hotkey: "Ctrl+F4",
                },
                MenuItem {
                    label: "Sort by Size",
                    action: MenuAction::SortBySize,
                    hotkey: "Ctrl+F5",
                },
                MenuItem {
                    label: "Sort by Date",
                    action: MenuAction::SortByDate,
                    hotkey: "Ctrl+F6",
                },
                MenuItem {
                    label: "─────────────",
                    action: MenuAction::None,
                    hotkey: "",
                },
                MenuItem {
                    label: "Toggle Hidden",
                    action: MenuAction::ToggleHidden,
                    hotkey: "Ctrl+H",
                },
                MenuItem {
                    label: "Refresh",
                    action: MenuAction::Refresh,
                    hotkey: "Ctrl+R",
                },
            ],
        },
        MenuColumn {
            title: " Files ",
            items: vec![
                MenuItem {
                    label: "View",
                    action: MenuAction::ViewFile,
                    hotkey: "F3",
                },
                MenuItem {
                    label: "Edit",
                    action: MenuAction::EditFile,
                    hotkey: "F4",
                },
                MenuItem {
                    label: "Copy",
                    action: MenuAction::CopyFile,
                    hotkey: "F5",
                },
                MenuItem {
                    label: "Move/Rename",
                    action: MenuAction::MoveFile,
                    hotkey: "F6",
                },
                MenuItem {
                    label: "Make Directory",
                    action: MenuAction::MkDir,
                    hotkey: "F7",
                },
                MenuItem {
                    label: "Delete",
                    action: MenuAction::DeleteFile,
                    hotkey: "F8",
                },
            ],
        },
        MenuColumn {
            title: " Commands ",
            items: vec![
                MenuItem {
                    label: "Find Files",
                    action: MenuAction::FindFiles,
                    hotkey: "Alt+F7",
                },
                MenuItem {
                    label: "AI Assistant",
                    action: MenuAction::ShowAiBar,
                    hotkey: "Ctrl+Space",
                },
                MenuItem {
                    label: "AI Coding Tools",
                    action: MenuAction::ShowAiPanel,
                    hotkey: "Ctrl+E",
                },
                MenuItem {
                    label: "Swap Panels",
                    action: MenuAction::SwapPanels,
                    hotkey: "",
                },
            ],
        },
        MenuColumn {
            title: " Options ",
            items: vec![MenuItem {
                label: "Toggle Fn Bar",
                action: MenuAction::ToggleFnBar,
                hotkey: "",
            }],
        },
    ]
}
