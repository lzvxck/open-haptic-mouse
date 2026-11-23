# Playback and click test pad

The scenario plays back in time with a moving playhead on the chart. A virtual button pad travels down, turns pressed, flashes on the modelled haptic click, and counts simulated Down, RapidDown and Up events.

## Sub-features

- `PP-1` Play/Pause/Restart and speed (0.25x, 1x, 4x) control playback; default speed is 0.25x.
- `PP-2` the pad counters (Down, RapidDown, Up) follow the playhead.
- `PP-3` the pad is labelled `SIMULATED firmware output, not real mouse events`.
- `PP-4` clicking or dragging the chart seeks the playhead.

## How to get to it (user POV)

- Top bar of the main area (Play, Restart, speeds, Loop, clock) and the `Click test pad` card in the lower left.

## Driving it with ohm.mjs

Preconditions: baseline (`OHM reset`, then `OHM select double_press_fast`; the trace is 355 ms long). Button labels: `Play` or `Pause` (one button that toggles), `Restart`, `0.25x`, `1x`, `4x`. `Restart` rewinds and starts playing, so the toggle then reads `Pause`; `click Play` fails while playing and `click Pause` fails while paused.

- PP-1 at 4x: `OHM click 4x`, `OHM click Restart`, wait about 1 s, `OHM counters pad` is `{"Down":1,"RapidDown":1,"Up":2}` (PP-2) and `OHM eval "[...document.querySelectorAll('.playback button')][0].textContent"` is `"Play"` (playback finished).
- PP-1 at 1x: `OHM click 1x`; `OHM eval "[...document.querySelectorAll('.segmented button')].map(b => b.getAttribute('aria-pressed'))"` is `["false","true","false"]` (the default 0.25x reads `["true","false","false"]`). `OHM click Restart`, wait about 1 s (355 ms of trace), same counters and `"Play"`.
- Mid-run, deterministic (do not race the clock): `OHM click Play`, `OHM click Pause`, then `OHM seek <ms>` and read the pad (PP-2, PP-4). `seek` presses the mouse on the chart at that trace time and prints the clock.
  - `OHM seek 9`: counters `{"Down":0,"RapidDown":0,"Up":0}`.
  - `OHM seek 33`: `{"Down":1,"RapidDown":0,"Up":1}`; `OHM shot playback-pad-mid`.
  - `OHM seek 40`: `{"Down":1,"RapidDown":1,"Up":1}`.
  - `OHM seek 50`: `{"Down":1,"RapidDown":1,"Up":2}`.
- PP-4: each `seek` above prints the clock (`"33.0 / 355 ms"` for `seek 33`); that clock moving to the requested time is the proof that a chart click seeks. Drag uses the same `scrub` handler on pointermove with the button held; the helper drives only the click.
- PP-3: `OHM eval "document.querySelector('.checks .card:nth-child(1) .modelled').textContent"` is `"SIMULATED firmware output, not real mouse events"`.
- Evidence: `OHM shot playback-pad-end` after the final counters.

## Gotchas

- Changing the scenario or any slider resets the playhead to 0 and pauses; `OHM reset` also returns the speed to 0.25x.
- Counters are a pure function of the playhead time (events at or before it); `Restart` zeroes them by rewinding.
- `seek` while playing is overwritten by the next animation frame; pause first.
- Pad counts are simulator output; do not report them as OS-level clicks.
