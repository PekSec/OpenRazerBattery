# AGENTS.md

# razer-bat

A lightweight Windows tray utility for displaying wireless Razer mouse battery status without running Razer Synapse.

## Goal

Build a small native Rust application that:

1. Detects supported Razer HID mouse or dongle devices.
2. Reads battery percentage through HID feature reports.
3. Shows the battery percentage in the Windows system tray.
4. Runs locally with no telemetry, no network access, no account system, and no Synapse dependency.

## Design Principle

This project must stay boring, small, and native.

Prefer:

- Single binary.
- Minimal dependencies.
- No web UI.
- No Electron.
- No Tauri.
- No .NET runtime.
- No async runtime unless proven necessary.
- No installer for v0.1.
- No configuration file until the battery path works.
- No write-capable HID commands in the first version.

## Target Platform

```text
OS: Windows 10 / Windows 11
Language: Rust
Target: x86_64-pc-windows-msvc
Distribution: single portable .exe
```

## Binary Modes

The same executable should support both diagnostic and tray modes.

```powershell
razer-bat.exe probe
razer-bat.exe tray
razer-bat.exe list
```

Default behavior:

```powershell
razer-bat.exe
```

Should start tray mode.

## Non-Goals

Do not implement these in v0.1:

1. DPI control.
2. Polling-rate control.
3. Lift-off distance control.
4. RGB control.
5. Macro support.
6. Button remapping.
7. Firmware updates.
8. Profile writes.
9. Synapse import/export.
10. Auto-update.
11. Cloud sync.
12. Telemetry.
13. Background service.
14. Kernel driver.
15. Admin-required installation.

## Minimal Project Structure

```text
razer-bat/
├── AGENTS.md
├── README.md
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── cli.rs
│   ├── device.rs
│   ├── hid.rs
│   ├── protocol.rs
│   ├── battery.rs
│   ├── tray.rs
│   └── error.rs
└── tests/
    └── protocol_tests.rs
```

Do not split into a workspace until there is a real reason.

## Dependency Policy

Start with as few dependencies as possible.

Recommended initial dependencies:

```toml
[dependencies]
hidapi = { version = "2", default-features = false, features = ["windows-native"] }
windows = { version = "0.62", features = [
    "Win32_Foundation",
    "Win32_UI_Shell",
    "Win32_UI_WindowsAndMessaging",
    "Win32_Graphics_Gdi",
    "Win32_System_LibraryLoader"
] }
```

Optional later dependencies:

```toml
[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

Avoid in v0.1:

```text
tokio
tauri
wry
egui
iced
slint
native-windows-gui
tray-icon
auto-launch
reqwest
any telemetry/crash-reporting SDK
```

A tray wrapper crate may be considered later, but only if raw Win32 tray code becomes too noisy.

## Architecture

Keep the architecture flat.

```text
main.rs
  ↓
cli.rs / tray.rs
  ↓
battery.rs
  ↓
device.rs + hid.rs + protocol.rs
```

### Module Responsibilities

## main.rs

Responsible for:

1. Parsing the first command argument.
2. Dispatching to `list`, `probe`, or `tray`.
3. Returning a process exit code.

Must not contain HID protocol logic.

## cli.rs

Responsible for:

1. Human-readable diagnostic output.
2. Listing candidate devices.
3. Running one-shot battery probe.
4. Printing errors clearly.

## tray.rs

Responsible for:

1. Creating a hidden message window.
2. Registering a Windows tray icon.
3. Updating tray tooltip.
4. Handling right-click menu.
5. Polling battery on a simple timer.
6. Exiting cleanly.

Must not construct HID reports directly.

## device.rs

Responsible for:

1. Razer VID filtering.
2. Known device catalog.
3. Device selection.
4. Usage page / usage matching.
5. Avoiding non-mouse Razer devices.

Razer vendor ID:

```text
0x1532
```

## hid.rs

Responsible for:

1. Enumerating HID devices.
2. Opening the selected HID path.
3. Sending feature reports.
4. Receiving feature reports.
5. Translating transport errors.

Must not know Razer command semantics beyond report length.

## protocol.rs

Responsible for:

1. Razer report construction.
2. Razer report parsing.
3. Checksum calculation.
4. Command class validation.
5. Command ID validation.
6. Battery byte extraction.

This module must be heavily unit-tested.

## battery.rs

Responsible for:

1. Calling protocol operations.
2. Returning a normalized battery snapshot.
3. Converting raw battery value to percentage.
4. Mapping protocol errors into user-facing state.

## error.rs

Responsible for:

1. Project-level error enum.
2. Short user-facing error messages.
3. Debug-friendly internal messages.

## Data Types

```rust
pub struct BatterySnapshot {
    pub device_name: String,
    pub vid: u16,
    pub pid: u16,
    pub percentage: Option<u8>,
    pub charging: Option<bool>,
    pub status: BatteryStatus,
}
```

```rust
pub enum BatteryStatus {
    Ok,
    DeviceNotFound,
    UnsupportedDevice,
    DeviceBusy,
    ProtocolError,
    TransportError,
}
```

```rust
pub struct RazerDeviceDefinition {
    pub vid: u16,
    pub pid: u16,
    pub name: &'static str,
    pub transaction_id: u8,
    pub usage_page: Option<u16>,
    pub usage: Option<u16>,
    pub supports_battery: bool,
    pub supports_charging: bool,
}
```

## Supported Device Catalog

All known devices must be declared in one place.

Example:

```rust
pub const RAZER_VID: u16 = 0x1532;

pub const DEVICES: &[RazerDeviceDefinition] = &[
    RazerDeviceDefinition {
        vid: RAZER_VID,
        pid: 0x00B7,
        name: "Razer DeathAdder V3 Pro Wireless",
        transaction_id: 0x1F,
        usage_page: Some(0x0001),
        usage: Some(0x0002),
        supports_battery: true,
        supports_charging: true,
    },
];
```

Rules:

1. Do not scatter PIDs across the codebase.
2. Do not guess support silently.
3. Unknown Razer devices may be listed, but battery probe should say unsupported.
4. New device support requires diagnostic output.

## HID Protocol Rules

Use read-like feature-report commands only in v0.1.

Do not implement persistent write commands.

Initial report constants:

```rust
pub const RAZER_REPORT_LEN: usize = 90;
pub const WINDOWS_FEATURE_REPORT_LEN: usize = 91;
pub const REPORT_ID: u8 = 0x00;

pub const CMD_CLASS_POWER: u8 = 0x07;
pub const CMD_GET_BATTERY: u8 = 0x80;
pub const CMD_GET_CHARGING: u8 = 0x84;
```

Checksum:

```text
XOR bytes 2 through 87 of the 90-byte Razer report.
Store result at byte 88.
```

Battery conversion:

```text
percentage = round(raw / 255 * 100)
```

Clamp output to:

```text
0..100
```

If a device family needs a different formula, add that behavior to the device catalog rather than adding random conditionals.

## CLI Behavior

### list

```powershell
razer-bat.exe list
```

Expected output:

```text
Razer HID candidates:

[1]
Name: Razer DeathAdder V3 Pro
VID: 0x1532
PID: 0x00B7
UsagePage: 0x0001
Usage: 0x0002
Known: yes
```

### probe

```powershell
razer-bat.exe probe
```

Expected success:

```text
Device: Razer DeathAdder V3 Pro
Battery: 87%
Charging: no
```

Expected unsupported:

```text
No supported Razer battery device found.
Run `razer-bat.exe list` to show detected Razer HID devices.
```

## Tray Behavior

The tray app should:

1. Start silently.
2. Poll immediately.
3. Poll every 60 seconds.
4. Show percentage in tooltip.
5. Show simple text icon if possible.
6. Provide right-click menu.
7. Exit cleanly.

Recommended tooltip:

```text
Razer DeathAdder V3 Pro: 87%
```

If charging state is available:

```text
Razer DeathAdder V3 Pro: 87% - charging
```

Recommended tray menu:

```text
Razer Battery
87%
Refresh
Exit
```

Do not create a visible main window.

## Polling Rules

Default interval:

```text
60 seconds
```

Rules:

1. Manual refresh bypasses interval.
2. Never busy-loop on failure.
3. On failure, keep the tray app alive.
4. On device disconnect, show unavailable state.
5. On reconnect, recover automatically.

Backoff:

```text
1st failure: 60 seconds
2nd failure: 120 seconds
3rd+ failure: 300 seconds
```

## Privacy and Security Rules

The application must:

1. Make zero network requests.
2. Include no telemetry.
3. Include no analytics.
4. Include no crash upload.
5. Avoid logging raw HID paths by default.
6. Avoid logging serial numbers.
7. Avoid admin privileges.
8. Avoid services.
9. Avoid drivers.
10. Avoid hooks.
11. Avoid input capture.

## Error Handling

Use small explicit errors.

```rust
pub enum AppError {
    NoDevice,
    UnsupportedDevice,
    AccessDenied,
    DeviceBusy,
    InvalidReportLength,
    InvalidChecksum,
    UnexpectedResponse,
    HidTransport,
}
```

User-facing messages should be short:

```text
No supported Razer mouse found.
Battery query failed.
Device busy. Will retry.
Unsupported Razer device.
```

Debug detail may be printed only in CLI mode with a future `--verbose` flag.

## Logging

No persistent log file in v0.1.

Use stderr for CLI diagnostics.

Tray mode should avoid disk writes.

A future version may add local logs behind a diagnostic flag.

## Build Commands

Create project:

```powershell
cargo new razer-bat --bin
cd razer-bat
```

Build debug:

```powershell
cargo build
```

Run list:

```powershell
cargo run -- list
```

Run probe:

```powershell
cargo run -- probe
```

Run tray:

```powershell
cargo run -- tray
```

Release build:

```powershell
cargo build --release
```

Release binary:

```text
target\release\razer-bat.exe
```

Optional size-focused release profile:

```toml
[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
strip = true
panic = "abort"
```

## Testing Strategy

Required unit tests:

1. Report length is correct.
2. Command class is placed correctly.
3. Command ID is placed correctly.
4. Checksum is calculated correctly.
5. Parser rejects invalid length.
6. Parser rejects invalid checksum.
7. Parser rejects wrong command class.
8. Parser rejects wrong command ID.
9. Battery conversion clamps to 0..100.
10. Device selector prefers known supported devices.

Hardware tests are manual.

Do not require physical Razer hardware in automated tests.

## Implementation Order

Implement in this order:

1. `cargo new razer-bat --bin`
2. Add `hidapi`.
3. Implement `razer-bat.exe list`.
4. Print detected Razer HID candidates.
5. Add known-device catalog.
6. Implement report checksum.
7. Implement battery report builder.
8. Implement response parser.
9. Implement `razer-bat.exe probe`.
10. Confirm battery read works.
11. Add tray code.
12. Add 60-second polling.
13. Add refresh and exit menu.
14. Add release profile.
15. Write README.

Do not start with the tray UI.

The first real milestone is:

```powershell
razer-bat.exe probe
```

returning a correct battery percentage.

## v0.1 Definition of Done

v0.1 is complete when:

1. `razer-bat.exe list` shows Razer HID candidates.
2. `razer-bat.exe probe` reads battery from at least one supported mouse.
3. `razer-bat.exe tray` shows battery in the Windows tray.
4. The app runs without Synapse.
5. The app performs no network activity.
6. The app requires no admin rights.
7. The app is a single portable executable.
8. Device disconnect does not crash the app.
9. Device reconnect recovers automatically.
10. Unit tests pass.

## Agent Rules

When working on this project:

1. Keep the binary small.
2. Keep dependency count low.
3. Do not add GUI frameworks.
4. Do not add async runtimes.
5. Do not add telemetry.
6. Do not add network access.
7. Do not add write-capable HID commands.
8. Do not require admin rights.
9. Do not add config before probe works.
10. Do not add logging before probe works.
11. Do not split into multiple crates before probe works.
12. Do not optimize the tray before the CLI probe works.
13. Prefer explicit code over abstractions.
14. Keep unsafe Win32 code isolated in `tray.rs`.
15. Keep HID protocol code isolated in `protocol.rs`.

## Future Features

Only after v0.1:

1. Low-battery notification.
2. Start with Windows.
3. Multiple-device selection.
4. Portable config file.
5. Optional log file.
6. Optional signed binary.
7. Optional device support database.
8. Optional charging animation.
9. Optional Windows toast notification.
10. Optional tiny settings window.

Anything that changes mouse settings must be treated as a separate project phase.
