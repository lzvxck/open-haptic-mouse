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

`start` rebuilds the debug exe (`pnpm tauri build --debug --no-bundle`, about a minute cold) when it is missing or older than the sources, launches `target\debug\ohm-desktop.exe` with `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=<free port>` and its own `WEBVIEW2_USER_DATA_FOLDER`, then waits until the sidebar (`.panel .mode`) exists. It prints `ready: pid=... cdp=... evidence=...`. State is in `.verify/state.json`. Ready means usable: the page is loaded and the Tauri IPC bridge answers.

Why the separate user-data folder: a running `pnpm tauri dev` holds the default WebView2 profile and a second instance with different args dies with error 0x8007139F. Using our own folder lets both run side by side. The ohm-desktop window opens on screen; do not click in it by hand during a run.

Needs: Rust stable (MSVC), Node + pnpm, `pnpm install` done in `ohm-desktop` (provides `puppeteer-core`), WebView2 runtime (ships with Windows 11).

## Doctor

```powershell
node .claude/skills/verify-open-haptic-mouse/ohm.mjs doctor
```

Read-only. Prints JSON and exits non-zero unless: an instance was started by this skill, its pid is alive, CDP answers, `__TAURI_INTERNALS__` is present, the sidebar reads exactly `Simulator mode` (not `... browser mock ...`, which means the page is not running inside Tauri), and the exe is not older than the sources. Run it first whenever anything looks off.

## Drive

Page-level commands attach over CDP to the instance in `.verify/state.json` and refuse to run without one, so the skill never drives an app it did not start.

- `OHM select double_press_fast` chooses a scenario (the `<select>` values are the scenario names: `single_press`, `double_press_fast`, `double_press_slow`, `double_press_edge`, `noisy_hold`, `long_hold`).
- `OHM click Play`, `OHM click Pause`, `OHM click Restart`, `OHM click 4x` press a button by its exact visible label.
- `OHM eval "<js expression>"` evaluates in the page and prints the JSON result. Stable handles used by the feature recipes: `.panel .mode`, `tbody tr` (event table rows; cell 2 is the event), `.checks .card:nth-child(1) .counters` (simulated pad), `.checks .card:nth-child(2) .counters` (real click box), `.real-target`, `.chart text.threshold-label.actuation`, `.chart.stale`, `.error[role=alert]`, `.control:nth-of-type(N) input[type=range]` (N=2 actuation, 3 hysteresis, 4 rapid window, 5 haptic intensity; sliders respond to ArrowLeft/ArrowRight).
- `OHM realclick 3` sends 3 real left clicks (CDP `Input.dispatchMouseEvent`: move, press, release) to the center of `.real-target`. It is browser-level input into the page, not an OS-level click.

The fastest complete proof is the maintained script: `cd ohm-desktop; pnpm e2e` (does its own launch and cleanup, asserts every feature below, writes screenshots to `E2E_SHOTS`). Prefer it for a full regression; use the helper when you need to look at or poke one feature.

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

Kills only the pid recorded in `.verify/state.json` (process tree) and deletes its temp WebView2 profile. Never kill `ohm-desktop.exe` or `node` by name: the user's `pnpm tauri dev` instance may be running. Evidence in `.verify/evidence/` is kept. Run `stop` after a failed attempt too.

## Helpers

`ohm.mjs` is the only script (commands: start, doctor, eval, click, select, realclick, shot, stop). With no or an unknown command it prints usage. `ohm-desktop/e2e/run.mjs` (`pnpm e2e`) is the maintained end-to-end regression and the source of the selectors above.

Feature recipes live in `features/`; start at `features/README.md`.
