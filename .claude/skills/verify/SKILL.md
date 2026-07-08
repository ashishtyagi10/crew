---
name: verify
description: Drive the live crew GUI (winit app) to verify changes end-to-end on macOS — build, launch isolated, inject keys/mouse safely, screenshot.
---

# Verifying crew changes against the live app

## DANGER: the user's own crew session is probably running

The Claude Code session itself often runs *inside* a pane of the user's live
crew app. That app is usually frontmost, and **global keystroke/scroll events
go to the frontmost app** — blind `osascript keystroke` calls will type into
the user's live agent chats. Two rules, no exceptions:

1. Never target `first process whose name is "crew"` — there may be two.
2. Before EVERY keystroke batch, verify focus by PID and abort otherwise:
   ```applescript
   if unix id of (first process whose frontmost is true) is not <DEVPID> then return "ABORT"
   ```

## Recipe

- Build: `cargo build -p crew-app` → binary at `target/debug/crew`.
- Launch isolated (own config/session, fresh welcome screen):
  `(HOME="$SCRATCH/home" CREW_BROKER_MOCK_REPLY="mock" nohup target/debug/crew >log 2>&1 &)`
- First cold launch can take ~2 min before the window appears (shader/font
  caches) and it comes up UNFOCUSED. `set frontmost` via System Events and
  `NSRunningApplication.activate` are both denied on macOS 14+. Fix: once
  caches are warm, kill and relaunch — a warm launch self-activates and is
  frontmost within ~2 s. Poll `count of windows` + frontmost pid.
- Keystrokes: `osascript` `keystroke`/`key code` (36 = Return) after the PID
  guard above. App chords: Cmd+T shell, Cmd+1..9 focus pane, Cmd+Z zoom,
  Cmd+I input bar. `/md <abs path>` in the input bar opens the zoomed
  markdown viewer. The first Return after a long `keystroke` string can be
  swallowed — screenshot, and re-send Return if the input bar still shows
  the text.
- Mouse (move/scroll/click) — osascript can't scroll; compile a tiny Swift
  CGEvent tool (`scrollWheelEvent2Source`, units .line; leftMouseDown/Up).
  Clicks land on the topmost window at that point: click only after the PID
  guard, and prefer coordinates inside the dev window's frame
  (`position`/`size of window 1` via System Events).
- Screenshots: `screencapture -x -R0,33,1512,949 out.png`. Retina: image
  pixels = 2 × screen points; screen point = (img_x/2, img_y/2 + 33) for a
  capture region starting at y=33.
- Cleanup: `kill <DEVPID>`; the isolated `$SCRATCH/home` keeps the user's
  real config untouched.
