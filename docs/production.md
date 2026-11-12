# Production

`disled-factory` ([`tools/disled-factory`](../tools/disled-factory))
flashes, checks and self-tests the boards one after the other, and logs
each one in a CSV file. It runs from the repository and has no release. A
board takes about 30 seconds.

## Before a batch

- **The repository and Rust**, through `rustup` at the pinned version
  ([CONTRIBUTING.md](../CONTRIBUTING.md#before-you-start)), to build the
  tool.
- **wchisp.** `cargo install wchisp@0.3.0 --locked` puts it on the PATH.
  Without it, the tool takes the one PlatformIO installs for the firmware.
  `--wchisp` gives another one.
- **Bootloader access**, once per computer. On Windows, install the WinUSB
  driver with Zadig
  ([flashing.md](flashing.md#first-time-on-windows-install-the-winusb-driver)).
  The tool shows the steps when the driver is missing. On Linux, install
  the udev rule of the app ([app/README.md](../app/README.md#the-udev-rule-on-linux)).
  macOS needs nothing.
- **The firmware.** From the firmware release to produce (`fw-vX.Y.Z` on
  the [releases page](https://github.com/Oui-Dev/Disled/releases)),
  download the release build `disled-fw-X.Y.Z.hex` and its signature
  `disled-fw-X.Y.Z.hex.sig` into the same folder. The tool flashes only a
  release build signed with the update key.
- **USB IDs.** The release builds use `1209:0001`, a test PID of pid.codes,
  until the lamp gets its own, `1209:D1ED`
  ([LICENSING.md](../LICENSING.md#usb-identifiers)). The tool accepts
  `1209:0001` until then, and refuses it from the first release whose
  firmware uses `1209:D1ED`.

## Run

```sh
cargo run --release -p disled-factory -- path/to/disled-fw-X.Y.Z.hex
```

| Option | Use |
|---|---|
| `--log <file>` | CSV log, appended (default `disled-production.csv`) |
| `--no-flash` | Test lamps that already run their firmware: no bootloader, no flash |
| `--unsigned` | Flash a firmware without signature, at your own risk (below) |
| `--wchisp <path>` | The wchisp program to use |

Stop with Ctrl+C. The log already holds every finished board.

## For each board

1. Plug the board in with **BOOT** held, the only way a blank board starts
   its bootloader. The tool finds the bootloader and flashes the board:
   erase, write, verify, restart.
2. The lamp starts its firmware. The tool checks what it reports
   (`INFO`): the version of the release, a release build, protocol 2.
3. **Self-test.** The lamp shows red, green, blue and warm white, 1.5 s
   each, while the tool names them. **Touch the lid** when it glows blue,
   within 10 s. The lamp checks its light sensor and its touch by itself.
4. The tool asks whether the four colors were right, and if not, which
   ones.
5. The tool shows `PASS` or `FAIL` and writes one row in the log. Unplug the
   board, and the tool waits for the next one.

A board plugged in without BOOT starts its firmware, if it has one. The
tool says so: unplug it and plug it in again with BOOT held.

## Second pass

`--no-flash` tests lamps that already run their firmware, without the
buttons. Use it once the board sits in its enclosure, for instance, to
check the touch through the lid. The firmware file still gives the version
the lamps must report.

## The log

One row per board, in `disled-production.csv` unless `--log` says
otherwise:

| Column | Content |
|---|---|
| `date` | End of the test, UTC (RFC 3339) |
| `uid` | Chip unique ID, also the USB serial number |
| `firmware` | Version the lamp reports |
| `signature` | `checked`, or `none` with `--unsigned` |
| `board_revision` | Board revision the firmware reports |
| `colors` | `ok`, or `fail:` and the wrong colors (operator) |
| `light_sensor`, `lux` | The sensor answered; the light it measured |
| `touch_idle`, `touch_baseline` | Touch reading at rest in the expected range; that reading |
| `touch`, `touch_peak_permille` | A touch was detected; the largest drop seen |
| `result` | `PASS` when every check passed |
| `note` | Why the board failed outside these checks (flash, version), and remarks such as a fault reported before the last reset |

## When a board fails

| Failure | Check |
|---|---|
| The bootloader does not show up | Hold BOOT while plugging in, and use a cable that carries data. On Windows, check the WinUSB driver: the tool says when it is missing. |
| wchisp error | Plug the board in again with BOOT held. Flashing again is harmless. |
| The lamp does not start | Press RESET, or plug it in again. |
| Wrong firmware | The lamp does not run the given release. Check the file and flash again. |
| Light sensor | The sensor and its soldering. |
| Touch at rest out of range | The electrode, its trace and soldering. |
| No touch detected | Touch the lid within the 10 s of blue. If the touch still fails, check the electrode. |
| Colors | The LED, its MOSFETs and their soldering. |

The ROM bootloader stays whatever firmware a `.hex` or a `.bin` writes, so a
board that failed can always be flashed and tested again.

## Unsigned firmware

`--unsigned` flashes any firmware file without checking it, to try the tool
with a local build or to install a custom firmware, at your own risk. Give
it a `.hex` or a `.bin`, never an `.elf`: a broken ELF file was reported to
erase the ROM bootloader once. The tool then checks no version, and the log
marks the rows `signature: none`.
