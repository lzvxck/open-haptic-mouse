# Playback and click test pad

The scenario plays back in time with a moving playhead on the chart. A virtual button pad travels down, turns pressed, flashes on the modelled haptic click, and counts simulated Down, RapidDown and Up events.

## Sub-features

- `PP-1` Play/Pause/Restart and speed (0.25x, 1x, 4x) control playback; default speed is 0.25x.
- `PP-2` the pad counters (Down, RapidDown, Up) follow the playhead.
- `PP-3` the pad is labelled `SIMULATED firmware output, not real mouse events`.
- `PP-4` clicking or dragging the chart seeks the playhead.

## How to get to it (user POV)

- Top bar of the main area (Play, Restart, speeds, Loop) and the `Click test pad` card in the lower left.

## Driving it with ohm.mjs

Preconditions: baseline; scenario `double_press_fast`.

- `OHM click Restart`, then `OHM click 4x`, then `OHM click Play`.
- Poll `OHM eval "Object.fromEntries([...document.querySelectorAll('.checks .card:nth-child(1) .counters > div')].map(d => [d.querySelector('dt').textContent, Number(d.querySelector('dd').textContent)]))"` until it returns `{"Down":1,"RapidDown":1,"Up":2}` (PP-2). At 4x the whole run takes about 90 ms, so a single read after about 1 s is enough.
- For a mid-run look, use speed `0.25x` (default): after `Play`, the counters read `Down:1, RapidDown:0, Up:1` around 33 ms of trace time (about 130 ms wall time). Take `OHM shot playback-pad-mid` while paused with `OHM click Pause`.
- PP-3: `OHM eval "document.querySelector('.checks .card:nth-child(1) .modelled').textContent"` contains `SIMULATED`.
- Evidence: `OHM shot playback-pad-end` after the final counters.

## Gotchas

- Changing the scenario or any slider resets the playhead to 0 and pauses; re-press Play.
- The playback button label toggles between `Play` and `Pause`; match the exact visible label.
- Counters are cumulative over one playback; `Restart` zeroes them.
- Pad counts are simulator output; do not report them as OS-level clicks.
