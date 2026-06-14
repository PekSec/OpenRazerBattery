# razer-bat

A lightweight Windows tray utility for displaying wireless Razer mouse battery
status without running Razer Synapse.

This repository is currently in early development. The crate uses Rust 2024
with a minimum supported Rust version of 1.85, builds as a single binary,
enumerates local Razer HID candidates, and can probe battery state for supported
OpenRazer-backed mouse entries. Tray integration is still to be implemented.

## Toolchain

```text
Rust edition: 2024
MSRV: 1.85
Target: x86_64-pc-windows-msvc
```

## Commands

```powershell
cargo build
cargo test
cargo run -- list
cargo run -- probe
cargo run -- tray
```

The final executable modes are planned as:

```powershell
razer-bat.exe list
razer-bat.exe probe
razer-bat.exe tray
```

Running without arguments will dispatch to `tray` mode.

## Current Status

- `list` enumerates Razer HID candidates, including HID interface numbers and
  usages, and matches known mouse PIDs from OpenRazer's device list.
- `probe` opens the supported HID interface and reads battery percentage.
- `tray` prints a placeholder tray-mode message.
- Protocol constants, checksum helpers, report construction, device catalog
  matching, and battery conversion are present and covered by tests.

## Device Data Source

Known Razer mouse VID/PID entries are derived from the OpenRazer supported
device list and mouse driver:

- https://openrazer.github.io/
- https://github.com/openrazer/openrazer

The app keeps this catalog local and only probes battery-capable devices marked
as supported in that catalog.

## Dependencies

- `hidapi` is used with default features disabled and `windows-native` enabled.
- `windows` is used with default features disabled and only the Win32 feature
  groups needed for the tray implementation enabled.
- No async runtime, GUI framework, telemetry, networking, or installer
  dependency is included.
