# Scenario events

Choosing a scenario runs it through the Rust simulator with the current profile and lists each event with its time and the modelled detect-to-trigger latency.

## Sub-features

- `SE-1` event table shows the scenario's events in order with time (ms), event name, position and latency.
- `SE-2` Rapid Trigger: with the 20 ms default window a Down within 20 ms of the previous Up is `RapidDown`; later or equal is a plain `Down`.
- `SE-3` a scenario with noise below the actuation threshold emits no events.
- `SE-4` the table is labelled `MODELLED, not measured on hardware`.

## How to get to it (user POV)

- Open the app; the scenario `<select>` is the first control in the left panel.
- The event table is at the bottom of the main area.

## Driving it with ohm.mjs

Preconditions: baseline from the README. `OHM select <name>` fails on an unknown name, then waits 400 ms for the debounce and Tauri round trip. `OHM events` prints the event names as JSON (`[]` when the table only shows the "No events" placeholder).

- `OHM select double_press_fast`, `OHM events`. Expect `["Down","Up","RapidDown","Up"]` (SE-1, SE-2). Times are 10, 21, 35, 46 ms: `OHM eval "[...document.querySelectorAll('tbody tr')].map(r => r.children[0].textContent)"` is `["10","21","35","46"]`.
- `OHM select double_press_slow`. Expect `["Down","Up","Down","Up"]` (second Down is outside the window).
- `OHM select double_press_edge`. Expect `["Down","Up","Down","Up"]`: the gap is exactly 20 ms and the window bound is exclusive.
- `OHM select noisy_hold`. Expect `[]` and `OHM eval "!!document.querySelector('.error')"` is `false` (SE-3).
- `OHM select long_hold`. Expect exactly `["Down","Up"]`, no repeats.
- SE-4: `OHM eval "document.querySelector('main > section > .modelled').textContent"` is `"MODELLED, not measured on hardware"`. (`.modelled` also matches the two card labels, which come first in the DOM.)
- Evidence: `OHM shot scenario-events-double_press_fast` after the first step.

## Gotchas

- Scenario values are the file stems (`double_press_fast`), not display text.
- `Up` position reads 400 because the sample after release is below the release line; that is expected.
- If `.panel .mode` mentions `browser mock`, results come from the JS mock, not the Rust simulator. Stop and fix the launch.
