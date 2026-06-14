# razer-bat

A lightweight Windows tray utility for displaying wireless Razer mouse battery
status without running Razer Synapse.

This repository is currently at the initial Rust scaffold stage. The crate uses
Rust 2024 with a minimum supported Rust version of 1.85, builds as a single
binary, exposes the planned module layout, and includes hardware-free protocol
tests. Real HID enumeration, battery probing, and tray integration are still to
be implemented.

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

- `list` prints a placeholder diagnostic message.
- `probe` prints a placeholder battery-probe message.
- `tray` prints a placeholder tray-mode message.
- Protocol constants, checksum helpers, report construction, and battery
  conversion are present and covered by tests.

## Dependencies

- `hidapi` is used with default features disabled and `windows-native` enabled.
- `windows` is used with default features disabled and only the Win32 feature
  groups needed for the tray implementation enabled.
- No async runtime, GUI framework, telemetry, networking, or installer
  dependency is included.
