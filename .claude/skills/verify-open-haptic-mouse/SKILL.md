---
name: verify-open-haptic-mouse
description: Launch and drive the Open Haptic Mouse desktop app (Tauri 2 + React, simulator mode) and the ohm-simulator CLI on Windows, over WebView2 CDP, to prove behavior with screenshots and counters. Use after changing ohm-core, ohm-simulator, ohm-protocol, or ohm-desktop, or when asked to verify the UI, a scenario, the click test pad, or the simulated haptic latency.
---

# Verify Open Haptic Mouse

Surfaces, in order of what a user touches: the desktop app (simulator mode, driven here), the `ohm-simulator` CLI (terminal), and the firmware (compile-only: it has never run on hardware and cannot be driven from this repo, see Gotchas in `features/README.md`).

Everything below runs from the repo root in PowerShell. The helper is `node .claude/skills/verify-open-haptic-mouse/ohm.mjs <command>` (called `OHM` below).

## Launch

```powershell
$env:Path="$env:USERPROFILE\.cargo\bin;$env:Path"
node .claude/skills/verify-open-haptic-mouse/ohm.mjs start
```

`start` rebuilds the debug exe (`pnpm tauri build --debug --no-bundle`, about a minute cold) when it is missing or older than the sources (the `SOURCES` list in `ohm.mjs`: Rust workspace files, `Cargo.lock`, the frontend, the Tauri config and capabilities); after a build that exits 0 it only requires the exe to exist. It launches `target\debug\ohm-desktop.exe` with `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=<free port>` and its own `WEBVIEW2_USER_DATA_FOLDER`, then waits until the sidebar (`.panel .mode`), at least one scenario `<option>`, the chart (a rendered result) and `.real-target` exist, and the Tauri IPC bridge answers. It prints `ready: pid=... cdp=... evidence=...`. If readiness fails, `start` stops the instance it launched and exits non-zero. State is in `.verify/state.json`.

Why the separate user-data folder: a running `pnpm tauri dev` holds the default WebView2 profile and a second instance with different args dies with error 0x8007139F. Using our own folder lets both run side by side. The ohm-desktop window opens on screen; do not click in it by hand during a run.

Needs: Rust stable (MSVC), Node + pnpm, `pnpm install` done in `ohm-desktop` (provides `puppeteer-core`), WebView2 runtime (ships with Windows 11).

## Doctor

```powershell
node .claude/skills/verify-open-haptic-mouse/ohm.mjs doctor
```

Read-only. Prints JSON and exits non-zero unless: an instance was started by this skill, its pid is alive and is an `ohm-desktop.exe`, CDP answers, `__TAURI_INTERNALS__` is present, the sidebar reads exactly `Simulator mode` (not `... browser mock ...`, which means the page is not running inside Tauri), and the exe is not older than the sources. Run it first whenever anything looks off.

## Drive

Page-level commands attach over CDP to the instance in `.verify/state.json` and refuse to run without one, so the skill never drives an app it did not start.

All page commands exit non-zero with a message on failure and never hang.

- `OHM reset` reloads the page and waits until it is ready again (baseline: default profile, speed 0.25x).
- `OHM select double_press_fast` chooses a scenario (the `<select>` values are the scenario names: `single_press`, `double_press_fast`, `double_press_slow`, `double_press_edge`, `noisy_hold`, `long_hold`). It fails on an unknown name and waits 400 ms for the result.
- `OHM click Play` presses a button by its exact visible label (`Play`/`Pause` is one toggle, `Restart`, `0.25x`, `1x`, `4x`, `Reset counters`). It waits up to 3 s for the button to exist and be enabled, then fails.
- `OHM set <actuation|hysteresis|window|haptic> <n>` sets a slider and prints all four values. `OHM events` prints the event-table names as JSON. `OHM counters pad|real` prints the counters of the simulated pad or the real click box.
- `OHM seek <ms>` clicks the chart at that trace time (pause first) and prints the clock. `OHM realclick 3` sends 3 real left clicks (CDP `Input.dispatchMouseEvent`: move, press, release) to the center of `.real-target`; browser-level input into the page, not an OS-level click.
- `OHM eval "<js expression>"` evaluates in the page and prints the JSON result (promises are awaited). Stable handles: `.panel .mode`, `.chart text.threshold-label.actuation|release`, `.chart.stale`, `.error[role=alert]`, `.clock`, `.real-target`, `main > section > .modelled` (the table label; the two card labels are also `.modelled`).

`pnpm e2e` (`cd ohm-desktop; pnpm e2e`, does its own launch and cleanup, screenshots to `E2E_SHOTS`) asserts only: the sidebar is not the mock and the IPC bridge is present, `double_press_fast` events and the 20 ms default, 4x pad counters `Down 1, RapidDown 1, Up 2`, three real clicks counted, one actuation slider change to 520 with no error, and no console errors. Everything else (other scenarios, PC-2 to PC-4, PP-4, RC-2, the CLI) is covered only by the recipes in `features/`.

The CLI surface needs no launch: `cargo run -q -p ohm-simulator -- --latency ohm-simulator/scenarios/<name>.json`.

## Evidence

Write proofs to `.verify/evidence/` (gitignored, survives `stop`): `OHM shot <name>` saves `<name>.png` of the whole page; save command output of CLI runs next to it. A proof is valid only if it shows the action and the resulting state: the event table rows, the pad counters, and the screenshot with the app visible. Rules:

- Drive the real app path (the Tauri window, the real `invoke` commands). A run where `doctor` says `browser mock` is not a proof of the Rust simulator.
- Every latency number is MODELLED (`trigger_delay_us = 1500`, an unverified DRV2605L assumption). Never report it as measured.
- The simulated pad counts the simulator's output; the real click box counts real pointer events. A pad count is not proof that an OS click happened. Report them separately.
- Do not claim firmware or USB behavior from anything here. The firmware is verified by `cargo build --release` and clippy only.

## Cleanup

```powershell
node .claude/skills/verify-open-haptic-mouse/ohm.mjs stop
```

Kills only the pid recorded in `.verify/state.json` (process tree), and only if that pid is currently an `ohm-desktop.exe` (a reused pid is treated as stale state: cleaned, nothing killed). Then it deletes the temp WebView2 profile. If the kill fails it keeps the state and profile and exits non-zero. Never kill `ohm-desktop.exe` or `node` by name: the user's `pnpm tauri dev` instance may be running. Evidence in `.verify/evidence/` is kept. Run `stop` after a failed attempt too.

## Helpers

`ohm.mjs` is the only script (commands: start, doctor, stop, eval, click, select, reset, set, events, counters, seek, realclick, shot). With no or an unknown command it prints usage and exits 2. `puppeteer-core` is loaded only by page commands, so `stop` and usage work on a fresh checkout. `ohm-desktop/e2e/run.mjs` (`pnpm e2e`) is the maintained regression and the source of the selectors above.

Feature recipes live in `features/`; start at `features/README.md`.
