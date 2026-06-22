# Farx

> **This repo is pivoting to [Crew](docs/CREW.md)** — a from-scratch native GPU
> terminal (Rust + wgpu). Run it with `cargo run --release -p crew-app`. The
> Farx file-manager docs below are retained for the legacy crates.

A next-generation, cross-platform terminal file manager inspired by FAR Manager. Built in Rust with dual-panel navigation, a built-in editor/viewer, AI assistant, and five beautiful themes.

Works on **macOS**, **Linux**, and **Windows**.

## Install

### Quick install (macOS / Linux)

```sh
curl -sSfL https://raw.githubusercontent.com/ashishtyagi10/farx/main/install.sh | sh
```

### From GitHub Releases

Download the latest binary for your platform from the [Releases page](https://github.com/ashishtyagi10/farx/releases), extract it, and move it to a directory in your `PATH`.

| Platform | Asset |
|----------|-------|
| macOS (Apple Silicon) | `farx-v*-aarch64-apple-darwin.tar.gz` |
| macOS (Intel) | `farx-v*-x86_64-apple-darwin.tar.gz` |
| Linux (x86_64) | `farx-v*-x86_64-unknown-linux-gnu.tar.gz` |
| Linux (ARM64) | `farx-v*-aarch64-unknown-linux-gnu.tar.gz` |
| Windows (x86_64) | `farx-v*-x86_64-pc-windows-msvc.zip` |

### Build from source

```sh
git clone https://github.com/ashishtyagi10/farx.git
cd farx
cargo build --release
# Binary is at target/release/farx
```

## Auto-Update

Farx checks for updates in the background every time it starts. If a newer version is available, you'll see a notification.

```sh
farx --update         # Download and install the latest version
farx --check-update   # Check without installing
```

## Usage

```sh
farx                  # Launch the file manager
farx --version        # Print version
farx --keydebug       # Debug terminal key events
```

## Keyboard Shortcuts

### Navigation

| Key | Action |
|-----|--------|
| `Up` / `Down` | Move cursor |
| `PageUp` / `PageDown` | Scroll page |
| `Home` / `End` | Jump to first / last entry |
| `Enter` | Enter directory or execute command |
| `Left` / `Right` | Collapse / expand tree node |
| `Tab` | Switch panels |
| `Ctrl+PageUp` | Parent directory |
| `Ctrl+\` | Go to root |
| `Alt+Left` | Go back in directory history |
| `Alt+Right` | Go forward in directory history |

### File Operations

| Key | Action |
|-----|--------|
| `F2` | Open with system app |
| `F3` | View file |
| `F4` | Edit file |
| `F5` | Copy to other panel |
| `F6` | Move / rename |
| `F7` | Create directory |
| `F8` | Delete |
| `Shift+F4` | Create new file |
| `Shift+F5` | Copy to same directory |
| `Shift+F6` | Rename |
| `Ctrl+M` | Batch rename (regex find/replace) |
| `Ctrl+Z` | Undo last file operation |
| `Alt+L` | Create symbolic link |
| `Alt+A` | File permissions (chmod) |

### Selection

| Key | Action |
|-----|--------|
| `Space` / `Insert` | Toggle select |
| `Ctrl+A` | Select all |
| `Ctrl+D` | Deselect all |
| `Alt+Up` / `Alt+Down` | Select while moving |
| `Alt++` | Select by pattern/mask |
| `Alt+-` | Deselect by pattern/mask |
| `Alt+*` | Invert selection |

### Sorting

| Key | Action |
|-----|--------|
| `Ctrl+F3` | Sort by name (press again to toggle asc/desc) |
| `Ctrl+F4` | Sort by extension (press again to toggle asc/desc) |
| `Ctrl+F5` | Sort by size (press again to toggle asc/desc) |
| `Ctrl+F6` | Sort by date (press again to toggle asc/desc) |

### Bookmarks

| Key | Action |
|-----|--------|
| `Ctrl+B` | Open bookmarks panel |
| `Alt+B` | Bookmark current directory |

### Filter

| Key | Action |
|-----|--------|
| `Ctrl+F` | Filter / narrow directory listing |

### Other

| Key | Action |
|-----|--------|
| `F1` | Help |
| `F9` | Menu |
| `F10` | Quit |
| `Ctrl+H` | Toggle hidden files |
| `Ctrl+R` | Refresh |
| `Ctrl+O` | Toggle panels / console |
| `Ctrl+L` | Info panel (with file preview) |
| `Ctrl+U` | Swap panels |
| `Ctrl+Space` | AI assistant |
| `Alt+F7` | Search files |
| `Alt+S` | Calculate directory / selection size |
| `Ctrl+Y` | Copy file path(s) to clipboard |
| `Alt+Y` | Copy file name(s) to clipboard |
| `Ctrl+\`` | Open terminal here |
| `Alt+H` | Recent directories |
| `Alt+E` | Extract archive to other panel |
| `Alt+C` | Compress selection to zip |
| `Ctrl+P` | Fuzzy file finder |
| `Alt+D` | Find duplicate files |
| `Alt+T` | Disk usage treemap |
| `Alt+Enter` | Quick actions palette |
| `Alt+K` | SHA-256 checksum |
| `Ctrl+G` | Go to directory |
| `Ctrl+F9` | Compare directories |
| `Alt+W` | Touch file (update mtime) |
| `Alt+I` | File statistics (line/word/byte count) |
| `Ctrl+T` | New tab (current directory) |
| `Ctrl+W` | Close tab |
| `Ctrl+Tab` | Next tab |
| `Alt+1`..`Alt+9` | Switch to tab by number |

## Built-in Editor

Open any file with `F4`. The editor supports:

- Syntax highlighting for common languages
- Undo / redo (`Ctrl+Z` / `Ctrl+Shift+Z`)
- Search (`Ctrl+F` or `F7`), find next (`F3`)
- Go to line (`Ctrl+G`)
- Save (`Ctrl+S` or `F2`), save and exit (`Ctrl+Q` or `Shift+F2`)

## Built-in Viewer

Open any file with `F3`. The viewer supports:

- Syntax-highlighted text files
- Hex dump for binary files, with hex/text toggle (`Ctrl+H`)
- Line wrapping toggle (`Ctrl+W`)
- Follow/tail mode (`Ctrl+F`) — auto-scrolls as file grows, like `tail -f`
- Text search (`/` or `F7`), find next (`n`)
- Go to line (`Ctrl+G`)

## AI Assistant

Press `Ctrl+Space` to open the AI bar and ask questions in plain English — for example *"find all log files larger than 10MB"* or *"show me recently modified configs"*.

Farx also provides AI-powered typeahead suggestions as you type commands.

### Configuring AI

Create a config file at `~/.config/farx/config.toml`:

**OpenRouter (free tier)**
```toml
[ai]
enabled = true
provider = "openrouter"
base_url = "https://openrouter.ai/api/v1"
model = "google/gemma-3-4b-it:free"
api_key_env = "OPENROUTER_API_KEY"
```

**Anthropic**
```toml
[ai]
enabled = true
provider = "anthropic"
base_url = "https://api.anthropic.com/v1"
model = "claude-sonnet-4-20250514"
api_key_env = "ANTHROPIC_API_KEY"
```

**Ollama (local)**
```toml
[ai]
enabled = true
provider = "openai-compatible"
base_url = "http://localhost:11434/v1"
model = "llama3.2"
api_key_env = "OLLAMA_API_KEY"
```

**OpenAI**
```toml
[ai]
enabled = true
provider = "openai-compatible"
base_url = "https://api.openai.com/v1"
model = "gpt-4o-mini"
api_key_env = "OPENAI_API_KEY"
```

Then set the corresponding environment variable (e.g. `export OPENROUTER_API_KEY=sk-...`).

## Agent Tiles

Farx runs coding agents and shells as tiles in an auto-arranging grid. Launch one
with `/claude`, `/codex`, `/gemini`, `/copilot`, `/opencode`, or `/shell` (each
takes an optional directory, e.g. `/claude ~/project`). Manage the grid from the
command line:

| Command | Action |
|---------|--------|
| `/agents` or `/ls` | List running tiles with their number, title, state (focused / minimized / exited), and working directory |
| `/focus <n>` or `/f <n>` | Focus the tile numbered `n` (the `[n]` shown in each tile's title bar); promotes a minimized tile back into the grid |
| `/title <name>` | Rename the focused tile — handy for telling apart several agents of the same kind |
| `/next` / `/prev` | Cycle focus forward / backward through the tiles |
| `/last` | Jump back to the previously-focused tile (ping-pong between two agents) |
| `/restart` | Respawn the focused tile's program in its original directory — useful to revive an exited agent |
| `/clear` / `/clearall` | Reset the focused tile's view / every tile's view |
| `/only` | Close every tile except the focused one |
| `/close` or `/x` | Close the focused tile; `/closeall` closes every tile |

Each tile's title bar shows its number as `[n]`, its title, and its working
directory (e.g. `[1] claude — myproject`) so you can jump straight to it with
`/focus n`. Press `F1` to focus the command input from anywhere (even while an
agent has the keyboard), and `F2` to cycle focus between tiles.

Scroll back through a tile's output with the **mouse wheel** — hover over any
tile and scroll up to walk through its history (up to 1000 lines). The title
shows `(↑N)` while you're viewing history; typing snaps back to the live bottom.

## Bookmarks

Press `Alt+B` to bookmark the current directory. Press `Ctrl+B` to open the bookmarks panel — navigate with arrow keys, press Enter to jump to a bookmark, or Delete/F8 to remove one. Bookmarks are persisted in `~/.config/farx/bookmarks.json`.

You can also use `/bookmark` or `/bm` from the command line.

## Filter

Press `Ctrl+F` to activate the filter bar. Type to narrow the file listing in real time — only files matching your input remain visible (directories are always shown). Press Enter to accept, Esc to clear. Use `/filter <pattern>` from the command line for a one-shot filter.

## Directory Size

Press `Alt+S` to calculate the total size of the directory under the cursor. If files are selected, it calculates the combined size of all selected items. Use `/size` from the command line.

## Directory History

Each panel maintains a navigation history. Press `Alt+Left` to go back to the previously visited directory and `Alt+Right` to go forward. History is cleared when you navigate to a new directory from an older position (like a browser). Use `/back` and `/forward` from the command line.

## Trash Support

By default, deleted files are moved to the system trash (Recycle Bin on Windows, Trash on macOS/Linux) instead of being permanently deleted. Set `use_trash = false` in config to permanently delete instead.

## Clipboard

Press `Ctrl+Y` to copy the path of the file under the cursor to the system clipboard. If files are selected, all selected paths are copied (one per line). Use `/yank` from the command line.

## Git Status Overlay

When browsing inside a git repository, files show colored status indicators at the end of each filename:
- **M** (orange) — modified
- **S** (green) — staged
- **?** (gray) — untracked
- **!** (red) — conflict
- **D** (red) — deleted
- **R** (blue) — renamed

Status is refreshed automatically on directory navigation.

## Archives

Press `Enter` on a `.zip`, `.tar`, `.tar.gz`, or `.tgz` file to browse its contents. Use `Alt+E` to extract the archive under the cursor to the other panel, or `Alt+C` to compress selected files into a zip archive. Slash commands: `/extract`, `/compress`, `/zip`.

## Plugins

Farx supports Lua plugins. Place `.lua` files in `~/.config/farx/plugins/`. Plugins register commands that become available as slash commands:

```lua
-- ~/.config/farx/plugins/hello.lua
farx.register_command("hello", "Say hello", [[
    farx.message("Hello from Farx plugin!")
]])
```

Use `/plugin` to list loaded commands, or `/hello` to run the example above.

## Undo

Press `Ctrl+Z` to undo the last file operation (move, rename, mkdir, create file). Moves are reversed, renames are swapped back. For trashed files, you'll be directed to the system trash. Use `/undo` from the command line.

## Batch Rename

Press `Ctrl+M` to open the batch rename dialog for selected files (or all files in the directory). Enter a regex find pattern and replacement text — the preview updates in real time showing old → new names. Press Enter to apply. Use `/rename-batch` from the command line.

## Fuzzy Finder

Press `Ctrl+P` to open the fuzzy file finder. Type to search recursively across all files from the current directory. Matches are scored by consecutive characters and word boundaries. Press Enter to navigate to the selected file. Use `/find-file` or `/ff` from the command line.

## Duplicate File Finder

Press `Alt+D` to scan the current directory tree for duplicate files. Uses a two-pass approach: groups by file size first, then verifies with SHA-256 checksums. Results show grouped duplicates with total reclaimable space. Use `/duplicates` from the command line.

## Disk Usage Treemap

Press `Alt+T` to see a visual bar-chart of disk usage for the current directory. Each entry shows a proportional bar, percentage, size, and name — sorted largest first. Use `/treemap` from the command line.

## SSH Remote Browsing

Use `/ssh user@host:/path` to list files on a remote server. Works with your existing SSH config and keys. Example: `/ssh myserver:~/projects`.

## Live File Watching

Directory listings auto-refresh when files are created, modified, or deleted by external programs. No manual refresh needed — changes from builds, git operations, or other tools appear automatically.

## Quick Actions Palette

Press `Alt+Enter` to see context-aware actions for the file under the cursor. Actions adapt to the file type — for example, Rust files offer "Cargo check/test/run", Python files offer "Run with Python" and "Lint", shell scripts offer "Run" and "Make executable". Use `/actions` from the command line.

## Checksums

Press `Alt+K` to compute and display the SHA-256 checksum for the file under the cursor or all selected files. Use `/checksum` or `/sha256` from the command line.

## Select by Pattern

Press `Alt++` to select files matching a glob pattern (e.g. `*.rs`, `test*`, `*.log`). Press `Alt+-` to deselect by pattern. Supports `*` (any characters) and `?` (single character) wildcards. Use `/select <pattern>` and `/deselect <pattern>` from the command line.

## Symbolic Links

Press `Alt+L` to create a symbolic link to the file under the cursor. A dialog prompts for the link name. Works on Unix and Windows. Use `/symlink` or `/ln` from the command line.

## Invert Selection

Press `Alt+*` to invert the current selection — selected files become deselected and vice versa. Use `/invert` from the command line.

## Go to Directory

Press `Ctrl+G` to open a dialog where you can type any directory path and jump directly to it. Supports `~` expansion for home directory. Use `/goto <path>`, `/go <path>`, or `/g <path>` from the command line.

## Compare Directories

Press `Ctrl+F9` to compare the active panel against the other panel. Files that exist only in the active panel, or that differ by size or modification time, get selected. This is useful for syncing directories or finding what changed. Use `/compare` or `/cmp` from the command line.

## Swap Panels

Press `Ctrl+U` to swap left and right panel contents (directories, files, and tree state). Use `/swap` from the command line.

## Open with System App

Press `F2` to open the file or directory under the cursor with your OS's default application (macOS `open`, Linux `xdg-open`, Windows `start`). Use `/open` from the command line.

## File Preview

Press `Ctrl+L` to toggle the info panel. It now shows a live preview of the file under the cursor: name, size, modification date, and the first 30 lines of text content. Binary files show a hex dump summary. Files larger than 5 MB show a placeholder.

## Touch File

Press `Alt+W` to update the modification time of the file under the cursor (or all selected files) to the current time. If the file doesn't exist, it creates it (like Unix `touch`). Use `/touch` from the command line.

## File Permissions

File listings now show Unix permission bits (e.g. `rwxr-xr-x`) next to the file size on Unix platforms.

## Copy File Names

Press `Alt+Y` to copy just the filename(s) (not full paths) to the clipboard. If files are selected, all selected names are copied (one per line). Complements `Ctrl+Y` which copies full paths. Use `/yank-names` or `/copy-names` from the command line.

## Open Terminal Here

Press `` Ctrl+` `` to open a new terminal window in the current panel's directory. macOS opens Terminal.app, Windows opens cmd, Linux uses `$TERMINAL` or xterm. Use `/terminal` or `/term` from the command line.

## Recent Directories

Press `Alt+H` to see a list of recently visited directories from the navigation history. The current directory is marked with `*`. Use `/recent` or `/history` from the command line, and `/goto` to jump to any listed path.

## Mouse Support

Farx supports full mouse interaction:
- **Left-click** on a file to move the cursor and switch panels
- **Double-click** to enter a directory or open a file
- **Right-click** to toggle file selection
- **Scroll wheel** scrolls whichever panel or agent tile the mouse is over (over a tile it walks back through the terminal's scrollback history)
- **Click breadcrumb** segments in the path bar to navigate to ancestors
- **Click fn-bar** buttons to trigger the corresponding action

## File Permissions (chmod)

Press `Alt+A` to open the permissions dialog for the file under the cursor. Toggle read/write/execute bits for owner/group/other with Space, navigate with arrow keys. Shows the octal representation live. Press Enter to apply. Use `/chmod` or `/permissions` from the command line. Unix only.

## Content Search (Grep)

The search dialog (`Alt+F7`) has a "Containing text" field for searching inside files. Results now show matching line numbers and text content. Use `/grep` from the command line to open the search dialog with the content field pre-focused.

## Tabs

Each panel supports multiple directory tabs:
- `Ctrl+T` — open a new tab in the current directory
- `Ctrl+W` — close the active tab
- `Ctrl+Tab` — switch to the next tab
- `Alt+1`..`Alt+9` — jump to tab by number

A tab bar appears at the top of the panel when multiple tabs are open.

## File Diff

Use `/diff` to compare the file under the cursor in the left panel against the file under the cursor in the right panel. Shows a side-by-side view with color-coded differences (red=removed, green=added, yellow=changed). Scroll with Up/Down/PgUp/PgDn, close with Esc.

## Image Preview

When the info panel (`Ctrl+L`) is open and the cursor is on an image file (PNG, JPG, GIF, BMP, WEBP, etc.), Farx renders a scaled thumbnail preview using Unicode half-block characters. Works in all terminals. Shows image dimensions alongside the preview.

## Copy/Move Progress

When copying or moving multiple files, a progress dialog shows the current file, file/byte counters, and a visual progress bar. The operation runs in a background thread so the UI stays responsive.

## Themes

Set the theme in `~/.config/farx/config.toml`:

```toml
[ui]
theme = "tokyo-night"
```

Available themes: `far-classic`, `tokyo-night`, `catppuccin`, `dracula`, `gruvbox`

## Configuration

Farx looks for `config.toml` in `~/.config/farx/` (or `$XDG_CONFIG_HOME/farx/` on Linux, `~/Library/Application Support/farx/` on macOS). All settings are optional and fall back to defaults.

```toml
[general]
confirm_delete = true       # Prompt before deleting
confirm_overwrite = true    # Prompt before overwriting
show_hidden_files = false   # Show dotfiles
use_trash = true            # Move to trash instead of permanent delete
editor = "internal"         # "internal" or path to external editor
viewer = "internal"         # "internal" or path to external viewer

[ui]
theme = "tokyo-night"       # Theme name
tick_rate_ms = 250          # Refresh interval in ms
show_fn_bar = true          # Show function key bar
date_format = "%Y-%m-%d %H:%M"

[panels]
directories_first = true    # Directories before files
default_sort = "name"       # "name", "extension", "size", or "date"

[keybindings]
# Remap any key to any action. Key format: "Modifier+Key"
# "Ctrl+E" = "edit"
# "Alt+G" = "goto"
# "F12" = "quit"

[ai]
enabled = false
provider = "openrouter"
base_url = "https://openrouter.ai/api/v1"
model = "google/gemma-3-4b-it:free"
max_tokens = 4096
api_key_env = "OPENROUTER_API_KEY"
```

## Architecture

Farx is organized as a Cargo workspace with six crates:

| Crate | Purpose |
|-------|---------|
| `farx-app` | Binary entry point, CLI, auto-update |
| `farx-ui` | TUI components, themes, event loop |
| `farx-core` | Config, keymaps, actions, types |
| `farx-fs` | Filesystem operations |
| `farx-ai` | AI agent (multi-provider LLM integration) |
| `farx-plugin` | Plugin system framework |

## License

MIT or Apache-2.0, at your option.
