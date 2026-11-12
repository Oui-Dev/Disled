# SPDX-License-Identifier: GPL-3.0-or-later
# SPDX-FileCopyrightText: 2026 The Disled contributors
# Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
"""Give the firmware behind the HID bootloader its whole flash area.

The platform writes the linker script into the build folder, with a flash
length in whole KB: 27 KB of the 28416 bytes (0x1000-0x7EFF) that the
bootloader accepts (the *_bl environments). Run after the platform, this
script writes the exact length, board_upload.maximum_size, so that the
linker and the size checks agree.
"""

import re

Import("env")  # noqa: F821 - provided by PlatformIO

size = int(env.BoardConfig().get("upload.maximum_size"))  # noqa: F821
path = env.subst("$LDSCRIPT_PATH")  # noqa: F821
# Only the script the platform generated, never one of the repository.
if not path.startswith(env.subst("$BUILD_DIR")):  # noqa: F821
    raise SystemExit(f"bl_ldscript.py: {path} is not a generated linker script")
with open(path, encoding="utf-8") as file:
    script = file.read()
script, count = re.subn(
    r"(FLASH \(rx\) : ORIGIN = 0x[0-9A-Fa-f]+, LENGTH = )\w+",
    rf"\g<1>{size}",
    script,
)
if count != 1:
    raise SystemExit(f"bl_ldscript.py: no flash region in {path}")
with open(path, "w", encoding="utf-8") as file:
    file.write(script)
