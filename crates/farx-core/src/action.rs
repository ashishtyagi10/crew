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
    SwapPanels,

    // File operations
    OpenSystemApp,
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
    InvertSelection,
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

    // Go to directory
    GotoDirectoryDialog,

    // History
    HistoryBack,
    HistoryForward,
    ShowRecentDirectories,

    // Bookmarks
    ShowBookmarks,
    AddBookmark,

    // Clipboard
    CopyPathToClipboard,
    CopyNameToClipboard,

    // Terminal
    OpenTerminalHere,

    // Touch file
    TouchFile,

    // Filter
    ToggleFilter,

    // Undo
    Undo,

    // Batch rename
    BatchRename,

    // Fuzzy finder
    ShowFuzzyFinder,

    // Archives
    ViewArchive,
    ExtractArchive,
    CompressSelection,

    // Symlink
    CreateSymlinkDialog,

    // Quick actions
    ShowQuickActions,
    /// Execute a shell command in the current directory
    RunShellAction(String),

    // Remote browsing
    SshBrowse(String),

    // Duplicate finder
    FindDuplicates,

    // Compare directories
    CompareDirectories,

    // File statistics
    ShowFileStats,

    // Checksums
    ShowChecksums,

    // File permissions
    ChmodDialog,

    // Disk usage treemap
    ShowTreemap,

    // Directory size
    CalculateDirSize,

    // AI
    AiQuery(String),
    ShowAiPanel,
    LaunchAiTool(crate::types::AiTool),

    // System
    Tick,
    Resize(u16, u16),
    Render,
    Quit,
    Noop,
}
