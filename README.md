# Hadron

A native **Rust** USB client for [Goldleaf](https://github.com/XorTroll/Goldleaf) on Nintendo Switch — a from-scratch port of XorTroll's Java **Quark** client, built to run true-native on **Apple Silicon Macs** (and Linux / Windows).

> Hadrons are made of quarks. Hadron is Quark, rewritten in Rust.

## Why

Quark is a JavaFX + `usb4java` (libusb) app. The last `libusb4java` release (1.3.0, Oct 2018) ships **no `darwin-aarch64` native** — only `darwin-x86-64`. On an Apple-Silicon JVM that JNI dylib is the wrong architecture, so USB enumeration silently comes back empty and Quark simply doesn't see the Switch. The app builds and launches; the failure is invisible at the USB layer. (See upstream issues [#707](https://github.com/XorTroll/Goldleaf/issues/707) and [#753](https://github.com/XorTroll/Goldleaf/issues/753).)

The current workaround ([#753](https://github.com/XorTroll/Goldleaf/issues/753)) is to compile a darwin-aarch64 `libusb4java.dylib` yourself, inject it into `Quark.jar`, and install an Azul JDK with JavaFX. It works, but it's a Java-on-Rosetta story with a repackaged jar.

Hadron takes a different route: a single native binary, no Java runtime, no Rosetta, no libusb. USB is handled by [`nusb`](https://crates.io/crates/nusb) — pure Rust, native on Apple Silicon.

## Status

Protocol-compatible with Goldleaf Quark **v1.1.0** (the minimum device version the Java client enforces). All 17 Goldleaf commands are implemented:

`GetDriveCount`, `GetDriveInfo`, `StatPath`, `GetFileCount`, `GetFile`, `GetDirectoryCount`, `GetDirectory`, `StartFile`, `ReadFile`, `WriteFile`, `EndFile`, `Create`, `Delete`, `Rename`, `GetSpecialPathCount`, `GetSpecialPath`, `SelectFile`.

The UI mirrors Quark's: a log panel, a special-paths list (add/remove), and a USB status bar, with native OS file/folder pickers via [`rfd`](https://crates.io/crates/rfd).

## Build & run

```sh
cargo run --release       # debug
cargo build --release     # binary at target/release/hadron
```

Then run Goldleaf on your Switch, choose *Remote PC (via USB)*, and launch Hadron. It will auto-detect the Switch (VID `0x057E`, PID `0x3000`) and start processing commands.

### Config

Special paths are stored at `~/.config/hadron/hadron-config.cfg` as `name=value` lines. The format is compatible with Quark's `quark-config.cfg`, so you can copy an existing Quark config over if migrating.

```
hadron --cfgfile /path/to/custom-config.cfg
```

## Platform support

| OS      | USB backend                | Status |
|---------|----------------------------|--------|
| macOS (arm64, Apple Silicon) | `nusb` (IOKit, pure Rust) | Primary target, tested |
| macOS (x86_64)               | `nusb`                   | Builds, untested |
| Linux                         | `nusb` (usbfs)           | Builds, untested |
| Windows                       | `nusb` (WinUSB)          | Builds, untested |

## Acknowledgements

Hadron is a derivative of **Quark** by [XorTroll](https://github.com/Xortroll), part of the [Goldleaf](https://github.com/XorTroll/Goldleaf) project. All credit for the protocol design and the original client goes to XorTroll. The `SelectFile` command and special-paths concept are unchanged from Quark.

Thanks also to [@snowflame0](https://github.com/snowflame0) whose [#753](https://github.com/XorTroll/Goldleaf/issues/753) dylib-injection workaround unblocked ARM-Mac users in the meantime and motivated this port.

## License

GPL-3.0-or-later, inherited from Goldleaf/Quark. See [`LICENSE`](LICENSE).