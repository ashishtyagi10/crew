# Copilot Instructions for Crew

## Build, test, and lint commands

Use Cargo workspace commands from the repository root:

```sh
# build
cargo build
cargo build --release

# run app
cargo run --release -p crew-app

# lint/format checks
cargo fmt --check
cargo clippy --workspace --all-targets -- -W clippy::all
cargo check --workspace

# tests
cargo test --workspace

# run a single integration test
cargo test -p crew-term --test scrollback osc_title_is_captured
```

## High-level architecture

Crew is a native GPU terminal — a Cargo workspace with four crates:

- `crates/crew-app`: binary entrypoint (`crew`), window + pane layout, input routing, in-pane UI (input bar, settings, sidebar).
- `crates/crew-render`: GPU rendering pipeline (`winit` + `wgpu` + `glyphon`); converts cells to GPU draws with SDF rounded borders.
- `crates/crew-term`: one PTY + terminal grid per pane (`alacritty_terminal` + `portable-pty`), scrollback, OSC title capture.
- `crates/crew-plugin`: chat / agent plugins.

See [docs/CREW.md](../docs/CREW.md) for the guide and
[docs/superpowers/specs/2026-06-20-crew-terminal-design.md](../docs/superpowers/specs/2026-06-20-crew-terminal-design.md)
for the design rationale.

## Key conventions in this codebase

- **File-size discipline is a hard rule:** every `.rs` file stays ≤200 lines. Split a file by responsibility before it grows past that.
- **Clippy is warning-free:** `cargo clippy --workspace --all-targets` must pass clean.
- **Everything renders as tiles, no overlays:** panes auto-tile into a near-square grid; in-pane UI (settings, command palette, help) is laid into a `ratatui` `Buffer` and converted to GPU cells.
- **Terminal keys pass through:** inside a focused terminal pane, all keys except Crew's own chords (mostly `Cmd+…`) pass through to the program.
- **Release process is tag-driven GitHub Actions:** `.github/workflows/release.yml` builds multi-target artifacts on `v*` tags. Tag (`git tag vX.Y.Z && git push origin vX.Y.Z`) to trigger release packaging.
