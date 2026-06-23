#!/usr/bin/env bash
#
# Crew Enhancement Agent Runner
#
# Runs the /enhance Claude Code command in one-shot or loop mode.
#
# Usage:
#   ./scripts/enhance-agent.sh              # Interactive one-shot (asks before implementing)
#   ./scripts/enhance-agent.sh --auto       # Auto mode: implements top 3 without asking
#   ./scripts/enhance-agent.sh --loop       # Loop: runs interactively, repeats after each cycle
#   ./scripts/enhance-agent.sh --loop-auto  # Loop + auto: fully autonomous continuous improvement
#   ./scripts/enhance-agent.sh --loop-auto --interval 3600  # Loop every hour
#
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

# Defaults
MODE="interactive"
LOOP=false
INTERVAL=0  # 0 = no wait between cycles (prompt immediately)
MAX_CYCLES=0  # 0 = unlimited

# Parse arguments
while [[ $# -gt 0 ]]; do
    case "$1" in
        --auto)
            MODE="auto"
            shift
            ;;
        --loop)
            LOOP=true
            shift
            ;;
        --loop-auto)
            LOOP=true
            MODE="auto"
            shift
            ;;
        --interval)
            INTERVAL="$2"
            shift 2
            ;;
        --max-cycles)
            MAX_CYCLES="$2"
            shift 2
            ;;
        --help|-h)
            cat <<'HELP'
Crew Enhancement Agent Runner

Runs the Claude Code /enhance command to autonomously analyze, research,
plan, implement, and document improvements to Crew.

USAGE:
    ./scripts/enhance-agent.sh [OPTIONS]

OPTIONS:
    --auto          Skip user confirmation, implement top 3 enhancements
    --loop          Run continuously (re-invokes after each cycle)
    --loop-auto     Combine --loop and --auto for fully autonomous operation
    --interval N    Wait N seconds between loop cycles (default: 0)
    --max-cycles N  Stop after N cycles (default: 0 = unlimited)
    --help, -h      Show this help

EXAMPLES:
    # One-shot interactive: shows proposals, asks what to implement
    ./scripts/enhance-agent.sh

    # One-shot auto: researches and implements top 3 immediately
    ./scripts/enhance-agent.sh --auto

    # Continuous improvement, implements top 3 each cycle, 1hr gap
    ./scripts/enhance-agent.sh --loop-auto --interval 3600

    # Run 5 interactive cycles back-to-back
    ./scripts/enhance-agent.sh --loop --max-cycles 5

REQUIREMENTS:
    - Claude Code CLI (`claude`) must be installed and authenticated
    - Must be run from the crew project root (or scripts/ dir)
HELP
            exit 0
            ;;
        *)
            echo "Unknown option: $1 (try --help)"
            exit 1
            ;;
    esac
done

# Verify claude CLI exists
if ! command -v claude &>/dev/null; then
    echo "Error: 'claude' CLI not found. Install Claude Code first."
    echo "  https://docs.anthropic.com/en/docs/claude-code"
    exit 1
fi

cd "$PROJECT_DIR"

# Verify we're in the right repo
if [[ ! -f "Cargo.toml" ]] || ! grep -q "crew" Cargo.toml 2>/dev/null; then
    echo "Error: not in crew project root (expected Cargo.toml with crew)"
    exit 1
fi

CYCLE=0

run_cycle() {
    CYCLE=$((CYCLE + 1))
    local timestamp
    timestamp="$(date '+%Y-%m-%d %H:%M:%S')"

    echo "========================================"
    echo " Crew Enhancement Agent - Cycle $CYCLE"
    echo " Mode: $MODE | $(date)"
    echo "========================================"
    echo ""

    # Run the /enhance command via Claude Code
    # --print for non-interactive auto, normal for interactive
    if [[ "$MODE" == "auto" ]]; then
        claude --print "/enhance auto"
    else
        claude "/enhance interactive"
    fi

    local exit_code=$?

    echo ""
    echo "--- Cycle $CYCLE completed at $(date '+%H:%M:%S') (exit: $exit_code) ---"
    echo ""

    return $exit_code
}

if [[ "$LOOP" == false ]]; then
    # One-shot mode
    run_cycle
else
    # Loop mode
    echo "Starting enhancement loop (mode=$MODE, interval=${INTERVAL}s, max=$MAX_CYCLES)"
    echo "Press Ctrl+C to stop."
    echo ""

    while true; do
        run_cycle || true  # Don't exit loop on failure

        # Check max cycles
        if [[ "$MAX_CYCLES" -gt 0 && "$CYCLE" -ge "$MAX_CYCLES" ]]; then
            echo "Reached max cycles ($MAX_CYCLES). Stopping."
            break
        fi

        # Wait between cycles
        if [[ "$INTERVAL" -gt 0 ]]; then
            echo "Waiting ${INTERVAL}s before next cycle..."
            sleep "$INTERVAL"
        fi
    done
fi
