//! The two binding tables the `/keys` overlay lists. Data only: the rendering,
//! the scrolling and the docs-parity contract live in [`crate::help`].
//!
//! They stopped having a height budget when the overlay learned to scroll, so
//! a new binding is a new row here and nothing else has to give way for it.

/// `(keys, description)` rows shown in the overlay.
pub(crate) const BINDINGS: &[(&str, &str)] = &[
    ("Ctrl+Tab / Ctrl+Shift+Tab", "Next / previous pane"),
    ("Cmd+1 … 9", "Jump to pane N"),
    ("Cmd+A / Cmd+.", "Jump to next active / waiting pane"),
    ("Cmd+{ / Cmd+}", "Move pane left / right"),
    (
        "Cmd+\u{2190}\u{2191}\u{2192}\u{2193}",
        "Focus the pane that way on the grid",
    ),
    (
        "Cmd+Shift+\u{2190}\u{2191}\u{2192}\u{2193}",
        "Swap the focused pane with that neighbour",
    ),
    ("Cmd+I / Cmd+T", "Focus the input bar / new shell pane"),
    ("Cmd+, / Cmd+J", "Settings / chat pane"),
    ("Cmd+G / Cmd+Z", "Toggle sidebar / zoom focused pane"),
    ("Cmd+S", "Broadcast to all panes"),
    (
        "Cmd+= / Cmd+- / Cmd+0 / Cmd+wheel",
        "Font size + / - / reset",
    ),
    ("Cmd+C / Cmd+V", "Copy screen / paste"),
    (
        "Cmd+Click",
        "open URL/file/dir · copy a code block in an agent pane",
    ),
    ("Cmd+W / Cmd+M", "Close pane / maximize"),
    ("Cmd+K", "Clear focused pane scrollback"),
    (
        "Ctrl+Shift+L",
        "Cycle themes (dark \u{2192} light \u{2192} crt)",
    ),
    ("Ctrl+Shift+M", "Chat: markdown preview \u{2194} raw source"),
    ("Ctrl+O", "Chat: compact transcript view"),
    ("Shift+PageUp / Shift+PageDown", "Scroll the focused pane"),
    ("Shift+Home / Shift+End", "Scroll to top / bottom"),
    (
        "Drag a card's right border",
        "Scroll it \u{b7} the sidebar's LOG scrolls with the wheel",
    ),
    ("Double-click / Triple-click", "Select the word / the line"),
    (
        "On a card's top border",
        "Double-click zooms \u{b7} drag it onto another card to swap them",
    ),
    (
        "PageUp/Down · Home/End · h · H",
        "Todo list: page · first / last · show done (or the [show N done] button) · history",
    ),
    (
        "Ctrl+A / Ctrl+E / Alt+Left / Alt+Right",
        "Todo composer: draft ends · word jump",
    ),
    ("/ (in input)", "Command palette"),
    (
        "! · * · ? · ?? (in input)",
        "New pane / broadcast / ask ai a command / explain this pane",
    ),
    ("Cmd+Q", "Quit"),
];

/// Keys that mean something specific inside an agent pane. The overlay
/// claimed to list "the bindings" and had none of these — including the two
/// that answer a drafted plan, added in v0.6.46.
pub(crate) const CHAT_BINDINGS: &[(&str, &str)] = &[
    (
        "Enter",
        "Send · answers a pending plan when the composer is empty",
    ),
    (
        "Esc",
        "Discard a pending plan · interrupt a running turn · close",
    ),
    ("Shift+Enter", "Newline instead of sending"),
    ("Tab", "Complete the leading @agent or /construct"),
    ("Ctrl+R", "Reverse-search prompts you've sent"),
    ("Cmd+F / Ctrl+F", "Find in the transcript, jump per match"),
    (
        "Up / Down",
        "Recall a prompt you already sent · navigate an open popup",
    ),
    (
        "Tab / Right",
        "Take the suggested rest of a prompt you sent before",
    ),
    (
        "@ · # (in composer)",
        "Attach an agent, skill or file · remember a note",
    ),
    (
        "@file:120-180 (in composer)",
        "Attach just those lines, instead of the whole file",
    ),
    (
        "@a+b (in composer)",
        "Fan one task out to both agents, in parallel",
    ),
];
