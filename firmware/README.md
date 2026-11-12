# Disled firmware

This is the firmware of the CH32V203G6U6 on the Disled board: lamp state
machine, custom colors and touch cycle set by the host, capacitive touch,
ambient light, LED output with a current cap, alerts, and version 2 of the
USB HID protocol. It is written in C11 on the WCH NoneOS SDK and TinyUSB,
and built with PlatformIO. It stores nothing in flash: the host sends its
colors and settings again at every connection.

## Quick start

```sh
git submodule update --init          # TinyUSB, once after cloning
pio run -e debug                     # build (debug: HID + CDC console)
pio run -e debug -t upload           # flash through the ROM bootloader
pio device monitor                   # debug console, type "help"
pio test -e native                   # unit tests on the build machine
pio check -e debug                   # static analysis (cppcheck)
```

The unit tests need a host C compiler (`gcc`). On Windows, a MinGW-w64 build
such as WinLibs works. CI runs them on every pull request into `main`.

The CI installs PlatformIO and clang-format from `pyproject.toml` with
[uv](https://docs.astral.sh/uv/). To use the same versions on the command
line, run `uv sync` once, then prefix the commands with `uv run`, as in
`uv run pio run -e debug`. The PlatformIO extension of VS Code works too.

[docs/flashing.md](../docs/flashing.md) covers the bootloader, the Windows
driver, the debug console and troubleshooting.

## Environments

| Environment | Content |
|---|---|
| `debug` (default) | HID protocol + CDC console and logs, USB ID `1209:0002` |
| `release` | HID protocol only, no logs, USB ID `1209:0001` |
| `debug_bl`, `release_bl` | The same, linked at `0x1000` behind the HID bootloader prototype ([bootloader/](bootloader/README.md)) |
| `native` | Unity tests of `lib/disled_core`, with the protocol vectors |

Both IDs are test PIDs of pid.codes. The release build moves to the lamp's
own, `1209:D1ED`, once pid.codes grants it, and the debug build keeps
`1209:0002` ([LICENSING.md](../LICENSING.md#usb-identifiers)). `debug` and
`release` start at 0x0 and may fill the 32 KB of flash: the ROM bootloader
of WCH, which writes them, lives outside it.

## Layout

| Path | Content |
|---|---|
| `src/main.c` | entry point |
| `src/app/` | wiring and tasks (`app.c`), host protocol (`app_host.c`), console commands, **all tunables** (`app_config.c`) |
| `src/bsp/` | pin map (`board.h`), clocks, time base, watchdog, faults, unique ID |
| `src/drivers/` | LED PWM, TouchKey, I2C, VEML7700 |
| `src/usb/` | TinyUSB configuration, descriptors, link state and report queues |
| `src/diag/` | logs and console (debug builds) |
| `lib/disled_core/` | hardware-independent logic, unit tested |
| `lib/tinyusb/` | TinyUSB 0.21.0 (submodule `upstream/`) and its build description |
| `test/` | Unity tests, one folder per core module |
| `scripts/` | version from git, memory budget, Intel HEX, protocol vectors to C, binary for an upload through the bootloader |
| `pyproject.toml`, `uv.lock` | build and check tools (PlatformIO, clang-format) for uv, kept up to date by Dependabot |
| `tools/touch_replay/` | replays touch recordings through the detector on the build machine |
| `bootloader/` | HID bootloader prototype, a PlatformIO project of its own |

[docs/architecture.md](../docs/architecture.md) explains the design, and
[docs/behavior.md](../docs/behavior.md) the lamp behavior.

## Conventions

- C11, in the GNU dialect for the SDK headers, formatted with clang-format
  23 (`.clang-format`). Warnings are errors on our sources.
- No dynamic allocation, no floating point, no 64-bit division
  ([why](../docs/architecture.md#safety-rules)).
- Interrupt handlers use `BSP_ISR` and only capture data.
- Includes are relative to `src/`, so each one names the layer it uses.
- The firmware version comes from the latest `fw-vX.Y.Z` tag.
