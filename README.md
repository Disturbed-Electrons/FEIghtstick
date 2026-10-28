# FEI Titan Control Panels as a Virtual Gamepad

Rust driver for FEI Titan control panels connected through the HPC2 USB adapter. FEIghtstick reads either or both panels and exposes a virtual gamepad. Applications can also use the Rust library to access all 36 controls directly.

## Installation

| Platform | Virtual gamepad | Requirements |
| --- | --- | --- |
| Linux | Generic gamepad through `uinput` | Device permissions; installer included |
| Windows 10/11 x64 | Xbox 360 / XInput through ViGEmBus | Signed ViGEmBus driver; installer included |
| macOS | Generic HID gamepad through `IOHIDUserDevice` | Apple-approved [virtual-device entitlement](https://developer.apple.com/documentation/bundleresources/entitlements/com.apple.developer.hid.virtual.device) and provisioned signature |

The build selects the platform backend automatically. Panel input and the JSON monitor are available on all three platforms. Windows uses the last signed release of [ViGEmBus](https://github.com/nefarius/ViGEmBus), whose upstream project is retired. Apple approval for macOS virtual-gamepad distribution is pending. macOS applications must support generic HID gamepads.

To build locally, install stable [Rust](https://rustup.rs/) and the platform's build tools: Xcode Command Line Tools, Visual Studio C++ Build Tools, or a Linux C toolchain.

```sh
cargo build --release
```

The executable is written to `target/release/feightstick` (`feightstick.exe` on Windows). The installers accept both local builds and unpacked build archives, and preserve existing profiles on reinstall. Re-running an installer updates the installation and restarts FEIghtstick. Keep the joystick released during installation, restart, and connection.

- **Linux (systemd):** run `sudo sh packaging/linux/install.sh`. The installer creates an unprivileged service account, installs device permissions, stores the profile at `/etc/feightstick/profile.toml`, and starts the service. It starts automatically at boot. View logs with `journalctl -u feightstick -f`. Stop it with `sudo systemctl stop feightstick` before running an interactive monitor. Desktop access to the panels uses the active session's `uaccess` permissions after replugging.
- **Windows:** open PowerShell as Administrator under the account that will use the panels, and run `powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\packaging\windows\install.ps1`. The installer installs the signed ViGEmBus prerequisite if needed and starts FEIghtstick in the background. A scheduled task starts it automatically when that account signs in. The Start menu shortcut can start the same task manually. Logs are written to `%ProgramData%\FEIghtstick\feightstick.log`. A ViGEmBus installation may require a Windows restart.
- **macOS:** for a signed bundle, place `FEIghtstick.app` at the archive root and launch `packaging/macos/Start FEIghtstick.command`. It opens an interactive terminal.

### Uninstall

On Linux, run `sudo sh /usr/local/libexec/feightstick/uninstall.sh`. Add `--purge` to remove the saved profile. The uninstaller stops and disables the service, removes the executable and device rules, and removes the service account.

On Windows, remove **FEIghtstick** through Installed Apps, or run `powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\packaging\windows\uninstall.ps1` from an elevated PowerShell. The uninstaller stops the bridge, removes its startup task, executable, and shortcuts, and preserves profiles and logs. Add `-Purge` to remove those too. ViGEmBus stays installed because other applications may use it.

## Usage

```sh
feightstick devices   # List connected panels and their HID paths
feightstick monitor   # Print decoded reports as JSON lines
feightstick run       # Run the virtual gamepad
```

These commands assume the executable is on PATH. From a source checkout, use `cargo run --release -- devices`, `cargo run --release -- monitor`, or `cargo run --release -- run`. The Windows installer leaves the executable in `%ProgramFiles%\FEIghtstick`; in PowerShell, invoke it with `& "$env:ProgramFiles\FEIghtstick\feightstick.exe" devices`. Running the executable without arguments starts the gamepad with the default profile. Avoid running a second gamepad bridge alongside the installed background instance.

In an interactive `run` or `monitor` session, let the joystick return to center and type **c**, then Enter. Type **q**, then Enter, or press Ctrl-C to stop. The installed background process uses `run --center-on-connect`, which assumes the joystick is centered at each connection. After a report sequence gap, center it again in the interactive session, or release it and reconnect its USB adapter when using the background process.

Panels reconnect automatically. Held buttons and joystick position persist between reports. Disconnecting a panel clears its mapped outputs. Shutdown clears the gamepad state and removes the virtual device. A forced process termination can leave a ViGEmBus controller attached.

When several panels of the same side are attached, use the paths printed by `devices` to select them with `--left-path PATH` and `--right-path PATH`. These options apply to `run` and `monitor`. Diagnostics go to stderr; monitor stdout contains one JSON object per report.

## Mapping

The default profile assigns these gamepad controls:

| Panel controls | Gamepad controls |
| --- | --- |
| L1 / L2 / L3 / Exposure | A / B / X / Y |
| Trackball − / + | Left / right bumper |
| R1 / R2 / R3 | Back / Start / Guide |
| Stigmator / Eucentric | Left / right thumb button |
| Dark field / Diffraction / Stage − / + | D-pad up / down / left / right |
| Right joystick | Left stick |
| Alpha / beta tilt | Right stick X / Y |
| Z down / up | Left / right trigger |

All other controls remain available through the library and monitor, and can be assigned in a custom profile.

```sh
feightstick profile > my-profile.toml
feightstick check-profile my-profile.toml
feightstick run --profile my-profile.toml
```

Each binding assigns a panel control to a gamepad target:

```toml
motion_hold_ms = 120

[[bindings]]
control = "left.intensity"
target = "right-x"
scale = 2.0

[[bindings]]
control = "right.focus"
target = "a"
scale = 16.0
threshold = 0.5
```

Input values are normalized as button 0/1, pressure / 15, joystick / 90, or relative increment / 16, then multiplied by `scale`. Negative scale reverses direction. Axis bindings add together and clamp to the output range. A button is pressed while any of its bindings reaches `threshold` (default 0.5).

Relative inputs hold the latest nonzero increment for `motion_hold_ms`, then return to neutral. Successive encoder ticks extend that hold. The example maps clockwise intensity movement to the right stick and positive focus movement to an A press.

Available targets: `a`, `b`, `x`, `y`, `left-bumper`, `right-bumper`, `back`, `start`, `guide`, `left-thumb`, `right-thumb`, `dpad-up`, `dpad-down`, `dpad-left`, `dpad-right`, `left-x`, `left-y`, `right-x`, `right-y`, `left-trigger`, `right-trigger`.

## Protocol

The adapter uses USB HID with vendor ID `0x0f3f`, product ID `0x0100` for the left panel and `0x0101` for the right panel, vendor usage page `0xff00`, and usage 1. The product ID identifies the panel side.

Each input report contains 20 payload bytes. Byte 0 is part of the payload; the HID report ID is separate. Byte 1 is a sequence counter that wraps from 255 to 0. Unknown bytes are ignored. LED output is currently unimplemented.

The table uses zero-based byte offsets and hexadecimal bit masks. Digital bits can be combined. Motion bytes are decoded as signed two's-complement `i8` values and widened before direction normalization. Decoded button readings expose masked bits in `raw` and the pressed state (0/1) in `value`.

| Side | Byte | Interpretation / control IDs |
| --- | --- | --- |
| Left | 2 | Bits `01/02/04/08/10/20/40/80`: `beamSpeedDown`, `beamSpeedUp`, `L1`, `L2`, `L3`, `mfFine`, `mfCoarse`, `exposure` |
| Left | 3 | Bits `01/02/04`: `stigmator`, `intensityCoarse`, `intensityFine` |
| Left | 4 / 5 | Unsigned pressure: `betaUp` / `betaDown` |
| Left | 6 / 7 | Unsigned pressure: `alphaRight` / `alphaLeft` |
| Left | 12 / 13 | Signed trackball deltas: `beamShift.X` / `beamShift.Y`; Y sign is reversed |
| Left | 14 / 17 | Signed encoder deltas: `intensity` / `multifunctionX` |
| Right | 2 | Bits `01/02/04/08/10/20/40/80`: `stageSpeedDown`, `stageSpeedUp`, `darkField`, `diffraction`, `wobbler`, `eucentric`, `R1`, `R2` |
| Right | 3 | Bit `01`: `R3` |
| Right | 4 / 5 | Unsigned pressure: `stageZDown` / `stageZUp` |
| Right | 12 / 13 | Signed joystick deflection deltas: `stagePosition.X` / `stagePosition.Y`; Y sign is reversed |
| Right | 14 / 15 | Signed encoder deltas: `focus` / `focusStep`; focusStep sign is reversed |
| Right | 16 / 17 | Signed encoder deltas: `magnification` / `multifunctionY` |

Prefix each control ID with `left.` or `right.` according to its side. Trackball and joystick values use right/up positive; encoders use clockwise positive. Pressure readings have an observed reference maximum of 15 and represent uncalibrated levels. The decoder preserves larger readings.

The joystick reports changes in deflection. After centering, the decoder integrates these deltas to estimate position. A zero delta leaves the position unchanged. The observed reference excursion is approximately ±90. Joystick values are `null` before centering and after a sequence discontinuity, including duplicate reports. Buttons and pressure are read directly from each report. Reconnecting starts a fresh decoder.

For offline decoding, `feightstick decode --side left` reads JSON arrays of 20 byte values from stdin and emits decoded frames. Use `--side right --centered` for a right-panel stream with a known centered starting position.

## Development

The project contains one library and one executable:

- `protocol.rs`: hardware mappings and joystick decoding.
- `device.rs`: HID enumeration and nonexclusive reads through `hidapi`.
- `mapping.rs`: TOML profiles and gamepad state.
- `output/`: Linux, Windows, and macOS backends.
- `main.rs`: CLI, polling, reconnects, and centering.

Applications can use `Decoder`, `Frame`, and `Control` directly. Protocol decoding operates independently of USB transport and gamepad output. Native macOS FFI is contained in `output/macos.rs`.

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
```

### macOS signing and distribution

For macOS distribution, build locally and run `packaging/macos/bundle.sh` with `SIGNING_IDENTITY`, `BUNDLE_ID`, and `PROVISIONING_PROFILE` set to matching Apple-approved values. Notarize and staple the resulting bundle, then place it at the release archive root.

## License

Copyright © 2026 [Hüseyin Çelik](https://www.github.com/hueseyincelik) and Tolga Wagner.

This project is licensed under [AGPL v3](/LICENSE).
