# /far Mini-Status Line Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an always-visible one-row status line between the /far panels and the command line showing the active panel's selected entry in full (Midnight Commander mini-status), and remove the old right-aligned name from the command bar.

**Architecture:** `farpane/render.rs` gains a fourth layout row (`Min(3)` panels, then three `Length(1)` rows: status, command, F-keys). A new `status_bar` function in `farpane/bars.rs` renders the existing `selected_label()` output, ellipsizing the name while keeping the ` · size` suffix intact. The `selected` parameter of `command_bar` is then deleted.

**Tech Stack:** Rust, ratatui (rendered to `CellView` cells via `crate::tui::to_cells`), cargo test.

**Spec:** `docs/superpowers/specs/2026-07-23-far-status-line-design.md`

## Global Constraints

- Workspace root: `/Users/atyagi/code/crew`; all paths below are relative to it.
- `crate::tui::to_cells` drops blank cells, so rendered-row string reconstruction loses spaces — assert the **squished** form (e.g. `"…·1B"`, not `"… · 1 B"`). See the comment in `suggested_command_highlights_the_bar_and_shows_the_accept_hint` (`render_tests.rs:346`).
- With the new layout at 24 rows: panels occupy rows 0–20, **status row = 21**, command row = 22 (unchanged), F-key bar = 23. Existing tests using `cmd_row = 22` must keep passing untouched.
- Run tests with `cargo test -p crew-app farpane` (test files are `#[path]`-mounted child modules).
- Pre-commit hook runs `cargo fmt --check` and `cargo check`; run `cargo fmt` before every commit.

---

### Task 1: Status line row (`status_bar`) + layout

**Files:**
- Modify: `crates/crew-app/src/farpane/bars.rs` (add `status_bar` + `ellipsize_keeping_suffix`)
- Modify: `crates/crew-app/src/farpane/render.rs:19-94` (layout + call)
- Test: `crates/crew-app/src/farpane/render_tests.rs`

**Interfaces:**
- Consumes: `bars::selected_label(&Panel) -> Option<String>` (exists, unchanged: `"name/"` for dirs, `"name · size"` for files, `None` on empty listing).
- Produces: `pub(super) fn status_bar(buf: &mut Buffer, area: Rect, label: Option<&str>)` in `bars.rs`, re-exported through the existing `use bars::{...}` line in `render.rs`. Task 2 relies on `render.rs` already passing `sel_label.as_deref()` to `status_bar`.

- [ ] **Step 1: Write the failing tests**

In `crates/crew-app/src/farpane/render_tests.rs`, REPLACE the whole test `command_bar_carries_the_selected_entrys_full_name` (lines 412–425) with:

```rust
#[test]
fn status_line_carries_the_selected_entrys_full_name() {
    // Listing rows truncate long names to fit beside the size column; the
    // status line above the command bar shows the active panel's selection
    // in full.
    let long = "a_very_long_filename_here.txt"; // 29 chars: fits a 40-col row
    let base = std::env::temp_dir().join("crew_far_render_statusname");
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&base).unwrap();
    std::fs::write(base.join(long), b"x").unwrap();
    let mut p = FarPane::new(base);
    p.left.sel = 1; // 0 is "..", 1 is the file
    let cells = render(&p, 40, 24);
    let status_row = 21; // rows(24) - cmd(1) - fkeys(1) - status(1)
    let mut row: Vec<(u16, char)> = cells
        .iter()
        .filter(|c| c.row == status_row)
        .map(|c| (c.col, c.c))
        .collect();
    row.sort_unstable_by_key(|(col, _)| *col);
    let line: String = row.into_iter().map(|(_, c)| c).collect();
    assert!(line.contains(long), "full name missing from status row: {line:?}");
    // The panel column itself must have truncated it (else this test proves
    // nothing) — the full name may appear on no other row.
    let t = text(&cells);
    assert_eq!(
        t.lines().filter(|l| l.contains(long)).count(),
        1,
        "panel row should truncate the long name:\n{t}"
    );
}

#[test]
fn status_line_truncation_keeps_the_size_suffix() {
    // A name longer than the whole row ellipsizes, but ` · size` survives.
    let long = "an_extremely_long_filename_that_cannot_fit_even_a_full_row.txt";
    let base = std::env::temp_dir().join("crew_far_render_statustrunc");
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&base).unwrap();
    std::fs::write(base.join(long), b"x").unwrap();
    let mut p = FarPane::new(base);
    p.left.sel = 1;
    let cells = render(&p, 40, 24);
    let status_row = 21;
    let mut row: Vec<(u16, char)> = cells
        .iter()
        .filter(|c| c.row == status_row)
        .map(|c| (c.col, c.c))
        .collect();
    row.sort_unstable_by_key(|(col, _)| *col);
    let line: String = row.into_iter().map(|(_, c)| c).collect();
    // to_cells drops blank (space) cells, so assert the squished suffix.
    assert!(line.contains("\u{2026}\u{b7}1B"), "ellipsis+suffix missing: {line:?}");
}

#[test]
fn status_line_blank_for_an_empty_listing() {
    let base = std::env::temp_dir().join("crew_far_render_statusempty");
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&base).unwrap();
    // A real pane always lists `..`, so force an empty listing directly.
    let mut p = FarPane::new(base);
    p.left.entries.clear();
    p.left.sel = 0;
    let cells = render(&p, 40, 24);
    let status_row = 21;
    // `to_cells` drops blank cells, so "blank" means no cells on the row.
    assert!(
        cells.iter().all(|c| c.row != status_row),
        "status row must be blank on an empty listing"
    );
}
```

Also update `tiny_renders_nothing` (line 84–87) to cover the new minimum:

```rust
#[test]
fn tiny_renders_nothing() {
    assert!(render(&fixture_pane("tiny"), 8, 2).is_empty());
    // 5 rows can no longer hold panels + status + command + fkey rows.
    assert!(render(&fixture_pane("tiny5"), 40, 5).is_empty());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p crew-app farpane::render`
Expected: FAIL — `status_line_carries_the_selected_entrys_full_name`, `status_line_truncation_keeps_the_size_suffix`, `status_line_blank_for_an_empty_listing` (no cells on row 21 hold the name; row 21 is currently a panel row), and `tiny_renders_nothing` (5 rows currently renders). Compilation itself must succeed.

- [ ] **Step 3: Implement `status_bar` in `bars.rs`**

In `crates/crew-app/src/farpane/bars.rs`, after `selected_label` (line 93), add:

```rust
/// The mini-status row above the command line: the active panel's selected
/// entry in full (the listing truncates long names; this row is the readable
/// copy). Blank on an empty listing — the row is always present, so the
/// layout never jumps.
pub(super) fn status_bar(buf: &mut Buffer, area: Rect, label: Option<&str>) {
    let t = crew_theme::theme();
    let bg = Color::Rgb(t.page_bg.0, t.page_bg.1, t.page_bg.2);
    let ink = Color::Rgb(t.ink.0, t.ink.1, t.ink.2);
    let line = match label {
        Some(l) => Line::from(Span::styled(
            ellipsize_keeping_suffix(l, area.width as usize),
            Style::new().fg(ink).bg(bg),
        )),
        None => Line::default(),
    };
    Paragraph::new(line).style(Style::new().bg(bg)).render(area, buf);
}

/// Fit `label` into `width` columns: the ` · size` suffix stays intact and
/// the name ellipsizes (the same rule the listing rows use). A label without
/// the separator (a directory) ellipsizes plainly from the end.
fn ellipsize_keeping_suffix(label: &str, width: usize) -> String {
    if label.chars().count() <= width || width == 0 {
        return label.to_string();
    }
    let (name, suffix) = match label.rfind(" \u{b7} ") {
        Some(i) => label.split_at(i),
        None => (label, ""),
    };
    let keep = width.saturating_sub(suffix.chars().count() + 1);
    let head: String = name.chars().take(keep).collect();
    format!("{head}\u{2026}{suffix}")
}
```

- [ ] **Step 4: Wire the layout in `render.rs`**

In `crates/crew-app/src/farpane/render.rs`:

1. Guard (line 20): change `if cols < 16 || rows < 5 {` to `if cols < 16 || rows < 6 {`.
2. Layout (lines 25–31): replace with

```rust
    // Panels, then the status line (selected entry in full), then the
    // command line, then the function-key bar.
    let split = Layout::vertical([
        Constraint::Min(3),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .split(area);
```

3. After the `sel_label` computation (line 73), add the status call and shift the row indices:

```rust
    status_bar(&mut buf, split[1], sel_label.as_deref());
    command_bar(
        &mut buf,
        split[2],
        &p.active_panel_folder(),
        &p.cmdline,
        ghost.as_deref(),
        ask_hint.as_deref(),
        suggested,
        running,
        sel_label.as_deref(),
    );
    // The make-folder prompt takes over the function-key row while it's open.
    match &p.prompt {
        Some(prompt) => prompt_bar(&mut buf, split[3], prompt),
        None => function_bar(&mut buf, split[3]),
    }
```

(`command_bar` still takes `selected` in this task — Task 2 removes it.)

4. Import (line 353): change to `use bars::{command_bar, function_bar, prompt_bar, status_bar};`

- [ ] **Step 5: Run the farpane tests**

Run: `cargo test -p crew-app farpane`
Expected: ALL PASS — the three new tests, the updated `tiny_renders_nothing`, and every pre-existing test (`cmd_row = 22` tests are unaffected because the bottom three rows keep their absolute positions; the panels shrink instead).

- [ ] **Step 6: Commit**

```bash
cargo fmt
git add crates/crew-app/src/farpane/bars.rs crates/crew-app/src/farpane/render.rs crates/crew-app/src/farpane/render_tests.rs
git commit -m "feat(far): mini-status line shows the selected entry in full"
```

---

### Task 2: Remove the selected name from the command bar

**Files:**
- Modify: `crates/crew-app/src/farpane/bars.rs:21-93` (`command_bar`)
- Modify: `crates/crew-app/src/farpane/render.rs` (call site)
- Test: `crates/crew-app/src/farpane/render_tests.rs`

**Interfaces:**
- Consumes: Task 1's `status_bar` wiring — `render.rs` already renders the name on the status row.
- Produces: `command_bar` with signature `(buf, area, folder, cmdline, ghost, ask_hint, suggested, running)` — no `selected` parameter.

- [ ] **Step 1: Write the failing test**

Append to `crates/crew-app/src/farpane/render_tests.rs`:

```rust
#[test]
fn command_bar_no_longer_carries_the_selected_name() {
    // The status row (Task 1) is the single home of the selected name; the
    // command line keeps its full width for typing/ghost/running hints.
    let name = "unique_marker_filename.txt";
    let base = std::env::temp_dir().join("crew_far_render_cmdnosel");
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&base).unwrap();
    std::fs::write(base.join(name), b"x").unwrap();
    let mut p = FarPane::new(base);
    p.left.sel = 1;
    let cells = render(&p, 120, 24);
    let cmd_row = 22;
    let line: String = cells
        .iter()
        .filter(|c| c.row == cmd_row)
        .map(|c| c.c)
        .collect();
    assert!(
        !line.contains("unique_marker"),
        "selected name must not render on the command row: {line:?}"
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p crew-app command_bar_no_longer_carries`
Expected: FAIL — at 120 cols the name fits after the prompt, so the old right-aligned rendering paints it on row 22.

- [ ] **Step 3: Remove the parameter and rendering**

In `crates/crew-app/src/farpane/bars.rs`:
1. Delete the `selected: Option<&str>,` parameter (line 31).
2. Delete the whole right-aligned block (lines 65–78: the comment starting `// The selected entry's full name…` through its closing `}`).
3. The `#[allow(clippy::too_many_arguments)]` on line 21 now guards eight args; leave it (still >7) but fix its comment: `// one bar, eight independent knobs`.

In `crates/crew-app/src/farpane/render.rs`:
1. Remove the trailing `sel_label.as_deref(),` argument from the `command_bar(...)` call.
2. Update the comment above the `active`/`sel_label` computation (lines 66–67) from "the command bar carries the readable copy" to:

```rust
    // The active panel's selected entry, in full — listing rows truncate
    // long names, so the status row carries the readable copy.
```

- [ ] **Step 4: Run the farpane tests**

Run: `cargo test -p crew-app farpane`
Expected: ALL PASS, including the new test and Task 1's tests.

- [ ] **Step 5: Commit**

```bash
cargo fmt
git add crates/crew-app/src/farpane/bars.rs crates/crew-app/src/farpane/render.rs crates/crew-app/src/farpane/render_tests.rs
git commit -m "refactor(far): command bar drops the selected-name fallback"
```
