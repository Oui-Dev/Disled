# SPDX-License-Identifier: GPL-3.0-or-later
# SPDX-FileCopyrightText: 2026 The Disled contributors
# Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
"""Also build the firmware as Intel HEX, next to the ELF and the binary.

The platform only makes ``firmware.bin``; the releases, the WCH ISP tools
and ``wchisp`` (docs/flashing.md) also use ``firmware.hex``.
"""

Import("env")  # noqa: F821 - provided by PlatformIO

env.Default(  # noqa: F821
    env.ElfToHex("$BUILD_DIR/${PROGNAME}", "$BUILD_DIR/${PROGNAME}.elf")  # noqa: F821
)
