# Crew Architecture

Crew is a from-scratch, native **GPU terminal** (Rust · `winit` + `wgpu` + `glyphon`)
that doubles as a **swarm orchestrator**: give it a goal and it decomposes the work
and runs a pool of agents toward it, rendered as a live constellation/heatmap.

The workspace is five crates: `crew-app` (the application), `crew-render` (GPU
pipeline), `crew-term` (PTY + terminal grid), `crew-plugin` (subprocess agent
plugins), and `crew-hive` (the orchestration engine).

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
│  input bar (/cmd) ──► command dispatch ──► spawn panes / /swarm goal           │
│                                                                                │
│  ┌── pane model (auto-tiling near-square grid; LRU cap = 6 full tiles) ─────┐  │
│  │   PaneContent::                                                          │  │
│  │     Terminal ─┐   Chat ─┐   Settings   Far   [Swarm]*                    │  │
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

  * Swarm pane + bridge are wired & tested headlessly (branch feat/crew-app-swarm);
    GPU draw of the pane is the deferred step needing a display.
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
                   ▼                              │ view: fleet_view        │
        ┌──────────────────────┐                 │  Constellation │Heatmap │
        │ Blackboard           │◄── gather deps ─┤  (mode by count)        │
        │  TaskResults +        │   merge upward  │  render_cells→glyph grid│
        │  artifacts (Arc<RwLock>)                └─────────────┬───────────┘
        └──────────────────────┘                               │
                                                                ▼  to crew-app SwarmPane
```

## How it fits together

- **crew-app** owns the window, input, and the auto-tiling pane grid. Each pane
  renders to `CellView`s; the LRU cap keeps ≤6 full tiles and demotes the rest to
  a minimized strip.
- **crew-render** paints `CellView` grids on the GPU (text via glyphon, rounded
  borders via an SDF shader). Everything is cells — no overlays.
- **crew-term** backs Terminal panes (PTY, scrollback, OSC titles); **crew-plugin**
  backs Chat/agent panes via JSON-over-stdio subprocesses.
- **crew-hive** is the engine: a goal is decomposed by the planner into a task
  graph, executed by a tokio scheduler over a bounded pool of agents (stub, native
  LLM, or remote/sidecar), with results merging through the blackboard and
  telemetry streaming over the event bus into the constellation/heatmap view.

See the design spec at
[`docs/superpowers/specs/2026-06-27-crew-agent-swarm-design.md`](superpowers/specs/2026-06-27-crew-agent-swarm-design.md)
and the implementation plans under `docs/superpowers/plans/`.
