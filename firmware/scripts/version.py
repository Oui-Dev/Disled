# SPDX-License-Identifier: GPL-3.0-or-later
# SPDX-FileCopyrightText: 2026 The Disled contributors
# Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
"""Inject the firmware version, taken from git, into the build.

The version comes from the latest tag matching ``fw-vMAJOR.MINOR.PATCH``.
Without such a tag the version is 0.0.0, which only a debug build accepts.
The full ``git describe`` output (commits since the tag, short hash,
``-dirty``) is kept in the version string so that every build can be traced
back to its sources.
"""

import re
import subprocess
import sys

Import("env")  # noqa: F821 - provided by PlatformIO

TAG_PATTERN = re.compile(r"^fw-v(\d+)\.(\d+)\.(\d+)")


def git_describe(project_dir):
    try:
        result = subprocess.run(
            ["git", "describe", "--tags", "--match", "fw-v*", "--dirty", "--always"],
            cwd=project_dir,
            capture_output=True,
            text=True,
            check=True,
        )
        return result.stdout.strip()
    except (OSError, subprocess.CalledProcessError):
        return "unknown"


def parse_version(describe):
    match = TAG_PATTERN.match(describe)
    if not match:
        return (0, 0, 0), "0.0.0+" + describe
    numbers = tuple(int(part) for part in match.groups())
    return numbers, describe[len("fw-v"):]


described = git_describe(env.subst("$PROJECT_DIR"))  # noqa: F821
(major, minor, patch), version_string = parse_version(described)

# A firmware that reports 0.0.0 would look older than any release to the app.
if env["PIOENV"] == "release" and not TAG_PATTERN.match(described):  # noqa: F821
    sys.stderr.write(
        "Error: a release build takes its version from a fw-vX.Y.Z tag, and git "
        "describe gives '%s': fetch the tags (git fetch --tags)\n" % described
    )
    env.Exit(1)  # noqa: F821

env.Append(  # noqa: F821
    CPPDEFINES=[
        ("DISLED_FW_VERSION_MAJOR", major),
        ("DISLED_FW_VERSION_MINOR", minor),
        ("DISLED_FW_VERSION_PATCH", patch),
        ("DISLED_FW_VERSION_STRING", env.StringifyMacro(version_string)),  # noqa: F821
    ]
)
print("Firmware version: %s" % version_string)
