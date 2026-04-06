# Farx

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

### Selection

| Key | Action |
|-----|--------|
| `Space` / `Insert` | Toggle select |
| `Ctrl+A` | Select all |
| `Ctrl+D` | Deselect all |
| `Alt+Up` / `Alt+Down` | Select while moving |

### Sorting

| Key | Action |
|-----|--------|
| `Ctrl+F3` | Sort by name |
| `Ctrl+F4` | Sort by extension |
| `Ctrl+F5` | Sort by size |
| `Ctrl+F6` | Sort by date |

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
| `Ctrl+L` | Info panel |
| `Ctrl+Space` | AI assistant |
| `Alt+F7` | Search files |
| `Alt+S` | Calculate directory / selection size |
| `Ctrl+Y` | Copy file path(s) to clipboard |
| `Alt+E` | Extract archive to other panel |
| `Alt+C` | Compress selection to zip |
| `Ctrl+P` | Fuzzy file finder |
| `Alt+D` | Find duplicate files |
| `Alt+T` | Disk usage treemap |
| `Alt+Enter` | Quick actions palette |
| `Alt+K` | SHA-256 checksum |

## Built-in Editor

Open any file with `F4`. The editor supports:

- Syntax highlighting for common languages
- Undo / redo (`Ctrl+Z` / `Ctrl+Shift+Z`)
- Search (`Ctrl+F`) and replace (`Ctrl+H`)
- Save (`Ctrl+S`), save and exit (`Ctrl+Q`)

## Built-in Viewer

Open any file with `F3`. The viewer supports:

- Syntax-highlighted text files
- Hex dump for binary files
- Line wrapping toggle
- In-file search

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
