use crate::types::PanelSide;

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    // Navigation
    CursorUp,
    CursorDown,
    CursorPageUp,
    CursorPageDown,
    CursorHome,
    CursorEnd,
    EnterDirectory,
    /// Tree-specific: expand without navigating
    TreeExpand,
    /// Tree-specific: collapse without navigating
    TreeCollapse,
    ParentDirectory,
    GotoRoot,
    SwitchPanel,

    // File operations
    CopyDialog,
    MoveDialog,
    DeleteDialog,
    MkDirDialog,
    RenameDialog,
    CreateFileDialog,
    CopySameDir,

    // Selection
    ToggleSelect,
    SelectUp,
    SelectDown,
    SelectPageUp,
    SelectPageDown,
    SelectHome,
    SelectEnd,
    SelectAll,
    DeselectAll,
    SelectByMaskDialog,
    DeselectByMaskDialog,

    // Sort and display
    SortByName,
    SortByExtension,
    SortBySize,
    SortByDate,
    ToggleHidden,
    RefreshPanel,

    // Views
    ViewFile,
    EditFile,
    TogglePanels,
    ShowInfoPanel,
    ShowMenu,
    ShowHelp,
    ShowDriveMenu(PanelSide),
    ShowSearchDialog,
    ShowAiBar,
    ShowUserMenu,
    ShowPluginCommands,
    ShowScreenList,

    // Command line
    CommandLineInput(char),
    CommandLineBackspace,
    CommandLineExecute,
    /// Enter key: execute command if command line has input, otherwise enter directory
    CommandLineEnterOrDir,
    CommandLineHistoryUp,
    CommandLineHistoryDown,
    CommandLineClear,

    // Quick search
    QuickSearch(char),
    QuickSearchClear,

    // History
    HistoryBack,
    HistoryForward,

    // Bookmarks
    ShowBookmarks,
    AddBookmark,

    // Clipboard
    CopyPathToClipboard,

    // Filter
    ToggleFilter,

    // Directory size
    CalculateDirSize,

    // AI
    AiQuery(String),

    // System
    Tick,
    Resize(u16, u16),
    Render,
    Quit,
    Noop,
}
