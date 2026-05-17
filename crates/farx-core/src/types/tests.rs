use std::path::PathBuf;

use super::*;

fn entry(name: &str, is_dir: bool, size: u64, ext: Option<&str>) -> FileEntry {
    FileEntry {
        name: name.to_string(),
        path: PathBuf::from(name),
        is_dir,
        is_symlink: false,
        is_hidden: false,
        size,
        modified: None,
        extension: ext.map(|e| e.to_string()),
        readonly: false,
        mode: None,
    }
}

#[test]
fn ai_tool_metadata_is_stable() {
    let all = AiTool::all();
    assert_eq!(all.len(), 5);
    assert_eq!(AiTool::GithubCopilot.command(), ("gh", &["copilot"][..]));
    assert!(AiTool::Codex.label().contains("Codex"));
    assert!(AiTool::ClaudeCode.description().contains("Anthropic"));
}

#[test]
fn panel_state_selection_and_cursor_flow() {
    let mut panel = PanelState::new(PanelSide::Left, PathBuf::from("."));
    panel.entries = vec![
        entry("..", true, 0, None),
        entry("a.txt", false, 1, Some("txt")),
        entry("b.rs", false, 2, Some("rs")),
    ];

    panel.toggle_select();
    assert!(panel.selected.contains(&0));
    assert_eq!(panel.cursor, 1);

    panel.select_move(1);
    assert!(panel.selected.contains(&1));
    assert_eq!(panel.cursor, 2);
}

#[test]
fn sort_entries_dirs_first_then_by_field() {
    let mut panel = PanelState::new(PanelSide::Left, PathBuf::from("."));
    panel.entries = vec![
        entry("b.txt", false, 2, Some("txt")),
        entry("folder", true, 0, None),
        entry("a.txt", false, 1, Some("txt")),
    ];
    panel.sort_field = SortField::Name;
    panel.sort_order = SortOrder::Ascending;
    panel.sort_entries();

    let names: Vec<String> = panel.entries.iter().map(|e| e.name.clone()).collect();
    assert_eq!(names, vec!["folder", "a.txt", "b.txt"]);
}

#[test]
fn quick_search_moves_cursor_to_prefix_match() {
    let mut panel = PanelState::new(PanelSide::Left, PathBuf::from("."));
    panel.entries = vec![
        entry("alpha.txt", false, 1, Some("txt")),
        entry("beta.txt", false, 1, Some("txt")),
        entry("gamma.txt", false, 1, Some("txt")),
    ];
    panel.enter_quick_search('g');
    assert_eq!(panel.cursor, 2);
    assert_eq!(panel.quick_search.as_deref(), Some("g"));
    panel.clear_quick_search();
    assert!(panel.quick_search.is_none());
}
