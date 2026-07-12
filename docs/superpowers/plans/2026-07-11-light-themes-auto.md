# Light Themes, Random Split & Auto Day/Night Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Four new newspaper light themes; `random` splits into `random-dark`/`random-light` pools; `/theme auto` follows the OS appearance, rotating within the matching pool every 10 minutes.

**Architecture:** Presets land in a new `crew-theme/src/presets_paper_light.rs`. The `RANDOM: AtomicBool` becomes a `MODE: AtomicU8` (`Off|RandomDark|RandomLight|Auto`) plus an `OS_DARK: AtomicBool` fed by winit's `ThemeChanged`; `random_pick` gains a `dark: bool` pool parameter. A shared `parse_selection`/`Selection` in crew-theme replaces per-call-site string matching in the app (startup, `apply_config`, `set_theme_cmd`, chat `/theme` intercept, `Ctrl+Shift+L` cycle).

**Tech Stack:** Rust; crew-theme (no deps, atomics only); winit `Window::theme()` + `WindowEvent::ThemeChanged` in crew-app.

## Global Constraints

- Zero `cargo check` warnings; rustfmt clean (pre-commit hook enforces).
- All four new presets: `dark: false`, `grain: 1.2` (the post-gamma-blending calibration — NOT the historical 3.0), `border_thickness: 3.0`, `term_bg == page_bg`.
- Existing persisted values must keep working: u8 ids 0–8 unchanged (new ids append 9–12); the config string `random` stays valid as an alias for `random-dark`.
- Mode names are exactly: `random-dark`, `random-light`, `auto`.
- `ROTATE_MS` (600_000) is unchanged and shared by every mode.
- Every existing lib_tests invariant that iterates `ALL_THEMES` (contrast_thresholds, dark_flag_matches_page_bg_luminance, grain, no pure black/white, term_bg==page_bg) must pass for the new presets unmodified.

---

### Task 1: Four light presets + ThemeId plumbing

**Files:**
- Create: `crates/crew-theme/src/presets_paper_light.rs`
- Modify: `crates/crew-theme/src/lib.rs` (mod + re-export, `ThemeId` enum/`ALL_THEMES`/`as_str`/`describe`/`from_name`/`theme`/`as_u8`/`from_u8`)
- Modify: `crates/crew-theme/src/lib_tests.rs` (u8 round-trip asserts; cycle test list)

**Interfaces:**
- Produces: `ThemeId::{SepiaLight, SalmonBroadsheet, ColdpressGray, IvoryLedger}`; statics `SEPIA_LIGHT, SALMON_BROADSHEET, COLDPRESS_GRAY, IVORY_LEDGER`; `ALL_THEMES: [ThemeId; 13]` ordered: PaperDark, PaperLight, SepiaDark, SepiaLight, MidnightInk, Graphite, ColdpressGray, SalmonBroadsheet, IvoryLedger, CrtGreen, CrtAmber, CrtBlue, CrtViolet.
- Consumes: nothing.

- [ ] **Step 1: Write the failing tests**

In `lib_tests.rs`, extend `u8_mapping_round_trips_all_nine_ids` (rename to `u8_mapping_round_trips_all_ids`, update its comment to "original nine") by adding after the existing from_u8 asserts:

```rust
    assert_eq!(ThemeId::from_u8(9), ThemeId::SepiaLight);
    assert_eq!(ThemeId::from_u8(10), ThemeId::SalmonBroadsheet);
    assert_eq!(ThemeId::from_u8(11), ThemeId::ColdpressGray);
    assert_eq!(ThemeId::from_u8(12), ThemeId::IvoryLedger);
```

In `cycle_next_walks_all_themes_then_random_then_wraps`, replace the hardcoded want-list with the new 12-name sequence (paper-dark is the start, so it's not in the list):

```rust
    for want in [
        "paper-light",
        "sepia-dark",
        "sepia-light",
        "midnight-ink",
        "graphite",
        "coldpress-gray",
        "salmon-broadsheet",
        "ivory-ledger",
        "crt-green",
        "crt-amber",
        "crt-blue",
        "crt-violet",
    ] {
        assert_eq!(cycle_next(1), want);
    }
```

(The trailing "random" assertions in that test are rewritten in Task 2 — for THIS task, keep them compiling by leaving them as-is; they still pass because Task 1 does not change the mode machinery.)

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p crew-theme`
Expected: compile FAIL — no variant `SepiaLight`.

- [ ] **Step 3: Implement**

Create `crates/crew-theme/src/presets_paper_light.rs` with header `//! Light paper-family presets: four newspaper pages (see presets_paper.rs for the family conventions).`, `use crate::Theme;`, and these four statics EXACTLY (they are pre-verified against every lib_tests floor):

```rust
/// **Sepia light**: warm aged-newsprint cream page with deep brown-black
/// ink — the light twin of SEPIA_DARK, echoing its warm gold accent
/// character.
pub static SEPIA_LIGHT: Theme = Theme {
    page_bg: (245, 235, 205),
    ink: (20, 13, 8),
    text_muted: (59, 45, 29),
    term_fg: (20, 13, 8),
    term_bg: (245, 235, 205),
    border_normal: (186, 160, 118),
    border_focused: (120, 90, 50),
    border_thickness: 3.0,
    legend_off: (108, 84, 52),
    accent_default: (150, 90, 20),
    status_fg: (140, 90, 10),
    broadcast: (140, 50, 90),
    activity: (50, 80, 120),
    bell: (146, 93, 15),
    dim: (120, 100, 80),
    placeholder: (120, 101, 78),
    hint_fg: (121, 101, 77),
    find_hl_bg: (225, 195, 110),
    ansi: [
        (40, 28, 18),    // 0  black
        (162, 40, 26),   // 1  red (brick)
        (70, 95, 28),    // 2  green (sage)
        (142, 95, 14),   // 3  yellow (ochre)
        (45, 80, 120),   // 4  blue (faded indigo)
        (120, 48, 100),  // 5  magenta (mauve)
        (20, 100, 95),   // 6  cyan (teal)
        (80, 68, 50),    // 7  white (warm gray)
        (100, 85, 62),   // 8  bright black
        (185, 55, 35),   // 9  bright red
        (85, 115, 35),   // 10 bright green
        (140, 95, 17),   // 11 bright yellow
        (55, 95, 140),   // 12 bright blue
        (138, 60, 115),  // 13 bright magenta
        (28, 118, 112),  // 14 bright cyan
        (42, 30, 20),    // 15 bright white (boldest ink)
    ],
    dark: false,
    grain: 1.2,
};

/// **Salmon broadsheet**: Financial-Times-style salmon-pink page with cool
/// near-black ink; accents lean navy/teal.
pub static SALMON_BROADSHEET: Theme = Theme {
    page_bg: (250, 231, 215),
    ink: (12, 13, 18),
    text_muted: (46, 46, 53),
    term_fg: (12, 13, 18),
    term_bg: (250, 231, 215),
    border_normal: (178, 152, 130),
    border_focused: (45, 50, 75),
    border_thickness: 3.0,
    legend_off: (95, 86, 90),
    accent_default: (30, 55, 95),
    status_fg: (110, 82, 20),
    broadcast: (115, 40, 85),
    activity: (35, 70, 105),
    bell: (118, 86, 15),
    dim: (110, 95, 90),
    placeholder: (114, 101, 96),
    hint_fg: (116, 102, 96),
    find_hl_bg: (232, 205, 140),
    ansi: [
        (24, 22, 26),    // 0  black
        (150, 38, 40),   // 1  red
        (45, 90, 55),    // 2  green (forest)
        (130, 95, 20),   // 3  yellow (ochre)
        (30, 70, 115),   // 4  blue (navy)
        (100, 45, 95),   // 5  magenta (plum)
        (15, 95, 98),    // 6  cyan (teal)
        (60, 58, 64),    // 7  white (cool gray)
        (86, 80, 86),    // 8  bright black
        (172, 50, 48),   // 9  bright red
        (58, 108, 66),   // 10 bright green
        (133, 98, 22),   // 11 bright yellow
        (40, 88, 140),   // 12 bright blue
        (118, 55, 112),  // 13 bright magenta
        (20, 115, 118),  // 14 bright cyan
        (26, 24, 28),    // 15 bright white (boldest ink)
    ],
    dark: false,
    grain: 1.2,
};

/// **Coldpress gray**: cool pale-gray page with near-black neutral ink — the
/// light twin of GRAPHITE; the lowest-glare option.
pub static COLDPRESS_GRAY: Theme = Theme {
    page_bg: (238, 238, 240),
    ink: (18, 18, 19),
    text_muted: (49, 49, 52),
    term_fg: (18, 18, 19),
    term_bg: (238, 238, 240),
    border_normal: (172, 172, 176),
    border_focused: (96, 96, 100),
    border_thickness: 3.0,
    legend_off: (96, 96, 100),
    accent_default: (62, 64, 72),
    status_fg: (108, 80, 25),
    broadcast: (108, 45, 92),
    activity: (38, 68, 102),
    bell: (112, 84, 20),
    dim: (108, 108, 112),
    placeholder: (106, 106, 109),
    hint_fg: (106, 106, 109),
    find_hl_bg: (230, 222, 160),
    ansi: [
        (26, 26, 28),    // 0  black
        (148, 42, 40),   // 1  red
        (52, 92, 52),    // 2  green
        (128, 98, 20),   // 3  yellow (ochre)
        (34, 78, 120),   // 4  blue
        (104, 46, 98),   // 5  magenta
        (16, 98, 98),    // 6  cyan (teal)
        (66, 66, 70),    // 7  white (cool gray)
        (92, 92, 96),    // 8  bright black
        (172, 55, 50),   // 9  bright red
        (66, 112, 64),   // 10 bright green
        (134, 101, 23),  // 11 bright yellow
        (46, 94, 142),   // 12 bright blue
        (124, 58, 116),  // 13 bright magenta
        (22, 116, 116),  // 14 bright cyan
        (28, 28, 30),    // 15 bright white (boldest ink)
    ],
    dark: false,
    grain: 1.2,
};

/// **Ivory ledger**: slightly yellow ivory page with green-black ink — an
/// old accounting-ledger feel; accents lean deep green.
pub static IVORY_LEDGER: Theme = Theme {
    page_bg: (244, 239, 214),
    ink: (15, 19, 12),
    text_muted: (46, 51, 41),
    term_fg: (15, 19, 12),
    term_bg: (244, 239, 214),
    border_normal: (178, 172, 138),
    border_focused: (90, 96, 70),
    border_thickness: 3.0,
    legend_off: (92, 90, 70),
    accent_default: (30, 80, 40),
    status_fg: (112, 84, 20),
    broadcast: (108, 44, 90),
    activity: (36, 70, 104),
    bell: (116, 86, 18),
    dim: (108, 104, 84),
    placeholder: (110, 106, 88),
    hint_fg: (110, 107, 87),
    find_hl_bg: (228, 214, 130),
    ansi: [
        (24, 28, 18),    // 0  black
        (150, 40, 30),   // 1  red
        (35, 95, 42),    // 2  green (ledger green)
        (138, 100, 18),  // 3  yellow (ochre)
        (36, 74, 112),   // 4  blue
        (108, 46, 92),   // 5  magenta
        (16, 98, 88),    // 6  cyan (teal)
        (68, 68, 54),    // 7  white (warm-green gray)
        (92, 90, 70),    // 8  bright black
        (174, 52, 38),   // 9  bright red
        (44, 116, 52),   // 10 bright green
        (139, 99, 18),   // 11 bright yellow
        (46, 92, 138),   // 12 bright blue
        (126, 58, 110),  // 13 bright magenta
        (20, 114, 104),  // 14 bright cyan
        (18, 22, 14),    // 15 bright white (boldest ink)
    ],
    dark: false,
    grain: 1.2,
};
```

In `lib.rs`:
- After `mod presets_paper;` add `mod presets_paper_light;` and `pub use presets_paper_light::{COLDPRESS_GRAY, IVORY_LEDGER, SALMON_BROADSHEET, SEPIA_LIGHT};`
- `ThemeId`: add variants `SepiaLight, SalmonBroadsheet, ColdpressGray, IvoryLedger` (after `Graphite`).
- `ALL_THEMES: [ThemeId; 13]` in the Interfaces order above (paper family grouped light-beside-dark twin, then CRTs).
- `as_str`: `"sepia-light"`, `"salmon-broadsheet"`, `"coldpress-gray"`, `"ivory-ledger"`.
- `describe`: `"aged-newsprint cream page (light sepia)"`, `"FT salmon-pink broadsheet (light)"`, `"cool pale-gray page (light graphite)"`, `"ivory page, ledger-green ink (light)"`.
- `from_name`: the four new names.
- `theme()`: map to the four statics.
- `as_u8`/`from_u8`: `SepiaLight=9, SalmonBroadsheet=10, ColdpressGray=11, IvoryLedger=12` (existing 0–8 untouched).

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p crew-theme`
Expected: PASS — every ALL_THEMES-iterating invariant (contrast, dark-flag-luminance, grain 1.2, no-pure-black/white, term_bg) covers the new presets; u8 round-trip and cycle tests green. NOTE: `random_pick_only_returns_dark_themes` must still pass unchanged (pool filter is `is_dark()`, unaffected by adding light themes).

- [ ] **Step 5: Full gate + commit**

`cargo check -p crew-theme -p crew-app 2>&1 | grep -c "^warning"` → 0; `cargo fmt --check`.

```bash
git add crates/crew-theme/src/presets_paper_light.rs crates/crew-theme/src/lib.rs crates/crew-theme/src/lib_tests.rs
git commit -m "feat(theme): four light newspaper presets — sepia-light, salmon-broadsheet, coldpress-gray, ivory-ledger"
```

---

### Task 2: Mode machinery — random-dark / random-light / auto in crew-theme

**Files:**
- Modify: `crates/crew-theme/src/lib.rs` (replace the `RANDOM` bool block, `random_pick`, `set_random`, `tick_random`, `cycle_next`; add `RandomMode`, `Selection`, `parse_selection`, `set_os_dark`/`os_dark`)
- Modify: `crates/crew-theme/src/lib_tests.rs` (rewrite mode tests)

**Interfaces:**
- Produces (Task 3 consumes exactly these):
  - `pub enum RandomMode { Dark, Light, Auto }` (+ `Clone, Copy, Debug, PartialEq, Eq`) with `pub fn as_str(self) -> &'static str` returning `"random-dark" | "random-light" | "auto"`.
  - `pub enum Selection { Fixed(ThemeId), Mode(RandomMode) }` (+ same derives).
  - `pub fn parse_selection(s: &str) -> Option<Selection>` — trims; theme names via `ThemeId::from_name`; `random-dark`/`random-light`/`auto` (ASCII case-insensitive); `random` → `Mode(Dark)` alias.
  - `pub fn apply_selection(sel: Selection, now_ms: u64)` — Fixed: mode off + `set_theme`; Mode: store mode, immediate pick from its pool, reset clock.
  - `pub fn mode() -> Option<RandomMode>`; `pub fn is_random() -> bool` (any mode active — keeps existing callers/readers meaningful).
  - `pub fn selection_label() -> &'static str` — active mode's `as_str()` or `current_id().as_str()`.
  - `pub fn set_os_dark(dark: bool)`, `pub fn os_dark() -> bool` (default TRUE before any report).
  - `pub fn random_pick(current: ThemeId, seed: u64, dark: bool) -> ThemeId` (pool = `is_dark() == dark`, minus current).
  - `tick_random(now_ms) -> bool` unchanged signature; rotates within the active mode's pool (Auto → pool by `os_dark()`).
  - REMOVED: `set_random(bool, u64)` — all callers migrate in Task 3 (crew-theme compiles standalone after this task; crew-app compiles only after Task 3, so gate with `cargo check -p crew-theme` here, not the workspace).

- [ ] **Step 1: Write the failing tests**

In `lib_tests.rs`, DELETE `set_random_true_enables_mode_and_switches_theme_now`, `set_random_false_disables_mode`, and `random_pick_only_returns_dark_themes`; UPDATE `random_pick_never_returns_current_and_is_deterministic` to call `random_pick(current, seed, true)`; UPDATE `tick_random_fires_at_rotate_ms_when_on` replacing `RANDOM.store(true, ...)` with `MODE.store(1, Ordering::Relaxed)` (1 = Dark; keep `ROTATED_MS.store(0, ...)`) and `set_random(false, 0)` with `apply_selection(Selection::Fixed(ThemeId::PaperDark), 0)`. Then ADD:

```rust
#[test]
fn parse_selection_names_modes_and_alias() {
    assert_eq!(parse_selection("paper-light"), Some(Selection::Fixed(ThemeId::PaperLight)));
    assert_eq!(parse_selection(" random-dark "), Some(Selection::Mode(RandomMode::Dark)));
    assert_eq!(parse_selection("Random-Light"), Some(Selection::Mode(RandomMode::Light)));
    assert_eq!(parse_selection("AUTO"), Some(Selection::Mode(RandomMode::Auto)));
    assert_eq!(parse_selection("random"), Some(Selection::Mode(RandomMode::Dark)), "back-compat alias");
    assert_eq!(parse_selection("nope"), None);
}

#[test]
fn random_pick_pools_are_pure() {
    for current in ALL_THEMES {
        for seed in [0u64, 1, 42, 600_000, u64::MAX] {
            assert!(random_pick(current, seed, true).is_dark());
            assert!(!random_pick(current, seed, false).is_dark());
            assert_ne!(random_pick(current, seed, true), current);
            assert_ne!(random_pick(current, seed, false), current);
        }
    }
}

#[test]
fn apply_selection_modes_pick_from_their_pool_immediately() {
    let _g = guard();
    apply_selection(Selection::Mode(RandomMode::Light), 1_000);
    assert_eq!(mode(), Some(RandomMode::Light));
    assert!(is_random());
    assert!(!current_id().is_dark(), "light mode must land on a light theme");
    apply_selection(Selection::Mode(RandomMode::Dark), 2_000);
    assert!(current_id().is_dark());
    apply_selection(Selection::Fixed(ThemeId::PaperDark), 3_000);
    assert_eq!(mode(), None);
    assert!(!is_random());
    assert_eq!(current_id(), ThemeId::PaperDark);
}

#[test]
fn auto_mode_follows_the_os_appearance() {
    let _g = guard();
    set_os_dark(true);
    apply_selection(Selection::Mode(RandomMode::Auto), 1_000);
    assert!(current_id().is_dark(), "auto + OS dark → dark pool");
    // OS flips to light: the NEXT tick (or re-apply) must land light.
    set_os_dark(false);
    ROTATED_MS.store(0, Ordering::Relaxed);
    assert!(tick_random(ROTATE_MS));
    assert!(!current_id().is_dark(), "auto + OS light → light pool");
    set_os_dark(true);
    apply_selection(Selection::Fixed(ThemeId::PaperDark), 2_000);
}

#[test]
fn tick_random_rotates_within_the_light_pool() {
    let _g = guard();
    apply_selection(Selection::Mode(RandomMode::Light), 0);
    for i in 1..=4u64 {
        ROTATED_MS.store(0, Ordering::Relaxed);
        assert!(tick_random(i * ROTATE_MS));
        assert!(!current_id().is_dark(), "tick {i} left the light pool");
    }
    apply_selection(Selection::Fixed(ThemeId::PaperDark), 0);
}

#[test]
fn selection_label_names_mode_or_theme() {
    let _g = guard();
    apply_selection(Selection::Fixed(ThemeId::Graphite), 0);
    assert_eq!(selection_label(), "graphite");
    apply_selection(Selection::Mode(RandomMode::Auto), 0);
    assert_eq!(selection_label(), "auto");
    apply_selection(Selection::Fixed(ThemeId::PaperDark), 0);
}
```

Also rewrite the tail of `cycle_next_walks_all_themes_then_random_then_wraps`: after the 12 fixed names, expect `"random-dark"`, then `"random-light"`, then `"auto"`, then `"paper-dark"` (mode off, wrapped).

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p crew-theme`
Expected: compile FAIL — `parse_selection`, `RandomMode`, `MODE` not found.

- [ ] **Step 3: Implement in `lib.rs`**

Replace the block from `/// Random-rotation mode...` `static RANDOM` down through `cycle_next` with:

```rust
/// Rotation mode: when set, the active theme changes every [`ROTATE_MS`]
/// within the mode's pool. Stored as a lock-free u8 for per-frame reads:
/// 0 = off, 1 = dark pool, 2 = light pool, 3 = auto (pool follows the OS
/// appearance via [`set_os_dark`]).
static MODE: AtomicU8 = AtomicU8::new(0);
/// Wall-clock ms of the last rotation (or of enabling a mode).
static ROTATED_MS: AtomicU64 = AtomicU64::new(0);
/// The OS appearance, fed by winit's ThemeChanged. Defaults to dark so a
/// platform that never reports stays on the dark pool.
static OS_DARK: AtomicBool = AtomicBool::new(true);
/// How long each rotated theme is shown: 10 minutes (fonts share this).
pub const ROTATE_MS: u64 = 600_000;

/// The rotation pools: dark, light, or follow-the-OS.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RandomMode {
    Dark,
    Light,
    Auto,
}

impl RandomMode {
    pub fn as_str(self) -> &'static str {
        match self {
            RandomMode::Dark => "random-dark",
            RandomMode::Light => "random-light",
            RandomMode::Auto => "auto",
        }
    }

    fn as_u8(self) -> u8 {
        match self {
            RandomMode::Dark => 1,
            RandomMode::Light => 2,
            RandomMode::Auto => 3,
        }
    }

    fn from_u8(v: u8) -> Option<RandomMode> {
        match v {
            1 => Some(RandomMode::Dark),
            2 => Some(RandomMode::Light),
            3 => Some(RandomMode::Auto),
            _ => None,
        }
    }

    /// Which pool this mode draws from right now (auto asks the OS).
    fn pool_is_dark(self) -> bool {
        match self {
            RandomMode::Dark => true,
            RandomMode::Light => false,
            RandomMode::Auto => os_dark(),
        }
    }
}

/// What a theme name string resolves to: a fixed theme or a rotation mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Selection {
    Fixed(ThemeId),
    Mode(RandomMode),
}

/// Parse a `/theme` argument / config value. `random` is a back-compat
/// alias for `random-dark` (the pre-split behaviour).
pub fn parse_selection(s: &str) -> Option<Selection> {
    let s = s.trim();
    if s.eq_ignore_ascii_case("random") || s.eq_ignore_ascii_case("random-dark") {
        return Some(Selection::Mode(RandomMode::Dark));
    }
    if s.eq_ignore_ascii_case("random-light") {
        return Some(Selection::Mode(RandomMode::Light));
    }
    if s.eq_ignore_ascii_case("auto") {
        return Some(Selection::Mode(RandomMode::Auto));
    }
    ThemeId::from_name(s).map(Selection::Fixed)
}

/// The active rotation mode, if any.
pub fn mode() -> Option<RandomMode> {
    RandomMode::from_u8(MODE.load(Ordering::Relaxed))
}

/// Whether any rotation mode is active.
pub fn is_random() -> bool {
    mode().is_some()
}

/// Report the OS appearance (winit ThemeChanged / startup Window::theme).
/// While `auto` is active the change takes effect on the next rotation tick;
/// callers that want an immediate flip re-apply the selection (the app does).
pub fn set_os_dark(dark: bool) {
    OS_DARK.store(dark, Ordering::Relaxed);
}

/// The last reported OS appearance (defaults to dark).
pub fn os_dark() -> bool {
    OS_DARK.load(Ordering::Relaxed)
}

/// Pick a theme from the dark (`dark = true`) or light pool that is NOT
/// `current`, deterministically from `seed`. Both pools have ≥ 5 entries,
/// so minus `current` they are never empty.
pub fn random_pick(current: ThemeId, seed: u64, dark: bool) -> ThemeId {
    let others: Vec<ThemeId> = ALL_THEMES
        .iter()
        .copied()
        .filter(|&t| t.is_dark() == dark && t != current)
        .collect();
    let idx = (seed.wrapping_mul(6364136223846793005).rotate_right(29) as usize) % others.len();
    others[idx]
}

/// Apply a parsed selection: a fixed theme pins it (mode off); a mode
/// switches immediately to a pick from its pool (so the effect is visible)
/// and starts the 10-minute clock.
pub fn apply_selection(sel: Selection, now_ms: u64) {
    match sel {
        Selection::Fixed(id) => {
            MODE.store(0, Ordering::Relaxed);
            set_theme(id);
        }
        Selection::Mode(m) => {
            MODE.store(m.as_u8(), Ordering::Relaxed);
            set_theme(random_pick(current_id(), now_ms, m.pool_is_dark()));
            ROTATED_MS.store(now_ms, Ordering::Relaxed);
        }
    }
}

/// The status-line label for the active selection: the mode's name while
/// rotating, else the pinned theme's name.
pub fn selection_label() -> &'static str {
    match mode() {
        Some(m) => m.as_str(),
        None => current_id().as_str(),
    }
}

/// Called each poll tick with the current wall-clock ms. When a mode is on
/// and `ROTATE_MS` has elapsed, switch to a new pick from the mode's pool
/// (auto re-reads the OS appearance every tick) and return `true` so the
/// caller repaints. Cheap and lock-free — safe at ~62 Hz on the winit thread.
pub fn tick_random(now_ms: u64) -> bool {
    let Some(m) = mode() else {
        return false;
    };
    let last = ROTATED_MS.load(Ordering::Relaxed);
    if now_ms.saturating_sub(last) < ROTATE_MS {
        return false;
    }
    set_theme(random_pick(current_id(), now_ms, m.pool_is_dark()));
    ROTATED_MS.store(now_ms, Ordering::Relaxed);
    true
}

/// Advance the Ctrl+Shift+L cycle one step: the 13 fixed themes in
/// [`ALL_THEMES`] order, then random-dark → random-light → auto, wrapping
/// back to the first fixed theme (mode off). Returns the status-line label.
pub fn cycle_next(now_ms: u64) -> &'static str {
    match mode() {
        Some(RandomMode::Dark) => {
            apply_selection(Selection::Mode(RandomMode::Light), now_ms);
            "random-light"
        }
        Some(RandomMode::Light) => {
            apply_selection(Selection::Mode(RandomMode::Auto), now_ms);
            "auto"
        }
        Some(RandomMode::Auto) => {
            apply_selection(Selection::Fixed(ALL_THEMES[0]), now_ms);
            ALL_THEMES[0].as_str()
        }
        None => {
            let cur = current_id();
            let i = ALL_THEMES.iter().position(|&t| t == cur).unwrap_or(0);
            if i + 1 < ALL_THEMES.len() {
                set_theme(ALL_THEMES[i + 1]);
                ALL_THEMES[i + 1].as_str()
            } else {
                apply_selection(Selection::Mode(RandomMode::Dark), now_ms);
                "random-dark"
            }
        }
    }
}
```

(Keep `static CURRENT`, `set_theme`, `current_id`, `theme` as they are. `AtomicU8` is already imported.)

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p crew-theme`
Expected: PASS. (`cargo check -p crew-app` FAILS here — `set_random` is gone; that migration is Task 3, committed together with this crate compiling standalone. Do NOT run the workspace gate in this task.)

- [ ] **Step 5: Commit**

```bash
git add crates/crew-theme/src/lib.rs crates/crew-theme/src/lib_tests.rs
git commit -m "feat(theme): rotation modes — random-dark / random-light / auto (OS appearance)"
```

---

### Task 3: App wiring — parse everywhere, OS appearance feed, persistence

**Files:**
- Modify: `crates/crew-app/src/handler.rs` (startup selection ~line 85; `resumed` reads `window.theme()`)
- Modify: `crates/crew-app/src/spawn.rs` (`apply_config` ~line 261; `set_theme_cmd` ~line 300)
- Modify: `crates/crew-app/src/chattheme.rs` (parser + list line + intercept)
- Modify: `crates/crew-app/src/toggles.rs` (~line 57 `set_random(false, 0)` call)
- Modify: `crates/crew-app/src/events.rs` OR wherever `WindowEvent` is matched (`grep -n "WindowEvent::" crates/crew-app/src/*.rs` — add a `ThemeChanged` arm in the same match that handles `Resized`)
- Test: `crates/crew-app/src/chattheme.rs` tests + `crates/crew-app/src/app_tests.rs`

**Interfaces:**
- Consumes: `crew_theme::{parse_selection, apply_selection, Selection, RandomMode, mode, is_random, selection_label, set_os_dark}` exactly as produced by Task 2.
- Produces: config `theme` string may now be any of the 13 names, `random`, `random-dark`, `random-light`, `auto`.

- [ ] **Step 1: Write the failing tests**

In `chattheme.rs` tests, replace `parse_random_is_case_insensitive` with:

```rust
    #[test]
    fn parse_modes_and_alias() {
        assert_eq!(parse_theme_cmd("random"), ThemeCmd::Select(crew_theme::Selection::Mode(crew_theme::RandomMode::Dark)));
        assert_eq!(parse_theme_cmd("random-light"), ThemeCmd::Select(crew_theme::Selection::Mode(crew_theme::RandomMode::Light)));
        assert_eq!(parse_theme_cmd(" AUTO "), ThemeCmd::Select(crew_theme::Selection::Mode(crew_theme::RandomMode::Auto)));
    }
```

and update `parse_known_name_switches` to expect `ThemeCmd::Select(Selection::Fixed(...))`. Update `theme_list_line` tests: the list must contain `random-dark (rotates dark themes every 10 min)`, `random-light (…light…)`, and `auto (light by day, dark by night — follows the OS)`; when a mode is on, that mode's entry is marked and no fixed theme is marked.

In `app_tests.rs` add:

```rust
#[test]
fn apply_config_resumes_saved_mode_and_pins_fixed_themes() {
    let _g = crate::app::theme_test_guard();
    let mut app = CrewApp::default();
    let mut cfg = app.config.clone();
    cfg.theme = Some("random-light".to_string());
    app.apply_config(cfg);
    assert_eq!(crew_theme::mode(), Some(crew_theme::RandomMode::Light));
    assert!(!crew_theme::current_id().is_dark());
    let mut cfg = app.config.clone();
    cfg.theme = Some("graphite".to_string());
    app.apply_config(cfg);
    assert_eq!(crew_theme::mode(), None);
    assert_eq!(crew_theme::current_id(), crew_theme::ThemeId::Graphite);
    crew_theme::apply_selection(crew_theme::Selection::Fixed(crew_theme::ThemeId::PaperDark), 0);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p crew-app chattheme; cargo test -p crew-app apply_config_resumes`
Expected: compile FAIL (crew-app doesn't build until the `set_random` call sites migrate — that's the point).

- [ ] **Step 3: Implement**

`chattheme.rs`: replace `ThemeCmd::{Random, Switch}` with one variant `Select(crew_theme::Selection)`; `parse_theme_cmd` delegates:

```rust
fn parse_theme_cmd(arg: &str) -> ThemeCmd {
    let arg = arg.trim();
    if arg.is_empty() {
        return ThemeCmd::List;
    }
    match crew_theme::parse_selection(arg) {
        Some(sel) => ThemeCmd::Select(sel),
        None => ThemeCmd::Unknown(arg.to_string()),
    }
}
```

`theme_list_line(current, mode: Option<crew_theme::RandomMode>)`: fixed themes marked only when `mode.is_none()`; then three mode entries:

```rust
    let modes: [(crew_theme::RandomMode, &str); 3] = [
        (crew_theme::RandomMode::Dark, "rotates dark themes every 10 min"),
        (crew_theme::RandomMode::Light, "rotates light themes every 10 min"),
        (crew_theme::RandomMode::Auto, "light by day, dark by night — follows the OS"),
    ];
    for (m, desc) in modes {
        let mark = if mode == Some(m) { "\u{25cf} " } else { "" };
        items.push(format!("{mark}{} ({desc})", m.as_str()));
    }
```

`intercept`'s Select arm applies + echoes with `selection_label()`; the pane-local `/theme` intercept must ALSO persist now (it previously didn't save — that inconsistency bit users when rotation resumed on restart). It has no `&mut CrewApp`, so keep persistence at the app layer: the intercept only applies + echoes, unchanged in responsibility. `theme_names()` appends the three mode names + keeps `random`.

`spawn.rs::set_theme_cmd` becomes:

```rust
    pub(crate) fn set_theme_cmd(&mut self, arg: &str) {
        let arg = arg.trim();
        if arg.is_empty() {
            self.set_status(format!("theme: {}", crew_theme::selection_label()));
            return;
        }
        let Some(sel) = crew_theme::parse_selection(arg) else {
            let names = crew_theme::ALL_THEMES
                .iter()
                .map(|t| t.as_str())
                .chain(["random-dark", "random-light", "auto"])
                .collect::<Vec<_>>()
                .join(" | ");
            self.set_status(format!("unknown theme '{arg}' ({names})"));
            return;
        };
        crew_theme::apply_selection(sel, crate::chattime::unix_now_ms());
        self.config.theme = Some(
            match sel {
                crew_theme::Selection::Fixed(id) => id.as_str(),
                crew_theme::Selection::Mode(m) => m.as_str(),
            }
            .to_string(),
        );
        crate::palette::set_accent(self.config.accent_rgb());
        self.config.save();
        self.redraw();
        self.set_status(format!("theme: {}", crew_theme::selection_label()));
    }
```

`spawn.rs::apply_config` random-reconcile block becomes:

```rust
        match self
            .config
            .theme
            .as_deref()
            .and_then(crew_theme::parse_selection)
        {
            Some(sel) => crew_theme::apply_selection(sel, crate::chattime::unix_now_ms()),
            None => crew_theme::apply_selection(
                crew_theme::Selection::Fixed(self.config.theme_id()),
                crate::chattime::unix_now_ms(),
            ),
        }
```

`handler.rs` startup (`run()`): same `parse_selection`-based block replacing the `== Some("random")` special case. In `resumed()`, right after the window is created:

```rust
        // Seed the OS appearance for `/theme auto` (ThemeChanged keeps it live).
        if let Some(t) = window.theme() {
            crew_theme::set_os_dark(t == winit::window::Theme::Dark);
            if crew_theme::mode() == Some(crew_theme::RandomMode::Auto) {
                crew_theme::apply_selection(
                    crew_theme::Selection::Mode(crew_theme::RandomMode::Auto),
                    crate::chattime::unix_now_ms(),
                );
            }
        }
```

WindowEvent match (found via grep): add

```rust
            WindowEvent::ThemeChanged(t) => {
                crew_theme::set_os_dark(t == winit::window::Theme::Dark);
                // An appearance flip lands immediately in auto mode.
                if crew_theme::mode() == Some(crew_theme::RandomMode::Auto) {
                    crew_theme::apply_selection(
                        crew_theme::Selection::Mode(crew_theme::RandomMode::Auto),
                        crate::chattime::unix_now_ms(),
                    );
                    crate::palette::set_accent(self.config.accent_rgb());
                    self.redraw();
                }
            }
```

`toggles.rs:57`: replace `crew_theme::set_random(false, 0);` with `crew_theme::apply_selection(crew_theme::Selection::Fixed(crew_theme::current_id()), 0);` (same intent: pin the current theme, mode off).

`spawn.rs:263-324` other `set_random` sites and `chattheme.rs:84-88`: all replaced by the Select arm / apply_config block above — after this task `grep -rn "set_random" crates/crew-app/src` must return ONLY test guards (if any); fix every hit.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p crew-app; cargo test -p crew-theme`
Expected: all PASS.

- [ ] **Step 5: Full gate + commit**

`cargo check --workspace 2>&1 | grep -c "^warning"` → 0; `cargo fmt --check`; `cargo test --workspace` green. Also update the `/theme` line in `docs/CREW.md` if it enumerates themes/modes (`grep -n "random" docs/CREW.md`).

```bash
git add crates/crew-app crates/crew-theme docs/CREW.md
git commit -m "feat(crew): /theme random-dark|random-light|auto — pools + OS-appearance auto mode"
```

---

## Self-Review Notes

- Spec coverage: presets (T1), pools/split/alias (T2), auto + ThemeChanged + startup seed (T2/T3), persistence via config theme string (T3), cycle order (T2), list line (T3), u8 stability (T1).
- Type consistency: `parse_selection`/`apply_selection`/`Selection`/`RandomMode` names identical across T2 (producer) and T3 (consumers); `random_pick(current, seed, dark)` 3-arg form used in T2 tests.
- The chat-pane `/theme` intercept still doesn't persist (documented behavior — persistence belongs to the app-level command which owns config); the list/echo strings match between the two paths.
- Palettes verified against every lib_tests floor before planning (WCAG script, all pass; repo floors are strictly weaker).
