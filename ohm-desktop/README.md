# Open Haptic Mouse: app de escritorio

App Tauri 2 + React para ver y ajustar el botón de Open Haptic Mouse. Hoy corre en modo simulador: ejecuta los escenarios de `ohm-simulator` y muestra la señal, los eventos y el clic háptico modelado (no medido en hardware).

## Comandos

```sh
pnpm install
pnpm dev        # solo el frontend en el navegador, con un mock en JS que NO es el simulador de Rust
pnpm tauri dev  # la app real, con el simulador de Rust detrás
pnpm test       # vitest
pnpm build      # tsc + vite build
pnpm e2e        # compila la app y la maneja por WebView2 (Windows)
```

`pnpm dev` sirve para iterar la interfaz; para comprobar comportamiento usa `pnpm tauri dev` o `pnpm e2e`.

Los comandos de Tauri (`list_scenarios`, `default_profile`, `run_scenario`) viven en `src-tauri/src/lib.rs`; la simulación está en `src-tauri/src/sim.rs`.
