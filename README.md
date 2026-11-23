# Open Haptic Mouse

Mouse abierto con botones de sensado inductivo y clic háptico. El botón detecta su recorrido sin contacto mecánico, permite punto de actuación y Rapid Trigger configurables, y responde con una vibración (LRA) en cada clic.

## Qué es

- **Sensado:** bobina PCB + convertidor inductivo (LDC1101) que mide la posición del botón.
- **Háptica:** actuador LRA controlado por un DRV2605L, disparado por hardware para minimizar la latencia.
- **MCU:** STM32G474RE en Rust (Embassy), USB Full-Speed a 1 kHz.
- **USB:** dispositivo compuesto con un mouse HID estándar y una interfaz HID vendor-defined para configuración y telemetría.
- **App de escritorio:** Tauri 2 + React para editar perfiles, calibrar y ver la señal en vivo.

Hoy todo corre contra un sensor y un actuador simulados. El hardware real se integra implementando los traits `SensorInterface` y `HapticDriver`, sin tocar la lógica.

## Estructura

| Carpeta | Contenido |
|---|---|
| `ohm-core` | Lógica `no_std`: botón, calibración, filtro, Rapid Trigger, pipeline del dispositivo. |
| `ohm-simulator` | Sensor y háptico simulados, escenarios JSON, modelo del LRA. |
| `ohm-protocol` | Reports HID vendor-defined y descriptores (ver `ohm-protocol/PROTOCOL.md`). |
| `ohm-firmware` | Firmware para STM32G474RE. Crate independiente. |
| `ohm-desktop` | App de escritorio (Tauri + React). |

## Requisitos

- Rust estable y el target `thumbv7em-none-eabihf`
- Windows: Microsoft C++ Build Tools
- Node.js y pnpm
- Para flashear: `probe-rs-tools`, `flip-link` y un ST-LINK

## Uso

```sh
# tests de la lógica
cargo test --workspace

# simulador por terminal
cargo run -p ohm-simulator -- ohm-simulator/scenarios/double_press_fast.json

# app de escritorio
cd ohm-desktop && pnpm install && pnpm tauri dev

# firmware (compila para el STM32)
cd ohm-firmware && cargo build --release
```

Las latencias que muestran el simulador y la app son modeladas, no medidas en hardware.
