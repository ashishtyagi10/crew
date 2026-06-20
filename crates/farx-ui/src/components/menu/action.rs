#[derive(Debug, Clone, PartialEq)]
pub enum MenuAction {
    None,
    Close,
    // Panel actions
    SortByName,
    SortByExtension,
    SortBySize,
    SortByDate,
    ToggleHidden,
    Refresh,
    // File actions
    ViewFile,
    EditFile,
    CopyFile,
    MoveFile,
    DeleteFile,
    MkDir,
    // Commands
    FindFiles,
    ShowAiBar,
    ShowAiPanel,
    SwapPanels,
}
