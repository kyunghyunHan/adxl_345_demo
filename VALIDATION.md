# Validation record

Date: 2026-11-16 (Asia/Seoul). Host: Apple M1, macOS 26.6.2, Metal.

## Automated checks

- `cargo fmt --check`: passed.
- `cargo test --offline`: 10 passed; the optional PTY test is ignored by default.
- `cargo test --offline uart_partial_stale_disconnect_reconnect_and_latest_mailbox -- --ignored --nocapture`: passed (1 test). Verified fragmented packets, malformed and oversized lines, a 200-packet burst, stale-data status, disconnect and reconnect through the production serial worker using a UNIX PTY. This does not verify a physical USB device or its baud rate.

## GPU playback

Run from the project directory:

```sh
RUST_LOG=info cargo run --offline -- --demo --smoke-test
```

Desktop/GPU access is required. The restricted sandbox could not discover a GPU; the desktop run found the Apple M1 Metal adapter. Use `cargo run` so Cargo sets the project asset root. Launching `target/debug/adxl345_bevy_gui` directly without an asset-root override looks for assets beside the executable and fails to load the model and font.

The initial playback exposed a validation timing issue: shader compilation/readback can consume wall time while Bevy clamps simulation delta. The check now waits for displayed roll/pitch to converge within 0.5 degrees, with a 45-second timeout per case, instead of failing immediately after two wall-clock seconds.

Playback passed all 10 cases: level, roll +/-30 degrees, pitch +/-20 degrees, combined attitudes, all five camera modes, and a resize to 1000 x 700 logical pixels. It exited successfully with `GPU SMOKE PASS`. Screenshots are saved as `target/validation/demo-00.png` through `demo-09.png` (Retina pixel dimensions are twice the logical dimensions).

Screenshot inspection also found unsupported Unicode minus glyphs in bank labels and control hints. These labels now use the supported ASCII minus character. The aircraft, model-ready state, readouts and instrument panels were visible in the captured desktop frames. This was a debug build with screenshot readback; release-mode frame-rate performance was not measured.

## Remaining manual verification

No STM32 USB serial port was present during this session; only Bluetooth/debug ports were listed. With the board connected, run `ls /dev/cu.*`, then `cargo run -- /dev/cu.usbmodemXXXX` using the actual device path.

- Confirm CONNECTED and plausible RAW angles at 115200 baud.
- Tilt the board along each axis; verify roll/pitch signs and zero calibration.
- Physically unplug/replug USB; confirm disconnect and automatic recovery.
- Check actual keyboard and mouse events: camera keys, free orbit, zoom, calibration and DEMO controls.
- Check F11 fullscreen and Escape return to window on the desktop.

The GPU playback changes camera state and window size programmatically; it does not substitute for these input/OS checks.

## Build note

The debug linker emitted an `__eh_frame section too large` compact-unwind warning. Linking and automated tests succeeded; the warning concerns exception-handling performance.
