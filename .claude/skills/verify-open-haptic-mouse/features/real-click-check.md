# Real click check

A dashed target square counts real `pointerdown` and `pointerup` events the window receives. Today any mouse works; once the STM32 board enumerates as a USB mouse its clicks land here for comparison against the simulated counts.

## Sub-features

- `RC-1` left-button pointerdown and pointerup are counted separately.
- `RC-2` `Reset counters` zeroes both.
- `RC-3` the box is labelled `real mouse events received by this window (left button)`.

## How to get to it (user POV)

- The `Real OS click check` card in the lower right; click inside the dashed `click here` square.

## Driving it with ohm.mjs

Preconditions: baseline (`OHM reset`).

- `OHM realclick 3`, then `OHM counters real`. Expect `{"pointerdown":3,"pointerup":3}` (RC-1). The count must be a positive integer.
- RC-2: `OHM click "Reset counters"` then `OHM counters real`; expect both 0.
- RC-3: `OHM eval "document.querySelector('.checks .card:nth-child(2) .modelled').textContent"` is `"real mouse events received by this window (left button)"`.
- Evidence: `OHM realclick 3`, then `OHM shot real-click-3`.

## Gotchas

- `OHM realclick` is browser-level input delivered over CDP into the page; it proves the counting code, not that a USB mouse reaches the OS. Say so in the report.
- Only the left button counts; an up with no matching down is ignored.
- This feature cannot verify the firmware. Real USB clicks need the board with a USB connector wired to PA11/PA12, which does not exist yet.
