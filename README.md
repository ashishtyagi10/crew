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
