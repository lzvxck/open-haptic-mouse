# Open Haptic Mouse verification map

Maintained source for verifying user-facing behavior. Read this index, then use the matching feature file as the recipe. `OHM` means `node .claude/skills/verify-open-haptic-mouse/ohm.mjs`.

## Baseline preconditions

- Windows, repo root as the working directory, `$env:Path="$env:USERPROFILE\.cargo\bin;$env:Path"`.
- `pnpm install` done in `ohm-desktop` (provides `puppeteer-core`).
- App started by this run with `OHM start`, then `OHM doctor` exits 0 (sidebar says exactly `Simulator mode`).
- The app starts on scenario `single_press` with the UI defaults: actuation 500, release hysteresis 50, rapid trigger window 20 ms, haptic intensity 128.
- Never drive an instance this run did not start.

## Driving conventions

- Start each recipe from the baseline: `OHM reset` (reloads the page: scenario `single_press`, default profile, speed 0.25x), then `OHM select <name>`.
- Read state with `OHM events` (event names), `OHM counters pad|real`, and `OHM set` (prints the four slider values). Use `OHM eval` only for one-off reads.
- Every helper command exits non-zero with a message on failure (unknown scenario, missing or disabled button, bad count) and never hangs.
- Evidence goes to `.verify/evidence/` via `OHM shot <feature-id>-<step>`.

## Proof and skip reporting

- Capture the action and the resulting state: the event rows or counters plus a screenshot.
- Latency is MODELLED. Report it as modelled, never as measured.
- Pad counters are simulator output; real click counters are real pointer events. Report them separately.
- If a precondition cannot be met, report the failing command and the unmet precondition. Do not substitute another path and call it verified.

## Feature entry contract

Each feature file has an H1, one paragraph, then exactly four H2s: `Sub-features`, `How to get to it (user POV)`, `Driving it with ohm.mjs`, `Gotchas`.

## Not drivable from here

- **Firmware (`ohm-firmware`)** compiles for the STM32G474RE but has never run on hardware or enumerated on a host. Verify with `cd ohm-firmware; cargo build --release` and `cargo clippy --release -- -D warnings` only. Say plainly that USB enumeration, HID delivery, HSI48/CRS lock and the LD2 pulse are unverified.
- **Real LDC1101 sensor, DRV2605L driver, LRA vibration**: no hardware exists; any number involving them is modelled.

## Features

- [Scenario events](./scenario-events.md) covers choosing a scenario and reading the Down/Up/RapidDown event table with modelled latency.
- [Profile controls](./profile-controls.md) covers the four sliders, the clamp that keeps the profile valid, the backend error text and chart re-render.
- [Playback and click test pad](./playback-pad.md) covers time playback, the playhead and the simulated pad counters.
- [Real click check](./real-click-check.md) covers the box that counts real pointer events.
- [CLI simulator](./cli-simulator.md) covers `ohm-simulator` on the terminal, no app needed.
