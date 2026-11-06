# FEI Titan Control Panels as a Virtual Gamepad

Rust driver for FEI Titan control panels connected through the HPC2 USB adapter. FEIghtstick reads either or both panels and exposes a virtual gamepad. The Rust library provides direct access to all 36 controls, individual LEDs, label illumination, and device information through USB HID.

## Installation

| Platform | Virtual gamepad | Requirements |
| --- | --- | --- |
| Linux | Generic gamepad through `uinput` | Device permissions; installer included |
| Windows 10/11 x64 | Xbox 360 / XInput through ViGEmBus | Signed ViGEmBus driver; installer included |
| macOS | Generic HID gamepad through `IOHIDUserDevice` | Apple-approved [virtual-device entitlement](https://developer.apple.com/documentation/bundleresources/entitlements/com.apple.developer.hid.virtual.device) and provisioned signature |

The build selects the platform backend automatically. Panel input and the JSON monitor are available on all three platforms. Windows uses the signed [ViGEmBus](https://github.com/nefarius/ViGEmBus) driver. macOS applications must support generic HID gamepads.

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

These commands assume the executable is on PATH. From a source checkout, use `cargo run --release -- <command>`. The Windows installer leaves the executable in `%ProgramFiles%\FEIghtstick`; in PowerShell, invoke it with `& "$env:ProgramFiles\FEIghtstick\feightstick.exe" devices`. Running the executable without arguments starts the gamepad with the default profile. Avoid running a second gamepad bridge alongside the installed background instance.

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

Each input report contains 20 payload bytes, starting at byte 0. The HID report ID precedes the payload in APIs that include it. Ordinary control reports have byte 0 equal to zero, and byte 1 is their sequence counter, wrapping from 255 to 0. Nonzero byte 0 identifies a command reply, routed to request handling. Unknown control-report bytes are ignored.

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

Prefix each control ID with `left.` or `right.` according to its side. Trackball and joystick values use right/up positive; encoders use clockwise positive. Pressure readings are unsigned levels, normalized against 15 for gamepad mapping. The decoder preserves the full byte.

The joystick reports changes in deflection. After centering, the decoder integrates these deltas to estimate position. A zero delta leaves the position unchanged. Gamepad mapping normalizes joystick position against ±90. Joystick values are `null` before centering and after a sequence discontinuity, including duplicate reports. Buttons and pressure are read directly from each report. Reconnecting starts a fresh decoder.

For offline decoding, `feightstick decode --side left` reads JSON arrays of 20 byte values from stdin and emits decoded frames. Use `--side right --centered` for a right-panel stream with a known centered starting position.

### LED commands and channels

The caller controls indicator LEDs and label illumination through the Rust library. Opening, reconnecting, and closing a panel leave both unchanged.

Commands `07` and `08` read or write one electrical channel. All offsets below refer to the 20-byte payload:

| Byte | Request | Reply |
| --- | --- | --- |
| 0 | Operation `07` / `08` | Matching operation |
| 1 | Zero | Status; zero means success |
| 2 | Channel index | Matching channel |
| 3 | Read: zero; write: `00` off / `01` on | Read: `00` off / `01` on |
| 4–18 | Zero | Ignored |
| 19 | Request sequence | Matching request sequence |

HIDAPI writes prepend a zero report-ID byte (21 bytes total); native macOS `IOHIDDeviceSetReport` takes the 20-byte payload with report ID zero. Replies arrive on the input endpoint.

The table lists electrical output channels in decimal:

| Panel | Indicator | Red channel | Green channel |
| --- | --- | --- | --- |
| Left | Coarse | 0 | — |
| Left | Fine | 8 | — |
| Left | L1 | 11 | 3 |
| Left | L2 | 10 | 2 |
| Left | L3 | 9 | 1 |
| Left | Exposure | 6 | — |
| Left | Stigmator | 14 | — |
| Right | R1 | 13 | 4 |
| Right | R2 | 12 | 3 |
| Right | R3 | 11 | 2 |
| Right | Wobbler | 5 | — |
| Right | Dark Field | 6 | — |
| Right | Diffraction | 14 | — |

Left channels **4, 5, 12, and 13** are accessible through the channel API; they acknowledge writes and read back off, with no visible light output. Left indexes 7 and 15 and right indexes 0, 1, 7, 8, 9, 10, and 15 are unassigned and rejected before writing. Use LED channel IDs to address the Coarse/Fine indicators.

`set_indicator_color` turns off the other color before enabling the requested one; `Color::Off` clears both channels. Green is rejected for red-only indicators. A color change sends up to two acknowledged writes in order; earlier writes remain applied if a later write fails. `Indicator::channels()` exposes the red and optional green indexes; there is no universal offset between them.

Command `09` reads all 16 LED channels in one exchange. Requests set bytes 1–18 to zero and byte 19 to the request sequence. Replies use the operation, status, and sequence fields above; bytes 2–17 form a 16-byte array (`00` off, `01` on), with channel `n` at byte `2 + n`. Byte 18 is ignored. Unassigned channels read off; label illumination is separate.

### Label illumination

Each panel has one on/off flag for its printed-label lighting. Indicator LEDs have their own channel settings.

Commands `05` (read) and `06` (write) use the LED command framing, with the flag in byte 2: `00` off or `01` on. Read requests set byte 2 to zero; all requests set byte 1 and bytes 3–18 to zero and carry the request sequence in byte 19. Replies use the same operation, status, and sequence fields as LED replies; reads return the flag in byte 2.

### Device information

Command `02` queries panel information. Requests set bytes 1–18 to zero and byte 19 to the request sequence. Replies use the common operation, status, and sequence fields, with information in bytes 2–18.

`PanelInformation::data` preserves all 17 information bytes, with `data[n]` corresponding to payload byte `2 + n`. `version_major()` reads payload byte 6 and `version_minor()` reads byte 5; the original driver's numeric version is `100 × major + minor`. All other fields remain raw: payload bytes 7 and 8 are available as `data[5]` and `data[6]`.

### Rust command API

`Panel::poll()` is the sole report reader. Pass `PanelEvent::Input` reports to `Decoder`; completion events carry request results, and `Reply` carries intermediate or unmatched command replies.

| Method | Completion event | Successful result |
| --- | --- | --- |
| `read_led(channel)` | `LedComplete` | `Some(bool)` |
| `read_led_states()` | `LedStatesComplete` | `[bool; 16]`, indexed by wire channel |
| `set_led(channel, enabled)` | `LedComplete` | `None` |
| `set_indicator_color(indicator, color)` | `LedComplete` | `None` |
| `read_illumination()` | `IlluminationComplete` | `Some(bool)` |
| `set_illumination(enabled)` | `IlluminationComplete` | `None` |
| `read_information()` | `InformationComplete` | `PanelInformation` |

Each method returns a `RequestId` scoped to the panel connection. Call `poll()` regularly, including while controls are idle, until a completion event arrives with that ID. Each panel handles one command at a time; another request returns `RequestError::Busy`. HID writes are synchronous. Polling reads available reports and services request deadlines.

Replies match the operation and byte-19 request sequence; single-channel LED replies also match the channel. The request sequence wraps independently of the input sequence. Writes complete on status zero; readback requires a separate read. Errors report nonzero status, non-Boolean lighting states, short writes, and timeouts. Status `02` indicates an unassigned LED channel. Each exchange has a 500 ms deadline (`COMMAND_REPLY_TIMEOUT`) and no automatic retries. After a HID read error or disconnect, discard the connection and pending operation, then reopen the panel when available.

## Development

The project contains one library and one executable:

- `protocol.rs` and `protocol/`: input decoding, LED mappings, lighting commands, and panel information.
- `device.rs`: HID enumeration, nonexclusive transport, report routing, and pending command exchanges through `hidapi`.
- `mapping.rs`: TOML profiles and gamepad state.
- `output/`: Linux, Windows, and macOS backends.
- `main.rs`: CLI, polling, reconnects, and centering.

Applications can decode report data directly through `Decoder`, `Frame`, and `Control`. Native macOS FFI is contained in `output/macos.rs`.

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
```

### macOS signing and distribution

For macOS distribution, build locally and run `packaging/macos/bundle.sh` with `SIGNING_IDENTITY`, `BUNDLE_ID`, and `PROVISIONING_PROFILE` set to matching Apple-approved values. Notarize and staple the resulting bundle, then place it at the release archive root.

## License

Copyright © 2026 [Hüseyin Çelik](https://www.github.com/hueseyincelik) and Tolga Wagner.

This project is licensed under [AGPL v3](/LICENSE).
