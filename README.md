# STM32 / ADXL345 Aircraft Attitude Lab

Bevy **0.20.1** + serialport **4.10.1**, Rust 2024. A local 3D airliner, live artificial horizon, UART telemetry, five camera modes and a keyboard DEMO. The existing STM32 firmware and `ROLL:-12.3,PITCH:5.4` protocol remain unchanged.

## Run on macOS / Apple Silicon

```bash
cargo run --release -- --demo

ls /dev/cu.*
cargo run --release -- /dev/cu.usbmodemXXXX
```

Run via `cargo run` from the project directory with `assets/` present. Cargo sets the asset root; launching the binary directly requires setting `BEVY_ASSET_ROOT` to the project directory (or placing `assets/` beside the binary). Close other programs using the same serial port. Serial settings are **115200, 8 data bits, no parity, 1 stop bit**. No serial port is opened in DEMO, and all simulated values are explicitly labelled DEMO.

```bash
cargo run --release -- /dev/cu.usbmodemXXXX --invert-roll --pitch-offset -2.5
cargo run --release -- --demo --response 12
```

Options: `--invert-roll`, `--invert-pitch`, `--roll-offset N`, `--pitch-offset N`, `--response N` (positive response in inverse seconds; default 12). Offsets are additive degrees after sign inversion. Settings changed with keys last for the current session; use command-line options to reproduce them.

## Controls

| Keys / mouse | Action |
| --- | --- |
| Left / Right | DEMO roll − / + |
| Up / Down | DEMO pitch + / − (limited to ±85°) |
| Space | DEMO raw attitude returns to zero |
| 1 / 2 / 3 / 4 / 5 | Perspective / Top / Front / Side / Free orbit |
| Tab | Next camera; preserves attitude |
| Right mouse drag / wheel | Rotate / zoom in Free orbit |
| F11 / Escape | Toggle fullscreen / return to window |
| I / K | Invert Roll / Pitch |
| Z | Zero calibration at the latest raw sample |
| C | Clear both offsets |
| [ / ] | Roll offset − / +1° |
| − / = | Pitch offset − / +1° |
| Hold left Shift | Offset steps become 0.1° |

Centering raw DEMO input with Space preserves mounting settings. Z makes the current sensor orientation the calibrated zero. The UI separates raw input, calibrated accepted target and smoothed displayed angles.

## Coordinates and instrument behavior

The body frame is **+X right wing, +Y up, −Z nose**. Positive roll lowers the right wing; positive pitch raises the nose. Yaw has no sensor input and the target heading is zero. `sensor_to_body` applies sign and calibration; `body_to_bevy` composes `q_pitch(+X) * q_roll(−Z)`. Quaternion multiplication applies roll about the pitched longitudinal axis. Reversing the multiplication changes combined attitudes; tests check the expected wing and nose vectors.

The original GLB has nose +X and up +Y. Its child correction rotates it +90° about Y, scales real-world meters to the scene, and moves the fuselage axis to the origin. The independent parent receives only the measured attitude quaternion. Camera rotations never modify it. The fixed colored world axes and ground grid are reference cues.

The horizon presents the outside world as seen by the aircraft: positive bank rotates sky/ground counterclockwise; nose-up moves the horizon downward along the banked local vertical. The yellow aircraft symbol stays fixed, with 10° pitch increments and a bank scale. Both instruments use the same displayed quaternion. Angles are decomposed in YXZ order; exactly ±90° pitch is an Euler singularity, so bank readout there is ambiguous even though quaternion rendering stays continuous.

**ADXL345 measures acceleration.** Roll/Pitch are gravity-based estimates, not complete inertial attitude. Linear acceleration, vibration and turns can make the estimated gravity direction differ from actual attitude. There is no yaw measurement. This is a visualization/reference instrument, not certified flight equipment.

## Data flow and resilience

- A separate serial OS thread reads fixed 512-byte chunks with a 150ms timeout. It reconnects one second after open/read failure or EOF; the render thread never waits for serial I/O.
- The line decoder keeps fragments across timeouts. Lines over 256 bytes are discarded through the next newline. Invalid UTF-8, incomplete fields, duplicate/unknown fields, malformed floats, NaN and infinity are ignored safely.
- A bounded `mpsc::sync_channel(1)` notifies Bevy of a shared latest-sample mailbox. Notifications coalesce, so fast streams cannot accumulate a backlog; the UI receives the latest raw sample and counters. Intermediate samples can be skipped intentionally.
- CONNECTED (green) requires an open port **and a valid packet within two seconds**, measured at receipt in the serial thread. Open without fresh data is WAITING (yellow); open/read failure is DISCONNECTED (red), including the actual error. DEMO is blue. A stale feed holds its last attitude; rate reads zero after the timeout.
- Raw data remains visible without clamping or rewriting. Rendering rejects targets outside roll ±180° / pitch ±90°. A jump exceeding 60° is held until three consistent rendering updates confirm it (within 10°); HELD counts rejected/held updates. This can briefly delay a real abrupt maneuver. Parser-invalid packets have a separate counter. UART freshness refers to syntactically valid packets; the raw/target difference and HELD counter expose range rejection.
- Accepted targets are mapped to a quaternion. The displayed orientation uses `slerp` with `1 − exp(−response × dt)`, giving a similar response across frame rates. Default time constant is about 83ms. Filtering changes the display, not raw measurements.
- Missing or failed GLB dependencies produce a visible MODEL ERROR; restore the bundled `assets/models/aircraft.glb` and restart. There are no runtime model downloads.

## Modules

```text
src/main.rs                 App and ordered systems
src/config.rs               CLI and mounting settings
src/serial/{mod,parser}.rs   UART worker, bounded mailbox, packet decoder
src/aircraft/{mod,attitude}.rs  GLB rendering, axes, calibration, filtering
src/camera/mod.rs           Views, orbit, fullscreen
src/ui/{mod,horizon}.rs     Telemetry panels and artificial horizon
src/demo/mod.rs             Keyboard synthetic input
src/validation.rs           Explicit GPU smoke playback
assets/models/aircraft.glb  Bundled CC0 airliner (see ASSETS.md)
```

## Verification

```bash
cargo fmt --check
cargo check
cargo test
cargo run -- --demo
# Optional repeatable GPU playback; exits after ten cases (at least ~23s after model loading):
RUST_LOG=info cargo run --release -- --demo --smoke-test
```

The smoke run checks GLB dependency loading and convergence of ±30° roll, ±20° pitch and combined attitudes; it visits all five cameras and resizes to 1000×700. Slow initial GPU frames are allowed up to 45 seconds per attitude to converge. Screenshots are written to `target/validation/demo-00.png` through `demo-09.png`. It needs a desktop session and GPU; inspect the images for visual correctness. It does not simulate real keyboard/mouse/OS fullscreen events.

Parser, bounded framing, status timeout, mounting correction, quaternion axes/order, horizon direction, outlier handling and frame-rate independence have automated tests.

An optional UNIX PTY integration test requires `python3` and terminal access:

```bash
cargo test uart_partial_stale_disconnect_reconnect_and_latest_mailbox -- --ignored --nocapture
```

It exercises the same serial worker with fragmented packets, invalid and oversized lines, a 200-packet burst without notification draining, timeout, disconnection and reconnection. macOS PTYs do not implement the baud-rate IOCTL, so only this private test entry uses baud 0; production remains 115200. Physical USB unplug/replug and real STM32 behavior still require the connected hardware. See `VALIDATION.md` for the commands and results actually obtained in this workspace.
