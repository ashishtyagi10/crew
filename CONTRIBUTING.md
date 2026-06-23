# Contributing to Crew

Thank you for your interest in contributing! Crew is open source and welcomes contributions of all kinds — bug fixes, new features, documentation, and more.

## Getting Started

### Prerequisites

- [Rust](https://rustup.rs/) (stable toolchain)
- Git

### Setup

```sh
git clone https://github.com/ashishtyagi10/crew.git
cd crew
cargo build
cargo run --release -p crew-app
```

A pre-commit hook runs `cargo fmt --check` and `cargo check` automatically on every commit.

## Project Structure

Crew is a Cargo workspace with four crates:

```
crew/
├── crates/
│   ├── crew-app/       # Window, panes, input, in-pane UI
│   ├── crew-render/    # GPU rendering (wgpu + glyphon)
│   ├── crew-term/      # PTY + terminal grid (alacritty_terminal + portable-pty)
│   └── crew-plugin/    # Chat / agent plugins
└── docs/               # Design docs and GitHub Pages site
```

`crew-app` (the binary) drives the window, lays out panes, and routes input;
`crew-render` owns the GPU pipeline; `crew-term` wraps each pane's PTY and grid;
`crew-plugin` hosts chat/agent plugins. See [docs/CREW.md](docs/CREW.md) for the
architecture overview and [docs/superpowers/specs/2026-06-20-crew-terminal-design.md](docs/superpowers/specs/2026-06-20-crew-terminal-design.md) for the design rationale.

## Hard Rules

- Every `.rs` file stays **≤200 lines** — split a file by responsibility before it grows past that.
- `cargo clippy --workspace --all-targets` is warning-free.

## Code Style

- Run `cargo fmt` before committing (the pre-commit hook enforces this)
- Keep `cargo clippy` warnings clean
- Prefer `anyhow::Result` for error handling in application code
- Use `tracing` macros (`info!`, `warn!`, `error!`) instead of `println!` / `eprintln!`

## Coverage

Install and run coverage locally:

```sh
cargo install cargo-llvm-cov
cargo llvm-cov clean --workspace
cargo llvm-cov --workspace --all-features --html
```

Scoped coverage target used in CI:

```sh
cargo llvm-cov clean --workspace
cargo llvm-cov -p crew-term -p crew-render -p crew-plugin --all-features --summary-only
```

## Pull Request Process

1. Fork the repository
2. Create a feature branch: `git checkout -b my-feature`
3. Make your changes
4. Ensure `cargo fmt --check` and `cargo check --workspace` pass
5. Commit with a clear message
6. Push and open a Pull Request against `main`

Keep PRs focused — one feature or fix per PR. If a change is large, open an issue first to discuss the approach.

## Reporting Bugs

Open an issue at [github.com/ashishtyagi10/crew/issues](https://github.com/ashishtyagi10/crew/issues) with:

- Your OS and GPU
- Steps to reproduce
- Expected vs actual behavior
- Screenshots if relevant

## License

By contributing, you agree that your contributions will be licensed under the same dual license as the project: MIT or Apache-2.0, at your option.
