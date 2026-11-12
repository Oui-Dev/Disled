# SPDX-License-Identifier: GPL-3.0-or-later
# SPDX-FileCopyrightText: 2026 The DISLED contributors
# Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
"""Combine the bootloader and an application into one 32 KB flash image.

The image is what the WCH ROM bootloader flashes once (``wchisp flash``):
the bootloader at 0x0000, the application at 0x1000, and the validity
record in the last page, 0x7F00, as the bootloader writes it after an
update (lib/bl_core/include/bl_core.h). The rest is 0xFF.

    python make_image.py --bootloader BL.bin --app APP.bin --out disled-combined

writes disled-combined.bin and disled-combined.hex. --no-record and
--bad-crc build the images of the on-board tests (README.md): the
bootloader must then stay in update mode.

Standard library only; runs with any Python 3.8 or later.
"""

import argparse
import struct
import sys
import zlib

FLASH_SIZE = 0x8000
APP_OFFSET = 0x1000
RECORD_OFFSET = FLASH_SIZE - 256
APP_MAX = RECORD_OFFSET - APP_OFFSET
RECORD_MAGIC = 0x50414C44  # "DLAP"


def record(app, tag, bad_crc=False):
    crc = zlib.crc32(app) & 0xFFFFFFFF
    if bad_crc:
        crc ^= 0x00000001
    return struct.pack("<IIII", RECORD_MAGIC, len(app), crc, tag)


def build(bootloader, app, tag=0, with_record=True, bad_crc=False):
    if len(bootloader) > APP_OFFSET:
        raise ValueError("bootloader of %d bytes: more than %d" % (len(bootloader), APP_OFFSET))
    if not 0 < len(app) <= APP_MAX:
        raise ValueError("application of %d bytes: 1 to %d" % (len(app), APP_MAX))
    image = bytearray(b"\xff" * FLASH_SIZE)
    image[0:len(bootloader)] = bootloader
    image[APP_OFFSET:APP_OFFSET + len(app)] = app
    if with_record:
        rec = record(app, tag, bad_crc)
        image[RECORD_OFFSET:RECORD_OFFSET + len(rec)] = rec
    return bytes(image)


def intel_hex(data, base=0):
    """Intel HEX of `data` at `base` (below 64 KB), 16 bytes a record."""
    lines = []
    for offset in range(0, len(data), 16):
        chunk = data[offset:offset + 16]
        address = base + offset
        fields = bytes([len(chunk), (address >> 8) & 0xFF, address & 0xFF, 0x00]) + chunk
        checksum = (-sum(fields)) & 0xFF
        lines.append(":" + fields.hex().upper() + "%02X" % checksum)
    lines.append(":00000001FF")
    return "\n".join(lines) + "\n"


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--bootloader", required=True, help="bootloader firmware.bin")
    parser.add_argument("--app", required=True, help="application firmware.bin, linked at 0x1000")
    parser.add_argument("--out", required=True, help="output path, without extension")
    parser.add_argument("--tag", type=lambda s: int(s, 0), default=0,
                        help="value kept in the record, e.g. 0x010000 for version 1.0.0")
    parser.add_argument("--no-record", action="store_true",
                        help="leave the record out (test: the bootloader stays)")
    parser.add_argument("--bad-crc", action="store_true",
                        help="record with a wrong CRC (test: the bootloader stays)")
    args = parser.parse_args(argv)

    with open(args.bootloader, "rb") as f:
        bootloader = f.read()
    with open(args.app, "rb") as f:
        app = f.read()
    try:
        image = build(bootloader, app, args.tag, not args.no_record, args.bad_crc)
    except ValueError as error:
        sys.stderr.write("make_image: %s\n" % error)
        return 1

    with open(args.out + ".bin", "wb") as f:
        f.write(image)
    with open(args.out + ".hex", "w", newline="\n") as f:
        f.write(intel_hex(image))
    print("bootloader %d bytes, application %d bytes (CRC-32 %08X), record %s -> %s.bin/.hex"
          % (len(bootloader), len(app), zlib.crc32(app) & 0xFFFFFFFF,
             "none" if args.no_record else ("bad CRC" if args.bad_crc else "valid"), args.out))
    return 0


if __name__ == "__main__":
    sys.exit(main())
