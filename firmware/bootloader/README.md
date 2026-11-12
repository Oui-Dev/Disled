# DISLED HID bootloader (prototype)

A bootloader in the first 4 KB of the flash that updates the firmware over
USB HID: no driver on any system, no BOOT button, the cube stays closed.
The application moves to 0x1000. The WCH ROM bootloader stays the way back
when everything else fails ([docs/flashing.md](../../docs/flashing.md)).

Status: prototype. It builds and its logic is unit tested, but it has not
run on a board yet.

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
   a reset. The CRC loop is 17 instructions per byte, so about 50 ms for an
   18 KB image at 8 MHz, still to be measured on a board.

To enter update mode, the application enables the PWR and BKP clocks, sets
`PWR_CTLR.DBP`, writes `0xB007` to `BKP->DATAR8` and resets
(`NVIC_SystemReset()`). A power cycle clears `DATAR8`.

## Update mode

Crystal and PLL at 48 MHz (USB clock; flash programming needs 60 MHz or
less). The USB device is `1209:0003`, "DISLED bootloader", with the serial
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
(or `debug_bl`). One image for the ROM bootloader, from `firmware/`:

```sh
uv run python bootloader/scripts/make_image.py --bootloader bootloader/.pio/build/bootloader/firmware.bin \
    --app .pio/build/release_bl/firmware.bin --tag 0x010000 --out disled-combined
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
