# SPDX-License-Identifier: GPL-3.0-or-later
# SPDX-FileCopyrightText: 2026 The Disled contributors
# Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
"""Build firmware.bin before an upload that sends it.

The upload target of the platform depends on the ELF only, so that
``pio run -t upload`` would send a missing or stale binary to the HID
bootloader (the *_bl environments).
"""

Import("env")  # noqa: F821 - provided by PlatformIO

env.Depends(  # noqa: F821
    env.Alias("upload"),  # noqa: F821
    env.File("$BUILD_DIR/${PROGNAME}.bin"),  # noqa: F821
)
