# Crew — Design Doc

**Date:** 2026-06-20
**Status:** Approved for planning
**Supersedes:** Farx (the TUI-inside-a-terminal era)

---

## 1. Summary

**Crew** is a native, GPU-accelerated terminal emulator built from scratch in Rust,
whose defining feature is an **orchestrator** at the bottom of the window. You talk
to one coordinating AI in that bottom box; it spawns and drives a grid of agent
panes (your "crew"), delegates tasks to them, and aggregates their results. Agents
coordinate *through* the orchestrator.

Crew is the successor to Farx. Farx was a TUI that ran *inside* someone else's
terminal, which capped it at the keys the host terminal chose to forward — the
recurring "keys limit" pain. Crew owns the OS keyboard layer because it *is* the
terminal. The keyboard-namespace collision that defined Farx disappears by
construction.

---

## 2. Why this pivot (the one-paragraph rationale)

Ghostty and Warp "just split panes and own every key" because they are terminal
emulators sitting at the OS layer. Farx, a ratatui TUI hosted inside another
terminal, could never get there: a host terminal eats `Cmd`, swallows many
`Ctrl`/`Alt` combos, and forwards only a subset of keys — the exact keys agents
also need. Crew's true peers were never Ghostty/Warp from the outside; they were
the terminals themselves. We are becoming one. We build from scratch (rather than
forking WezTerm) so the bottom-box UI, the keyboard/focus model, the agent grid,
and image rendering are all ours from line one — no fork-rebase tax, no fighting
another project's GUI model, and Farx's existing Rust code (grid tiling, AI
provider layer, PTY usage) ports in.

---

## 3. Goals & non-goals

### Goals
- A fast, native, GPU-rendered terminal emulator (Metal/Vulkan/DX12/GL via wgpu).
- Own the full keyboard — no reserved-key scarcity; a clean, self-designed focus
  model between the orchestrator box and agent panes.
- An **orchestrator chat** as the bottom surface: spawns/drives agent panes,
  delegates, aggregates.
- An **auto-tiling agent grid** (panes pack near-square; LRU demotion past a cap).
- **Inline images** as a first-class primitive (Kitty graphics protocol + Sixel
  fallback), designed frame-capable so video is a later flag, not a rewrite.
- Cross-platform (macOS, Linux; Windows via ConPTY where feasible).

### Non-goals (v1)
- **Inline video.** Deferred — gimmick economics today (6× CPU over a PTY, no audio
  story, FFmpeg C-dependency). The image path is built frame-capable so video is a
  bounded later addition.
- **Native API sub-agents** (orchestrator running agents directly via LLM tool-use,
  rendered as transcripts). North-star (hybrid), but phase 2.
- **Full hybrid orchestration** (CLI + native API agents coordinating). Phase 2+.
- **Remote/SSH TUI mode.** Going native means Crew is a local GPU app. Farx's
  "runs over SSH" property is intentionally retired. (A remote-mux story could be
  revisited far later; out of scope now.)
- A plugin system, theming engine, or config language. Later.

---

## 4. Product decisions (locked during brainstorming)

| Decision | Choice | Notes |
|---|---|---|
| Name | **Crew** | You direct a crew of agents. |
| Substrate | **From scratch, Rust** | Not a WezTerm fork; not another language. |
| Bottom box role | **Orchestrator chat** | One coordinating AI drives the panes. |
| Sub-agent model | **Hybrid (north star)** | v1 = real CLI agents in panes; native API agents later. |
| v1 scope | **Orchestrator drives N panes** | Multiple real CLI-agent panes, parallel delegate + aggregate. |
| Video | **Deferred** | Image path built frame-capable. |
| Language | **Rust** | The entire reusable terminal ecosystem is Rust. |

---

## 5. Architecture

```
┌───────────────────────────── Crew (one native GPU window) ─────────────────────────────┐
│                                                                                          │
│  ┌── Agent pane grid (cols = ceil(√n); cap full tiles, LRU-demote the rest) ──────────┐ │
│  │   pane 0            pane 1            pane 2                                          │ │
│  │   [Term model]      [Term model]      [Term model]   each pane =                     │ │
│  │   ↕ portable-pty    ↕ portable-pty    ↕ portable-pty  a real CLI-agent child process │ │
│  └──────────────────────────────────────────────────────────────────────────────────┘ │
│                                                                                          │
│  ┌── Orchestrator box (bottom input + transcript) ────────────────────────────────────┐ │
│  │  > user prompt …   crew-core: decompose → spawn panes → delegate → aggregate         │ │
│  └──────────────────────────────────────────────────────────────────────────────────┘ │
│                                                                                          │
│  Renderer: winit (window/input/IME) → wgpu (surface)                                     │
│            → glyphon + cosmic-text (text)  +  own quads (cursor/bg)                       │
│            → crew-image (Kitty/Sixel → wgpu textures)                                     │
└──────────────────────────────────────────────────────────────────────────────────────────┘
```

### Module / crate boundaries

Each of the four known risk points lives behind exactly one module boundary, so a
breaking upstream change touches one file, not the app.

- **`crew-render`** *(new)* — owns the window, the wgpu surface, the frame loop, the
  grid layout geometry, and the bottom-box surface. Wraps `winit` + `wgpu` +
  `glyphon` + `cosmic-text`. Draws text via glyphon and cursor/background/selection
  as its own quads.
- **`crew-term`** *(thin wrapper; ports Farx PTY usage)* — one instance per pane.
  Adapter around `alacritty_terminal::Term` (grid/scrollback/damage) fed by a
  `portable-pty` child. Exposes a stable internal `TermModel` interface
  (`renderable_content()`, `damage()`, `write_input()`), insulating the renderer
  from `alacritty_terminal`'s unstable API.
- **`crew-core`** *(new; reuses Farx's `farx-ai` provider client)* — the
  orchestrator brain. Task decomposition, agent lifecycle, delegate/aggregate
  policy, and result-gathering. Knows nothing about wgpu; testable headless.
- **`crew-image`** *(new)* — `image`-crate decode → wgpu texture atlas; Kitty
  graphics protocol parser + Sixel fallback (`icy_sixel` to emit, `sixel-image` to
  relay child output). Built frame-capable (cheap per-frame texture swap) for future
  video.

---

## 6. The crate stack

All permissively licensed (MIT/Apache/Zlib) — no AGPL exposure (a benefit of *not*
forking Warp/WezTerm-AGPL paths). Versions current as of June 2026.

| Layer | Crate | Version | License | Role |
|---|---|---|---|---|
| Window + input + IME | `winit` | 0.30.x | Apache-2.0 | Cross-platform window/event/IME |
| GPU surface | `wgpu` | 29.x | MIT/Apache | Metal/Vulkan/DX12/GL abstraction |
| Text shaping + fallback | `cosmic-text` | 0.19.x | MIT/Apache | Shaping, bidi, emoji/CJK fallback |
| Glyph atlas → GPU | `glyphon` | 0.11.x | MIT/Apache/Zlib | Atlas + GPU upload into our pass |
| CPU fallback | `softbuffer` | 0.4.x | MIT/Apache | No-GPU rendering path |
| VT engine | `alacritty_terminal` | 0.26.x | Apache-2.0 | Grid/scrollback/damage model |
| PTY | `portable-pty` | 0.9.x | MIT | Spawn agents; Windows ConPTY |
| Image decode | `image` | 0.25.x | MIT/Apache | PNG/JPEG/GIF/WebP + frames |
| Image protocol (emit) | `icy_sixel` | 0.5.x | MIT/Apache | Pure-Rust Sixel encode |
| Image protocol (relay) | `sixel-image` | 0.2.x | MIT | Parse/relay child Sixel |

**Proven-path note:** System76's COSMIC Terminal ships `glyphon` + `wgpu` over an
alacritty-style engine in production — the front-end half of this stack is a paved
road, not trail-breaking.

**References to study (not dependencies):** Rio's `sugarloaf` (custom wgpu renderer
patterns), COSMIC Terminal (the exact stack in a real terminal), Ghostty (Kitty
graphics + keyboard protocol conformance, clean core/UI split).

---

## 7. Component design

### 7.1 `crew-render`
- **Frame loop:** winit `ApplicationHandler` (0.30 trait API) holds app state; on
  redraw, compute grid geometry, ask each visible `crew-term` for
  `renderable_content()` + `damage()`, feed glyph runs to glyphon, draw cursor/bg
  quads, composite `crew-image` textures at correct z-order, present.
- **Grid geometry:** ported from Farx — `cols = ceil(sqrt(n))`; cap full tiles
  (default 6), demote least-recently-active panes to a minimized strip (LRU).
- **Bottom box:** a dedicated surface region below the grid (the grid area shrinks
  to make room). It is a self-drawn input widget — Crew owns text editing here
  (cursor, history, paste, selection), since no Rust GPU stack ships a text-input
  widget. Kept deliberately small in v1 (single-line growing to a few lines).

### 7.2 `crew-term`
- One `Term` + one `portable-pty` child per pane. PTY reader runs on a thread that
  forwards bytes into the app via a channel (standard WezTerm/Zellij pattern, since
  `portable-pty` is blocking).
- Exposes `TermModel`: `feed(&[u8])`, `renderable_content()`, `damage()`,
  `write_input(&[u8])`, `resize(cols, rows)`. This is the **adapter seam** that
  pins `alacritty_terminal` behind a stable internal interface.

### 7.3 `crew-core` (orchestrator)
- **Input:** the user's prompt from the bottom box.
- **Decompose:** call the LLM (via ported `farx-ai` provider client) to produce a
  plan — a set of sub-tasks, each with the agent/command to run.
- **Spawn:** for each sub-task, ask the app to open a pane running the chosen CLI
  agent (`claude-code`, `codex`, `gemini-cli`, or a shell), via `crew-term`.
- **Delegate:** write the task into each pane's PTY.
- **Aggregate:** gather each agent's result, summarize back in the transcript.
- **Headless-testable:** `crew-core` depends on a `PaneController` trait
  (spawn/send/read), not on the renderer — so orchestration logic is unit-tested
  with a fake controller.

#### Result-gathering (the fragile part — explicit design)
Reading a CLI agent's "answer" by scraping its TUI is brittle. Crew does **not**
screen-scrape free-form. Instead the orchestrator drives sub-agents with a
**structured boundary convention**:
- Preferred: instruct the sub-agent to **write its final result to a known file**
  (`$CREW_RESULT_FILE`); the orchestrator reads the file on completion.
- Fallback: instruct the sub-agent to wrap its final answer in **sentinel markers**
  (`<<<CREW_RESULT … CREW_END>>>`); the orchestrator keys off the markers in the
  pane's output stream, not arbitrary parsing.
- Phase 2 (native API agents) gets structured results for free and is the long-term
  answer; the convention above is the v1 bridge for CLI agents.

### 7.4 `crew-image`
- Decode via `image` → upload to a reused wgpu texture/atlas (`queue.write_texture`,
  `bytes_per_row = 4*width`, no 256-byte alignment needed for `write_texture`).
- **Never allocate per frame** — create textures once, overwrite (documented wgpu
  stall otherwise). Frame-capable: a texture can be re-driven from a frame source,
  which is the entire seam future video would plug into.
- Implement **Kitty graphics protocol first** (truecolor, placement, z-order,
  animation, tmux-safe placeholders; use shared-memory/temp-file transmission to
  dodge base64-over-PTY cost), **Sixel as compatibility fallback**.

---

## 8. Keyboard & focus model

Owning the keyboard removes scarcity but *transfers* the routing decision to us
(this is the honest cost of going native — the keys problem becomes ours to design,
not to suffer). Design:

- **One focus owner at a time:** either the orchestrator box or a specific agent
  pane holds keyboard focus.
- **Focus switching:** a mouse click on a pane/box focuses it (reliably, including
  on non-content regions — a retained Farx guardrail). A single reserved chord
  (e.g. a `Cmd`/`Super`-based key, which we now *can* use because we own the OS
  layer) jumps to/from the orchestrator box. `Esc` from the box returns focus to the
  last pane.
- **Everything else passes through** to the focused pane's agent untouched — the
  Farx principle that agents keep their keys, now without the host-terminal
  collision.
- Exact bindings are a planning detail; the *model* (single focus owner + one
  reserved jump + click-to-focus + passthrough) is fixed here.

---

## 9. Data flow

**Keystroke:** winit event → app routes by focus owner → (box) edit/submit prompt to
`crew-core`, or (pane) `crew-term.write_input()` → PTY → agent.

**Agent output:** PTY reader thread → channel → `crew-term.feed()` → `Term` updates →
next frame renders the pane's `renderable_content()`/`damage()`.

**Orchestration:** box submit → `crew-core` decompose (LLM) → spawn N panes →
delegate tasks → agents run (visible in their panes) → results gathered via the
boundary convention → orchestrator summarizes in the transcript.

---

## 10. Known risks & mitigations

| Risk | Mitigation |
|---|---|
| `alacritty_terminal` has **no API-stability guarantee** | Pin exact version; isolate behind `crew-term`'s `TermModel`; review CHANGELOG on bumps. |
| `wgpu` ↔ `glyphon` **breaking majors ~quarterly, in lockstep** | Upgrade both together; budget a periodic migration; confine to `crew-render`. |
| **Font fallback** rough edges (blank COLRv1 emoji, CJK overlap, Nerd-Font collisions) | Ship a **patched Nerd Font as primary** so icons never hit fallback; own the emoji/CJK fallback ordering behind a font module. |
| **IME** fragile on Linux/X11 + fcitx5/ibus | Isolate input handling; test IME per-platform; accept macOS/Wayland-first, X11 hardening as follow-up. |
| **Result-gathering** from CLI agents is brittle | Structured boundary convention (file or sentinels); native API agents in phase 2. |
| Building a **text-input widget** from scratch (no Rust GPU widget toolkit) | Keep the bottom box deliberately minimal in v1; grow features incrementally. |
| Scope creep (terminal correctness long tail) | `alacritty_terminal` carries VT correctness; we own only *presentation* — keep that line bright. |

---

## 11. What ports from Farx

- **Grid-tiling geometry** (`cols = ceil(sqrt(n))`, full-tile cap, LRU demotion) →
  `crew-render` layout.
- **AI provider layer** (`farx-ai`) → `crew-core`'s model client.
- **`portable-pty` usage** → `crew-term`.
- **Click-to-focus reliability** principle → keyboard/focus model.
- **File-size discipline** (Farx's small-file rule) → retained as a project
  convention for Crew's modules.

New: `crew-render`'s GPU renderer, the winit/input layer, `crew-core`'s
orchestration, `crew-image`.

---

## 12. v1 scope vs later

**v1 (this spec):**
- Native GPU terminal that builds and runs (winit/wgpu/glyphon/cosmic-text +
  alacritty_terminal/portable-pty), renders a working shell pane.
- Auto-tiling agent grid with focus model.
- Bottom orchestrator box (minimal input widget + transcript).
- Orchestrator spawns **N real CLI-agent panes**, delegates in parallel, aggregates
  via the boundary convention.
- Inline images (Kitty + Sixel), frame-capable.

**Phase 2+:**
- Native API sub-agents (transcript panes) → full hybrid.
- Richer orchestration (retries, dependencies between sub-tasks, agent-to-agent via
  the orchestrator).
- Inline video (feature-flagged, on top of the frame-capable image path).
- Config/theming, Windows hardening, remote-mux exploration.

---

## 13. Testing strategy

- **`crew-core`:** unit tests against a fake `PaneController` — decomposition,
  delegation, aggregation, and result-boundary parsing tested fully headless.
- **`crew-term`:** feed canned PTY byte streams, assert `renderable_content()` —
  pins the `alacritty_terminal` adapter behavior across version bumps.
- **`crew-image`:** decode + atlas-upload tests; Kitty/Sixel parse tests against
  known payloads.
- **`crew-render`:** hardest to unit-test; rely on geometry unit tests (grid math)
  + manual/visual verification and screenshot checks for the GPU path.
- **Integration:** a smoke test that boots the window, spawns a shell pane, and
  verifies output renders (headless where the platform allows).

---

## 14. Open questions (resolve during planning, not blocking)

1. Workspace layout: single binary crate with modules, or a Cargo workspace of
   `crew-*` crates? (Leaning workspace for clean boundaries + faster incremental
   builds.)
2. Async runtime: tokio (matches `farx-ai`) vs. a thread-per-PTY + small executor.
3. Exact reserved focus chord(s) and whether the orchestrator box is always visible
   or has a show/hide toggle.
4. Which CLI agents ship as first-class spawn targets in v1 (claude-code / codex /
   gemini-cli / plain shell) and how the orchestrator picks among them.
5. Config surface for the result-boundary convention (env var name, sentinel
   format).
```
