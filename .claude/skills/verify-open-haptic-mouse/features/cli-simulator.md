# CLI simulator

`ohm-simulator` replays a scenario JSON through the core pipeline on the terminal and prints each event, optionally with the modelled haptic latency.

## Sub-features

- `CL-1` plain run prints `t=<us>us DOWN|UP|RAPID_DOWN` lines.
- `CL-2` `--latency` adds `detect_to_trigger_us=<n>` on trigger events and a final `MODELLED, not measured on hardware` line.
- `CL-3` an unreadable or invalid scenario exits non-zero with a message and no panic.

## How to get to it (user POV)

- `cargo run -q -p ohm-simulator -- [--latency] <scenario.json>` from the repo root.

## Driving it with ohm.mjs

Preconditions: repo root, Rust toolchain on `PATH`. No app launch needed.

- `cargo run -q -p ohm-simulator -- ohm-simulator/scenarios/single_press.json` prints `t=20000us DOWN` and `t=52000us UP`, exit 0 (CL-1).
- `cargo run -q -p ohm-simulator -- --latency ohm-simulator/scenarios/double_press_fast.json` prints, in order: `t=10000us DOWN detect_to_trigger_us=1500`, `t=21000us UP`, `t=35000us RAPID_DOWN detect_to_trigger_us=1500`, `t=46000us UP`, then the MODELLED line (CL-2).
- `cargo run -q -p ohm-simulator -- does-not-exist.json` exits non-zero with `no se pudo leer` (CL-3); check `$LASTEXITCODE`.
- Save stdout, stderr and exit code to `.verify/evidence/cli-<name>.txt`.

## Gotchas

- `--latency` mode uses a 20 ms rapid trigger window; the plain run uses the core default window of 0, so `double_press_fast` prints `DOWN, UP, DOWN, UP` without `--latency`.
- The latency value is the constant `trigger_delay_us = 1500`, an unverified DRV2605L assumption.
- PowerShell prints a `NativeCommandError` banner when a native command writes to stderr; judge by `$LASTEXITCODE`, not by the banner.
