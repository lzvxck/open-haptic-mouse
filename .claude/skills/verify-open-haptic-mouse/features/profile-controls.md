# Profile controls

Four sliders edit the profile (actuation, release hysteresis, rapid trigger window, haptic intensity). Changes re-run the scenario automatically. The UI keeps the profile valid by clamping; the Rust backend still rejects an invalid one with a readable error.

## Sub-features

- `PC-1` actuation slider moves the actuation line and its label on the chart.
- `PC-2` release hysteresis moves the release line (actuation minus hysteresis) and its label.
- `PC-3` rapid trigger window changes whether the second press is `RapidDown`.
- `PC-4` the UI cannot build an invalid profile: hysteresis is clamped to actuation minus 1 and actuation to at least 1, with no error banner. The backend rejects an invalid profile with `release hysteresis (X) must be below actuation (Y)` or `actuation must be at least 1`.

## How to get to it (user POV)

- Left panel, under the scenario select: Actuation, Release hysteresis, Rapid trigger window, Haptic intensity.

## Driving it with ohm.mjs

Preconditions: baseline (`OHM reset`, then `OHM select double_press_fast`). `OHM set <actuation|hysteresis|window|haptic> <n>` sets a slider with the React-safe native setter, waits 400 ms for the debounce and Tauri round trip, and prints all four values as JSON.

- PC-1: `OHM set actuation 520` prints `"actuation":520`; `OHM eval "document.querySelector('.chart text.threshold-label.actuation').textContent"` is `"actuation 520"`.
- PC-2: from baseline, `OHM set hysteresis 100`; `OHM eval "document.querySelector('.chart text.threshold-label.release').textContent"` is `"release 400"` (500 - 100).
- PC-3: `OHM set window 0`; `OHM events` becomes `["Down","Up","Down","Up"]`. `OHM set window 20`; `RapidDown` returns.
- PC-4 clamp, from baseline:
  - `OHM set actuation 40` prints `{"actuation":40,"hysteresis":39,...}` (hysteresis followed actuation down to actuation - 1) and the release label is `release 1`.
  - `OHM set hysteresis 150` still prints `"hysteresis":39` (cannot exceed actuation - 1).
  - `OHM set actuation 0` prints `{"actuation":1,"hysteresis":0,...}` (actuation minimum is 1).
  - `OHM eval "[!!document.querySelector('.error[role=alert]'), !!document.querySelector('.chart.stale')]"` is `[false,false]`.
  - `OHM shot profile-controls-pc4`.
- PC-4 backend error, by calling the real command from the page (rejections come back as strings):
  - `OHM eval "window.__TAURI_INTERNALS__.invoke('run_scenario', {name:'single_press', profile:{actuation:100, release_hysteresis:150, rapid_trigger_window_ms:20, haptic_intensity:128}}).then(() => 'resolved', (e) => String(e))"` is `"release hysteresis (150) must be below actuation (100)"`.
  - Same call with `actuation:0, release_hysteresis:0` is `"actuation must be at least 1"`.

## Gotchas

- Setting `input.value` directly does not update React state; `OHM set` uses the native setter plus an `input` event. Real key presses (ArrowLeft/ArrowRight on a focused slider) also work.
- The error banner (`.error[role=alert]`) and `.chart.stale` are not reachable from the sliders because of the clamp; the invoke check above is the only way to see the backend messages from the page.
- `OHM reset` reloads the page and restores the defaults (scenario `single_press`, actuation 500, hysteresis 50, window 20 ms, intensity 128).
- Rapid window is in ms in the UI and microseconds in the core; the UI default is 20 ms, while the Rust `Profile::default()` is 0.
