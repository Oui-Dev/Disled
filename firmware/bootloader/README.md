# Disled HID bootloader (prototype)

A bootloader in the first 4 KB of the flash that updates the firmware over
USB HID: no driver on any system, no BOOT button, the lamp stays closed.
The application moves to 0x1000. The WCH ROM bootloader stays the way back
when everything else fails ([docs/flashing.md](../../docs/flashing.md)).

Status: prototype, set aside on 2026-11-08, since Disled 2.0 updates every
lamp through the ROM bootloader. It builds with firmware 2.0, its logic is
unit tested, and it passed the [board tests](#board-tests) on Windows 11.
[Coming back](#coming-back) lists what it still needs.

## Flash layout

| Address | Size | Content |
|---|---|---|
| `0x0000` | 4 KB | Bootloader: one write-protect sector |
| `0x1000` | 28,416 bytes | Application (111 pages of 256 bytes) |
| `0x7F00` | 256 bytes | Validity record: magic `0x50414C44` ("DLAP"), size, CRC-32, tag |

The record is erased first by an update and written last, after the
bootloader has checked the CRC-32 (zlib) of the image in flash. Erased flash
reads `0xE339E339` on this chip, never the magic.

## Boot

After a reset, on the internal 8 MHz oscillator:

1. `BKP->DATAR8` holds `0xB007`: the application asked for update mode. The
   bootloader clears it (once) and stays.
2. No record, a size out of range or a CRC mismatch: it stays.
3. Otherwise it jumps to `0x1000` in machine mode, with interrupts off and
   every clock at its reset value: the application's startup runs as after
   a reset. The CRC loop is 17 instructions per byte, about 50 ms for an
   18 KB image at 8 MHz: on the board, the lamp came back on the USB 446 ms
   after a RESET through the bootloader, 389 ms without it, for an 18.5 KB
   application.

To enter update mode, the application enables the PWR and BKP clocks, sets
`PWR_CTLR.DBP`, writes `0xB007` to `BKP->DATAR8` and resets
(`NVIC_SystemReset()`). A power cycle clears `DATAR8`.

Firmware 2.x does it on `ENTER_UPDATE` (`0x09`, no body), which it answers
with `ACK` and announces with capability bit 5, in its `debug_bl` and
`release_bl` builds only (`DISLED_ENTER_UPDATE`, `bsp_reset_to_update()`).
Its other builds keep `0x09` and bit 5 reserved, as protocol 2.0 says, and
answer `ERROR` (unknown command). A request that reaches the application
found no bootloader: the application drops it at boot.

## Update mode

Crystal and PLL at 48 MHz (USB clock; flash programming needs 60 MHz or
less). The USB device is `1209:0003`, "Disled bootloader", with the serial
number of the application (the chip unique ID), bus powered at 100 mA. The
green LED blinks slowly while waiting and fast during an update (63 mA at
most, constraint N2), and goes off when the computer suspends the bus. Pins
PA10 and PB8 are never configured (N3).

Update mode ends with a reset only: after `BOOT`, or after 60 s without a
request when the application was valid and no update has begun.

`1209:0003` is a pid.codes test PID: the bootloader needs a PID of its own
before lamps ship with it.

## Protocol

HID interface, vendor page `0xFF00`, 64-byte reports without report ID, on
interrupt endpoints polled every 1 ms. Each request gets one reply; the
bootloader takes the next request only once the host has collected the
reply. With hidapi, write `[0x00, request[0..64]]` and read 64 bytes.

Request:

| Offset | Size | Field |
|---|---|---|
| 0 | 1 | command |
| 1 | 1 | `seq`, echoed in the reply |
| 2 | 1 | data length (`DATA` only), 0 to 56 |
| 3 | 1 | 0 |
| 4 | 4 | argument, `u32` little-endian |
| 8 | 56 | data |

Reply: command, `seq`, status, 0, then the `INFO` body or zeros.

| Command | Argument | Data | Action |
|---|---|---|---|
| `0x01` INFO | | | Reply with the body below |
| `0x02` BEGIN | | | Erase the record: the application is invalid from now on |
| `0x03` DATA | offset in the page buffer | 1 to 56 bytes | Copy into the 256-byte page buffer |
| `0x04` PROGRAM | page index, 0 to 110 | | Erase the page at `0x1000 + 256 × index`, program the buffer, read it back |
| `0x05` FINISH | image size | CRC-32 (`u32`), tag (`u32`) | Check the CRC-32 of the image in flash, then write the record |
| `0x06` BOOT | | | Reply, then reset: the bootloader starts the application if it is valid |

Status: 0 OK, 1 unknown command, 2 argument out of range, 3 `PROGRAM` or
`FINISH` without `BEGIN`, 4 flash error (the page does not read back as
written: write-protected?), 5 CRC mismatch. After a CRC or flash error,
`FINISH` may be sent again.

INFO body, from offset 4 of the reply:

| Offset | Size | Field |
|---|---|---|
| 0 | 2 | protocol version, major and minor (1.0) |
| 2 | 1 | why in update mode, at reset: bit 0 requested by the application, bit 1 no valid application |
| 4 | 4 | application offset, `0x1000` |
| 8 | 4 | maximum application size, 28416 |
| 12 | 2 | page size, 256 |
| 14 | 1 | maximum data per `DATA`, 56 |
| 16 | 4 | record: image size (0 without a record) |
| 20 | 4 | record: CRC-32 |
| 24 | 4 | record: tag (the host's value, such as the firmware version) |

An update: `INFO`, `BEGIN`, then for each page of the image (the last one
padded with `0xFF`) five `DATA` and one `PROGRAM`, then `FINISH`, then
`BOOT`. A power loss at any point leaves the bootloader in update mode with
no valid application, and the next update starts again from `BEGIN`.

## Build and test

From this folder (the tools of `../pyproject.toml`):

```sh
uv run pio run                             # the bootloader, .pio/build/bootloader/firmware.bin
uv run pio test -e native -e native_bitwise # protocol, record and CRC tests
```

The application at `0x1000`, from `firmware/`: `uv run pio run -e release_bl`
(or `debug_bl`), about 21.4 KB and 27.4 KB of the 28,416 bytes with firmware
2.0.0. `scripts/bl_ldscript.py` gives the linker those 28,416 bytes, which
the platform would round down to 27 KB. `uv run pio test -e native_bl`
tests the decoder of these builds. One image for the ROM bootloader, from
`firmware/`:

```sh
uv run python bootloader/scripts/make_image.py --bootloader bootloader/.pio/build/bootloader/firmware.bin \
    --app .pio/build/release_bl/firmware.bin --tag 0x020000 --out disled-combined
wchisp erase
wchisp flash disled-combined.bin
```

`--no-record` and `--bad-crc` build images on which the bootloader must stay
in update mode. The host tool, `tools/disled-hidboot` (`cargo run -p
disled-hidboot -- info|flash APP.bin|boot|enter`), runs an update; its tests
drive this C core through the whole protocol. `uv run pio run -e release_bl -t
upload` goes through it.

## Notes

- The driver of the USB device (`src/usbd.c`) is polled and serves only
  what a HID device needs. It stalls control writes that have a data stage:
  on this chip, endpoint 0 may acknowledge an OUT packet while it should
  answer NAK, an erratum TinyUSB works around.
- The bootloader writes the flash only from `0x1000` on, page by page.
  Write-protecting sector 0 (option bytes) also guards it against a faulty
  application; `wchisp config unprotect` lifts the protection for the ROM
  bootloader.
- Windows suspends the bootloader about 2 s after the last program closes
  it, and the bootloader then turns its LED off, as USB requires (2.5 mA at
  most): the lamp looks dead. A host that waits for an update, or resumes
  one, must keep it open meanwhile.

## Board tests

On 2026-10-31, Windows 11, one lamp, with firmware 1.x and a 2.0 prototype
whose `ENTER_UPDATE` is the one above. Every test passed:

| Test | Result |
|---|---|
| Image without record, through the ROM | The bootloader stays in update mode, a HID device with no driver; `info`: no valid application |
| Firmware 1.x `release_bl` through HID | 18,468 bytes in 1.6 s, 1.96 s in all; the application starts with the same serial number |
| BOOT + RESET, then an image with a bad CRC | The ROM bootloader still starts; the bootloader then stays in update mode |
| Firmware 2.0 `debug_bl` through HID | 26,824 bytes in 2.3 s |
| `enter` | `ACK`, the bootloader 468 ms later, requested by the application; back to the application alone 60.2 s later; `enter` then `boot`: back in about 1 s |
| Firmware 2.0 `release_bl`, from the running application | `flash` sends `ENTER_UPDATE`: 20,892 bytes in 1.8 s, 2.65 s in all |
| Tool killed at 11,008 of 26,824 bytes, lamp unplugged at 6,656 | The bootloader stays in update mode with no valid application, and the next update completes |
| Back to firmware 1.0.0 at `0x0` through the ROM | The lamp works as before |

Not tested on a board: a power cut just before the end of an update (the
unit tests inject it), the current in update mode, the write protection of
sector 0.

## Coming back

- A USB PID of its own: `1209:0003` is a test PID of pid.codes.
- Tests on macOS, on Linux and on Windows 10 behind a hub, and the current
  in update mode.
- `ENTER_UPDATE` in every build, through a new minor version of the
  protocol, with its capability bit.
- A desktop app that updates through the bootloader and keeps it open while
  it waits. Today it installs every firmware through the ROM bootloader,
  after `wchisp erase`, which empties the whole flash, this bootloader with
  it.
- Each lamp gets the bootloader once, opened, through the ROM bootloader:
  the combined image above.
