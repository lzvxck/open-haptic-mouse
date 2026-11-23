# Profile controls

Four sliders edit the profile (actuation, release hysteresis, rapid trigger window, haptic intensity). Changes re-run the scenario automatically and invalid combinations show an error instead of a result.

## Sub-features

- `PC-1` actuation slider moves the actuation line and its label on the chart.
- `PC-2` release hysteresis moves the release line (actuation minus hysteresis).
- `PC-3` rapid trigger window changes whether the second press is `RapidDown`.
- `PC-4` invalid profile (hysteresis greater than actuation) shows an alert and dims the chart.

## How to get to it (user POV)

- Left panel, under the scenario select: Actuation, Release hysteresis, Rapid trigger window, Haptic intensity.

## Driving it with ohm.mjs

Preconditions: baseline; scenario `double_press_fast`.

- PC-1: set the actuation slider (control 2) to 520 with the React-safe setter: `OHM eval "(() => { const s = document.querySelector('.control:nth-of-type(2) input[type=range]'); Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value').set.call(s,'520'); s.dispatchEvent(new Event('input',{bubbles:true})); return s.value })()"`. After about 600 ms, `OHM eval "document.querySelector('.chart text.threshold-label.actuation').textContent"` must be `actuation 520`. Replace `:nth-of-type(2)` and `'520'` to set another slider.
- PC-3: set slider 4 to `0` the same way; the event table must become `["Down","Up","Down","Up"]`. Set it back to `20`; `RapidDown` returns.
- PC-4: set actuation to `100` and hysteresis (slider 3) to `150`. Expect `document.querySelector('.error[role=alert]')` to exist with text containing `release hysteresis (150) cannot exceed actuation (100)` and `.chart.stale` present.
- Wait about 300 ms after each change (debounce plus a Tauri round trip), then read.
- Evidence: `OHM shot profile-controls-pc4`.

## Gotchas

- Setting `input.value` directly does not update React state; use the native setter plus an `input` event, or send real key presses.
- Sliders are not independent: lowering actuation below the hysteresis triggers PC-4. Reset by reloading (`OHM eval "location.reload()"`).
- Rapid window is in ms in the UI and microseconds in the core; the UI default is 20 ms, while the Rust `Profile::default()` is 0.
