# Font Rotation (`/font random`) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `/font random` rotates the UI font every 10 minutes through installed verified-monospace families; a manual pick turns it off; the mode survives restart.

**Architecture:** A new `crew-app/src/fontrotate.rs` holds pure, unit-testable state (pool, pick, due-gating on `crew_theme::ROTATE_MS`). `poll.rs` drives it beside the theme rotation; `fontcmd.rs` gains the `random` argument; config gains `font_random: bool`. Rotation NEVER writes `config.font_family` — the rotated family lives only in `FontRotate::current`, so unrelated `config.save()` calls can't persist a rotated pick.

**Tech Stack:** Rust; existing `renderer.monospace_families()` (verified fixed-pitch scan) and `renderer.set_font_family()`; `crew_theme::ROTATE_MS` (600_000) as the shared rotation clock.

## Global Constraints

- Zero `cargo check` warnings; rustfmt clean (pre-commit hook enforces).
- Rotation must NEVER mutate `config.font_family` (spec: restart returns to the pinned family; resize-settle `config.save()` must not persist a rotated pick).
- Interval is `crew_theme::ROTATE_MS` — no new interval constant.
- The pick is deterministic from a seed (same hash recipe as `crew_theme::random_pick`: `seed.wrapping_mul(6364136223846793005).rotate_right(29) as usize % len`) — no rand dependency.
- A pool of ≤ 1 family: `/font random` reports "only one monospace font installed" and stays off.

---

### Task 1: `fontrotate.rs` — pure rotation state

**Files:**
- Create: `crates/crew-app/src/fontrotate.rs`
- Modify: `crates/crew-app/src/main.rs` (add `mod fontrotate;` alongside the existing mod list)

**Interfaces:**
- Produces (used by Tasks 2–3):
  - `pub(crate) struct FontRotate { pub on: bool, pub last_ms: u64, pub pool: Option<Vec<String>>, pub current: Option<String> }` with `Default`
  - `impl FontRotate { pub(crate) fn due(&self, now_ms: u64) -> bool }`
  - `pub(crate) fn pick(pool: &[String], current: Option<&str>, seed: u64) -> Option<String>`

- [ ] **Step 1: Write the failing tests**

Create `crates/crew-app/src/fontrotate.rs` with tests first (module body below in Step 3; put this at the bottom):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn pool() -> Vec<String> {
        vec!["Menlo".into(), "Monaco".into(), "Hack".into()]
    }

    #[test]
    fn pick_never_returns_the_current_family() {
        for seed in 0..50u64 {
            let p = pick(&pool(), Some("Menlo"), seed).unwrap();
            assert_ne!(p, "Menlo", "seed {seed}");
        }
    }

    #[test]
    fn pick_is_deterministic_for_a_seed() {
        assert_eq!(pick(&pool(), Some("Menlo"), 7), pick(&pool(), Some("Menlo"), 7));
    }

    #[test]
    fn pick_returns_none_when_no_alternative_exists() {
        assert_eq!(pick(&["Menlo".to_string()], Some("Menlo"), 1), None);
        assert_eq!(pick(&[], None, 1), None);
    }

    #[test]
    fn due_gates_on_the_shared_rotate_clock() {
        let mut r = FontRotate {
            on: true,
            last_ms: 1_000,
            ..Default::default()
        };
        assert!(!r.due(1_000 + crew_theme::ROTATE_MS - 1));
        assert!(r.due(1_000 + crew_theme::ROTATE_MS));
        r.on = false;
        assert!(!r.due(1_000 + crew_theme::ROTATE_MS));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p crew-app fontrotate`
Expected: compile FAIL — module contents missing (`FontRotate`, `pick` not found).

- [ ] **Step 3: Implement**

Top of `crates/crew-app/src/fontrotate.rs`:

```rust
//! `/font random`: rotate the UI font through the installed monospace
//! families on the shared 10-minute rotation clock (`crew_theme::ROTATE_MS`).
//! The rotated family lives HERE (`current`), never in `config.font_family`
//! — unrelated `config.save()` calls must not persist a rotated pick, and a
//! restart returns to the user's pinned family.

/// Rotation state on the app. `pool` is scanned once per session (loading
/// faces is not free) and cached; `None` = not scanned yet.
#[derive(Default)]
pub(crate) struct FontRotate {
    pub on: bool,
    pub last_ms: u64,
    pub pool: Option<Vec<String>>,
    pub current: Option<String>,
}

impl FontRotate {
    /// Whether a rotation is due at `now_ms` (only while on).
    pub(crate) fn due(&self, now_ms: u64) -> bool {
        self.on && now_ms.saturating_sub(self.last_ms) >= crew_theme::ROTATE_MS
    }
}

/// A family from `pool` that isn't `current`, deterministically from `seed`
/// (same hash recipe as `crew_theme::random_pick`). `None` when the pool has
/// no alternative.
pub(crate) fn pick(pool: &[String], current: Option<&str>, seed: u64) -> Option<String> {
    let others: Vec<&String> = pool.iter().filter(|f| Some(f.as_str()) != current).collect();
    if others.is_empty() {
        return None;
    }
    let idx = (seed.wrapping_mul(6364136223846793005).rotate_right(29) as usize) % others.len();
    Some(others[idx].clone())
}
```

In `crates/crew-app/src/main.rs`, add `mod fontrotate;` in alphabetical position within the existing `mod` list (after `mod fontcmd;`).

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p crew-app fontrotate`
Expected: PASS (4 tests).

- [ ] **Step 5: Commit**

```bash
git add crates/crew-app/src/fontrotate.rs crates/crew-app/src/main.rs
git commit -m "feat(crew): fontrotate — pure font-rotation state and pick"
```

---

### Task 2: Config flag `font_random`

**Files:**
- Modify: `crates/crew-app/src/config.rs` (struct field + `Default` + `clamped()`; follow the exact pattern of the existing `paper_texture` bool field)

**Interfaces:**
- Produces: `pub font_random: bool` on `CrewConfig` (serde `#[serde(default)]`, default `false`).
- Consumes: nothing.

- [ ] **Step 1: Write the failing test**

In `config.rs`'s existing test module:

```rust
    #[test]
    fn font_random_round_trips_and_defaults_off() {
        let cfg = CrewConfig::from_toml_str("");
        assert!(!cfg.font_random);
        let cfg = CrewConfig::from_toml_str("font_random = true\n");
        assert!(cfg.font_random);
        assert!(cfg.clamped().font_random, "clamped() must carry the flag");
    }
```

(If `from_toml_str` doesn't exist, use whatever parse helper the neighboring tests use — the file's tests at lines ~211-300 show the local convention.)

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p crew-app font_random`
Expected: compile FAIL — no field `font_random`.

- [ ] **Step 3: Implement**

Add to the `CrewConfig` struct (near `font_family`):

```rust
    /// `/font random`: rotate the UI font every 10 minutes through the
    /// installed monospace families. The rotated pick itself is NOT saved —
    /// `font_family` stays whatever the user pinned.
    #[serde(default)]
    pub font_random: bool,
```

Add `font_random: false,` to the `Default` impl and `font_random: self.font_random,` to `clamped()` (match how `paper_texture` threads through both).

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p crew-app font_random` → PASS. Then `cargo test -p crew-app config` → all config tests PASS (other tests construct `CrewConfig { .. }` literally and may need the new field — fix them with `font_random: false`).

- [ ] **Step 5: Commit**

```bash
git add crates/crew-app/src/config.rs
git commit -m "feat(crew): font_random config flag"
```

---

### Task 3: Wiring — command, poll tick, manual-pick override

**Files:**
- Modify: `crates/crew-app/src/app.rs` (add `pub(crate) font_rotate: crate::fontrotate::FontRotate` field to `CrewApp` + `Default`/constructor sites — find them with `grep -n "procnames" crates/crew-app/src/app.rs crates/crew-app/src/handler.rs`, it's a peer field)
- Modify: `crates/crew-app/src/fontcmd.rs` (the `random` argument + no-arg report)
- Modify: `crates/crew-app/src/poll.rs` (tick beside `tick_random`, ~line 28)
- Modify: `crates/crew-app/src/spawn.rs` (`apply_config` re-applies the rotated family; manual family change turns rotation off)

**Interfaces:**
- Consumes: `FontRotate`, `fontrotate::pick` (Task 1); `config.font_random` (Task 2).

- [ ] **Step 1: Write the failing tests**

In `fontcmd.rs`'s test module:

```rust
    #[test]
    fn font_random_arg_enables_rotation_or_reports_thin_pool() {
        let mut app = CrewApp::default();
        app.set_font_cmd("random");
        // Headless default app has no renderer → pool scan yields nothing →
        // rotation must stay off with the thin-pool report.
        assert!(!app.font_rotate.on);
        assert!(app.active_status().is_some());
    }

    #[test]
    fn no_arg_report_mentions_rotation_state() {
        let mut app = CrewApp::default();
        app.set_font_cmd("");
        let s = app.active_status().unwrap();
        assert!(s.contains("font size"), "{s}");
    }
```

In `spawn.rs`'s test module (or `app_tests.rs`, following where `apply_config` is already tested — check `grep -rn "apply_config" crates/crew-app/src/*tests*.rs`):

```rust
    #[test]
    fn manual_family_change_disables_rotation() {
        let mut app = CrewApp::default();
        app.font_rotate.on = true;
        let mut cfg = app.config.clone();
        cfg.font_family = Some("Menlo".to_string());
        app.apply_config(cfg);
        assert!(!app.font_rotate.on, "explicit family pick stops rotation");
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p crew-app font_random_arg; cargo test -p crew-app manual_family_change`
Expected: compile FAIL — no `font_rotate` field.

- [ ] **Step 3: Implement**

`app.rs`: add the field to `CrewApp`:

```rust
    /// `/font random` rotation state (pool cache + 10-minute clock).
    pub(crate) font_rotate: crate::fontrotate::FontRotate,
```

and `font_rotate: Default::default(),` at every `CrewApp { ... }` construction site (handler.rs `run()` and the test `Default`). On startup (handler.rs `run()`, right after `let config = CrewConfig::load();` block where the theme is applied), seed it:

```rust
    let font_rotate = crate::fontrotate::FontRotate {
        on: config.font_random,
        last_ms: crate::chattime::unix_now_ms(), // first rotation after ROTATE_MS, no launch swap
        ..Default::default()
    };
```

and pass `font_rotate` into the `CrewApp { ... }` literal.

`fontcmd.rs` — extend `set_font_cmd`:

```rust
        if arg.eq_ignore_ascii_case("random") {
            let pool = self.font_pool();
            let now = crate::chattime::unix_now_ms();
            let seed = now;
            let cur = self.current_family();
            match crate::fontrotate::pick(&pool, cur.as_deref(), seed) {
                Some(fam) => {
                    self.font_rotate.on = true;
                    self.font_rotate.last_ms = now;
                    self.apply_rotated_family(fam);
                    self.config.font_random = true;
                    self.config.save();
                }
                None => {
                    self.font_rotate.on = false;
                    self.set_status("font random: only one monospace font installed".to_string());
                }
            }
            return;
        }
```

and change the no-arg report to include rotation state:

```rust
        if arg.is_empty() {
            let rot = if self.font_rotate.on {
                match &self.font_rotate.current {
                    Some(f) => format!(" — rotating (now: {f})"),
                    None => " — rotating".to_string(),
                }
            } else {
                String::new()
            };
            self.set_status(format!(
                "font size {}{rot} — /font <n> to set, /font random to rotate",
                self.config.font_size as i32
            ));
            return;
        }
```

Add three small helpers on `CrewApp` (in `fontcmd.rs`, they're font-scoped):

```rust
    /// The cached monospace pool, scanning once on first use (loads faces).
    fn font_pool(&mut self) -> Vec<String> {
        if self.font_rotate.pool.is_none() {
            let pool = self
                .renderer
                .as_mut()
                .map(|r| r.monospace_families())
                .unwrap_or_default();
            self.font_rotate.pool = Some(pool);
        }
        self.font_rotate.pool.clone().unwrap_or_default()
    }

    /// The family rotation should avoid repeating: the rotated pick if one is
    /// live, else the pinned config family.
    fn current_family(&self) -> Option<String> {
        self.font_rotate.current.clone().or_else(|| self.config.font_family.clone())
    }

    /// Apply a rotated family to the renderer and status line — NEVER to config.
    fn apply_rotated_family(&mut self, fam: String) {
        if let Some(r) = &mut self.renderer {
            r.set_font_family(Some(fam.clone()));
        }
        self.set_status(format!("font → {fam}"));
        self.font_rotate.current = Some(fam);
        self.redraw();
    }
```

`poll.rs` — after the `let rotated = crew_theme::tick_random(...)` block (~line 31), add:

```rust
        // Font rotation: same 10-minute clock as the theme rotation. The pick
        // updates the renderer only — config.font_family stays pinned.
        let now_ms = crate::chattime::unix_now_ms();
        if self.font_rotate.due(now_ms) {
            let pool = self.font_pool();
            let cur = self.current_family();
            if let Some(fam) = crate::fontrotate::pick(&pool, cur.as_deref(), now_ms) {
                self.apply_rotated_family(fam);
            }
            self.font_rotate.last_ms = now_ms;
        }
```

(`apply_rotated_family` already calls `redraw()`; no `any_changed` change needed. Make the helpers `pub(crate)` if poll.rs can't see them.)

`spawn.rs` `apply_config` — after the `r.set_font_family(self.config.font_family.clone());` line, add:

```rust
        // A manual family pick in Settings stops rotation; otherwise a live
        // rotation keeps its current pick on top of the re-applied config.
        if self.config.font_family != old_family {
            self.font_rotate.on = false;
            self.font_rotate.current = None;
            self.config.font_random = false;
        } else if let (true, Some(fam)) = (self.font_rotate.on, self.font_rotate.current.clone()) {
            if let Some(r) = &mut self.renderer {
                r.set_font_family(Some(fam));
            }
        }
```

capturing `let old_family = self.config.font_family.clone();` at the top of `apply_config` before `self.config = cfg;`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p crew-app`
Expected: PASS including the three new tests and all existing suites.

- [ ] **Step 5: Full gate + commit**

Run: `cargo check -p crew-app 2>&1 | grep -c warning` → `0`; `cargo fmt --check` → clean.

```bash
git add crates/crew-app/src/app.rs crates/crew-app/src/fontcmd.rs crates/crew-app/src/poll.rs crates/crew-app/src/spawn.rs crates/crew-app/src/handler.rs
git commit -m "feat(crew): /font random — 10-minute font rotation over installed monospace families"
```

---

## Self-Review Notes

- Spec coverage: state+pick (T1), persistence (T2 + fontcmd save), tick/pool-cache/immediate-first-pick/manual-override/status-flash (T3), "never touches config.font_family" (constraint + apply_rotated_family), ≤1-pool report (T1 pick=None + T3 command arm), startup-no-swap (handler seed `last_ms = now`).
- Type consistency: `FontRotate{on,last_ms,pool,current}`, `pick(&[String], Option<&str>, u64) -> Option<String>` used identically in T3.
- The headless-test caveat (no renderer → empty pool) is what makes the command tests deterministic.
