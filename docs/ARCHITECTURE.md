# Crew Architecture

Crew is a from-scratch, native **GPU terminal** (Rust · `winit` + `wgpu` + `glyphon`)
that doubles as a **swarm orchestrator**: give it a goal and it decomposes the work
and runs a pool of agents toward it, rendered as a live per-task status list.

The workspace is six crates: `crew-app` (the application), `crew-render` (GPU
pipeline), `crew-term` (PTY + terminal grid), `crew-plugin` (subprocess agent
plugins), `crew-theme` (the theme presets + palette contracts), and `crew-hive`
(the orchestration engine).

## Application architecture

```
╔══════════════════════════════════════════════════════════════════════════════╗
║                            CREW  —  GPU Swarm Terminal                         ║
╚══════════════════════════════════════════════════════════════════════════════╝

  user (keyboard / mouse)
        │
        ▼
┌──────────────────────────────────────────────────────────────────────────────┐
│ crew-app          the application: winit window, input routing, frame loop     │
│                                                                                │
│  input bar (/cmd · !cmd · *line) ──► command dispatch ──► spawn panes / goal   │
│                                                                                │
│  ┌── pane model (auto-tiling near-square grid; LRU cap = 6 full tiles) ─────┐  │
│  │   PaneContent::                                                          │  │
│  │     Terminal ─┐   Chat ─┐   Settings   Far   Md   [Swarm]*               │  │
│  │   minimized strip (LRU demotion of overflow tiles)                       │  │
│  │   each pane → Vec<CellView>  →  PaneScene (fieldset card: border+legend) │  │
│  └────────┬───────────────┬───────────────────────────┬───────────────────┘  │
│           │               │                            │                       │
│   per-frame mpsc poll (PTYs · plugins · swarm bridge — non-blocking)           │
└───────────┼───────────────┼────────────────────────────┼──────────────────────┘
            │               │                            │
            ▼               ▼                            ▼
   ┌────────────────┐ ┌────────────────┐   ┌────────────────────────────────┐
   │ crew-term      │ │ crew-plugin    │   │ swarm bridge*  (off-thread)     │
   │ PtyTerm (PTY)  │ │ subprocess     │   │  tokio runtime on worker thread │
   │ grid+scrollback│ │ host (JSON     │   │  EventBus ─► std mpsc ─► Fleet  │
   │ OSC titles     │ │ stdio)         │   │  Fleet ─► swarm_cells ─► CellView│
   └────────────────┘ │ orchestrator   │   └───────────────┬────────────────┘
                      └────────────────┘                   │ drives
                                                           ▼
        ┌───────────────────────────────────────────────────────────────────┐
        │ crew-hive          the "Hive" orchestration engine                  │
        └───────────────────────────────────────────────────────────────────┘
            │
            ▼ rendering
┌──────────────────────────────────────────────────────────────────────────────┐
│ crew-render       GPU pipeline:  CellView grid ──► wgpu draws (text via        │
│                   glyphon) + SDF rounded-border shader.  No overlays.          │
└──────────────────────────────────────────────────────────────────────────────┘

  * Swarm pane + bridge are wired on main (/goal, /batch) & tested
    headlessly; the GPU draw of the pane's CellViews shares this render path.
```

## crew-hive — the orchestration engine

Runs an arbitrary DAG of agents to completion on its own thread. Headless and
fully unit/integration-tested; bring-your-own-LLM per agent.

```
        goal / batch of jobs
                │
                ▼
        ┌───────────────┐        ┌──────────────────────────────────────────┐
        │ Planner       │        │ Provider (bring-your-own-LLM)             │
        │  StubPlanner  │◄──────►│  trait Provider                          │
        │  LlmPlanner   │        │   ├─ MockProvider      (tests)            │
        └──────┬────────┘        │   └─ AnthropicProvider (reqwest /messages)│
               │ TaskGraph       │  ModelTier→model: haiku/sonnet/opus       │
               │ (DAG)           └──────────────────────────────────────────┘
   batch_graph │  (flat parallel-jobs graph = same engine, other shape)
               ▼
        ┌──────────────────────────────────────────────────────────────────┐
        │ Scheduler   tokio JoinSet + Semaphore(concurrency cap)            │
        │   ready()→spawn ▸ failure cascade-cancel ▸ panic→failed          │
        │   with_cancel(AtomicBool) ◄── budget_governor (cost cap)         │
        └───────┬───────────────────────────────────────────┬──────────────┘
                │ dispatch (bounded pool)                    │ events
                ▼                                            ▼
   ┌──────────────────────────────┐              ┌─────────────────────────┐
   │ Agent Pool (trait Agent)     │              │ EventBus (broadcast)    │
   │  ├─ StubAgent   (tests)      │── HiveEvent ─►│  state·tokens·cost·out │
   │  ├─ ApiAgent    (LLM, native │              └───────────┬─────────────┘
   │  │              futures)     │                          ▼
   │  └─ RemoteAgent ─► Transport │              ┌─────────────────────────┐
   │        ├─ Loopback (in-proc) │              │ Fleet telemetry         │
   │        └─ stdio worker codec │              │  per-agent + totals     │
   │           (LangGraph/sidecar)│              └───────────┬─────────────┘
   └───────────────┬──────────────┘                          ▼
                   │ read deps / write result    ┌─────────────────────────┐
                   ▼                              │ crew-app swarm/view     │
        ┌──────────────────────┐                 │  task list: glyph+title │
        │ Blackboard           │◄── gather deps ─┤  +last output line      │
        │  TaskResults +        │   merge upward  │  under a fleet HUD row  │
        │  artifacts (Arc<RwLock>)                └─────────────┬───────────┘
        └──────────────────────┘                               │
                                                                ▼  to crew-app SwarmPane
```

## How it fits together

- **crew-app** owns the window, input, and the auto-tiling pane grid. Each pane
  renders to `CellView`s; the LRU cap keeps ≤6 full tiles and demotes the rest to
  a minimized strip.
- **crew-render** paints `CellView` grids on the GPU (text via glyphon, rounded
  borders via an SDF shader). Everything is cells — no overlays. Colours cross
  the sRGB boundary once (linear conversion at the GPU edge) and the whole-pixel
  cell box never changes with the font.
- **crew-theme** defines the theme presets (9 themes: paper, sepia/ink/graphite
  darks, CRT phosphors) and the palette contracts (contrast thresholds,
  dark/grain data fields) every surface draws from.
- **crew-term** backs Terminal panes (PTY, scrollback, OSC titles, OSC 10/11 +
  DSR query replies from the active theme); **crew-plugin**
  backs Chat/agent panes via JSON-over-stdio subprocesses.
- **crew-hive** is the engine: a goal is decomposed by the planner into a task
  graph, executed by a tokio scheduler over a bounded pool of agents (stub, native
  LLM, or remote/sidecar), with results merging through the blackboard and
  telemetry streaming over the event bus into the pane's task-list view.

See the design spec at
[`docs/superpowers/specs/2026-06-27-crew-agent-swarm-design.md`](superpowers/specs/2026-06-27-crew-agent-swarm-design.md)
and the implementation plans under `docs/superpowers/plans/`.
