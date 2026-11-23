# Protocolo OHM host <-> dispositivo

Implementación: `ohm-protocol` (`no_std`, sin `alloc`). Transporte: reports HID vendor-defined (usage page `0xFF00`) de tamaño fijo. Todo entero es little-endian. El byte 0 de cada report es el Report ID. Longitudes incluyen ese byte.

## Report IDs

| ID     | Nombre       | Tipo HID | Dirección        | Longitud |
|--------|--------------|----------|------------------|----------|
| `0x01` | `SetProfile` | Feature  | host -> device   | 10       |
| `0x02` | `GetStatus`  | Feature  | device -> host   | 17       |
| `0x03` | `Telemetry`  | Input    | device -> host   | 16       |

## Layouts

### `SetProfile` (0x01), 10 bytes

| Offset | Tamaño | Campo                     | Notas                        |
|--------|--------|---------------------------|------------------------------|
| 0      | 1      | Report ID = `0x01`        |                              |
| 1      | 2      | `actuation`               | u16, 0..=1000                |
| 3      | 2      | `release_hysteresis`      | u16, 0..=1000, <= `actuation`|
| 5      | 4      | `rapid_trigger_window_us` | u32                          |
| 9      | 1      | `haptic_intensity`        | u8                           |

Los 9 bytes desde el offset 1 forman el bloque *Profile*, reutilizado por `GetStatus`.

### `GetStatus` (0x02), 17 bytes

| Offset | Tamaño | Campo                | Notas                                  |
|--------|--------|----------------------|----------------------------------------|
| 0      | 1      | Report ID = `0x02`   |                                        |
| 1      | 1      | `protocol_version`   | u8, `PROTOCOL_VERSION` del firmware    |
| 2      | 9      | Profile activo       | mismo bloque que `SetProfile` offsets 1..10 |
| 11     | 4      | `last_raw`           | u32, última lectura cruda del sensor (sin calibrar; cubre 24 bits) |
| 15     | 1      | `button`             | 0 = Up, 1 = Down                       |
| 16     | 1      | `last_event`         | código de evento (ver abajo)           |

### `Telemetry` (0x03), 16 bytes

| Offset | Tamaño | Campo        | Notas                                    |
|--------|--------|--------------|------------------------------------------|
| 0      | 1      | Report ID = `0x03` |                                    |
| 1      | 8      | `timestamp`  | u64, microsegundos                       |
| 9      | 2      | `position`   | u16, 0..=1000                            |
| 11     | 1      | `event`      | código de evento (ver abajo)             |
| 12     | 4      | `latency_us` | u32, detección -> envío del report, en µs|

### Código de evento

| Valor | Significado         |
|-------|---------------------|
| 0     | ninguno (`None`)    |
| 1     | `Event::Down`       |
| 2     | `Event::Up`         |
| 3     | `Event::RapidDown`  |

## Validación al decodificar

`Report::decode` devuelve error, nunca entra en pánico:

- buffer vacío o más corto que el report: `Truncated`.
- primer byte distinto de 1, 2, 3: `UnknownId`.
- valor de enum fuera de tabla (`button`, `event`): `BadEnum`.
- `actuation`, `release_hysteresis` o `position` > 1000: `PositionOutOfRange`.
- `release_hysteresis` > `actuation`: `InvalidProfile` (según `Profile::validate`).

Los bytes sobrantes tras la longitud del report (relleno HID) se ignoran. `Report::encode` rechaza buffers pequeños (`BufferTooSmall`) y perfiles inválidos (`InvalidProfile`), no escribe más allá de la longitud del report, y todo lo que codifica se decodifica al mismo valor.

## Descriptores HID

`MOUSE_REPORT_DESCRIPTOR`: ratón estilo boot, sin Report ID, 4 bytes `[botones (3 bits + 5 de relleno), X, Y, rueda]`, X/Y/rueda relativos de -127 a 127.

`VENDOR_REPORT_DESCRIPTOR`: usage page `0xFF00`, una colección Application, Feature `0x01` (9 bytes de datos), Feature `0x02` (16 bytes), Input `0x03` (15 bytes). Cada Report Count es la longitud de la tabla anterior menos el byte de ID; un test recorre los items del descriptor y lo verifica contra las constantes de longitud.

## Versionado

`PROTOCOL_VERSION` (actualmente `2`) viaja en `GetStatus`. El host lo lee antes de enviar `SetProfile`.

- Cambio incompatible (mover o redimensionar campos, cambiar longitud o códigos, quitar un report): sube `PROTOCOL_VERSION`. Host y firmware con versiones distintas no se comunican hasta actualizar.
- Cambio compatible: solo añadir un Report ID nuevo. No cambia la versión; un receptor que no lo conoce responde `UnknownId`.
- Añadir variantes a un enum (códigos de evento) es incompatible, porque un receptor antiguo rechaza el valor con `BadEnum`: sube la versión.
- `decode` no compara `protocol_version`; devuelve el valor y el host decide.

### Historial

- `2`: `GetStatus.last_raw` pasa de u16 a u32 (el LDC1101 devuelve 16 bits en RP y 24 en L); `GetStatus` pasa de 15 a 17 bytes. `SetProfile` y `Telemetry` no cambian.
