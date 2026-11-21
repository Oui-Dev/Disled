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
# static analysis (cppcheck) of both builds, failing on any defect as CI does
pio check -e debug -e release --fail-on-defect low --fail-on-defect medium --fail-on-defect high
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
| `native` | Unity tests of `lib/disled_core`, with the protocol vectors, and of the shipped configuration (`src/app/app_config.c`) |

Both are pid.codes test PIDs
([LICENSING.md](../LICENSING.md#usb-identifiers)).

## Layout

| Path | Content |
|---|---|
| `src/`, `lib/disled_core/` | the layers of [architecture.md](../docs/architecture.md#layers) |
| `lib/tinyusb/` | TinyUSB (submodule `upstream/`, at a release tag) and its build description |
| `test/` | Unity tests, one folder per core module, `test_app_config` for the shipped configuration, and the fixtures they share (`test_support.h`) |
| `scripts/` | version from git, Intel HEX, protocol vectors to C |
| `pyproject.toml`, `uv.lock` | build and check tools (PlatformIO, clang-format) for uv, kept up to date by Dependabot |
| `tools/touch_replay/` | replays touch recordings through the detector on the build machine |

[docs/architecture.md](../docs/architecture.md) explains the design, and
[docs/behavior.md](../docs/behavior.md) the lamp behavior.

## Conventions

- C11, in the GNU dialect for the SDK headers, formatted with clang-format
  23 (`.clang-format`). Warnings are errors on our sources.
- No dynamic allocation, no floating point, no 64-bit division
  ([why](../docs/architecture.md#safety-rules)).
- Interrupt handlers use `BSP_ISR` and keep their work short
  ([concurrency](../docs/architecture.md#concurrency)).
- Includes are relative to `src/`, so each one names the layer it uses.
- The firmware version comes from the latest `fw-vX.Y.Z` tag.
