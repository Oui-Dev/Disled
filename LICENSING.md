# Licensing

Disled is free software and open hardware.

| Part | Path | License |
|---|---|---|
| Firmware, desktop app, protocol, scripts and documentation | everything outside `hardware/` | GPL-3.0-or-later with the additional permission below ([LICENSE](LICENSE)) |
| Hardware design files | `hardware/` | CERN-OHL-S-2.0 ([hardware/LICENSE](hardware/LICENSE)) |

Third-party code keeps its own license:

- **TinyUSB** (`firmware/lib/tinyusb`, git submodule): MIT.
- **WCH CH32V20x SDK**: Nanjing Qinheng Microelectronics terms. PlatformIO downloads it at build time and this repository does not contain it. The additional permission below covers linking with it.
- **wchisp** (the flasher of the lamp updates, in the desktop app installers): GPL-2.0. A separate program that the app runs, never linked with it. Its license and notice are in `app/src-tauri/third-party/wchisp`, and its source is attached to every app release.
- **Montserrat** (the font of the desktop app, bundled from the `@fontsource-variable/montserrat` package): SIL Open Font License 1.1, The Montserrat Project Authors.
- **Lucide** (the icons of the desktop app, bundled from the `@iconify-json/lucide` package): ISC, Lucide Contributors.
- **hyphen** (the hyphenation of the color names in the desktop app, bundled from the `hyphen` package): ISC, Yevhen Tiurin. Its patterns come from the hyph-utf8 project: the French ones under the MIT license, Daniel Flipo, Bernard Gaulle and Arthur Reutenauer; the American English ones under the terms of Gerard D.C. Kuiken, which allow copying and distribution provided the copyright notice and these terms are preserved.

## Software notice

Copyright (C) 2026 The Disled contributors

This program is free software; you can redistribute it and/or modify it
under the terms of the GNU General Public License as published by the Free
Software Foundation; either version 3 of the License, or (at your option)
any later version.

This program is distributed in the hope that it will be useful, but
WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY
or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License
for more details.

You should have received a copy of the GNU General Public License along
with this program; if not, see <https://www.gnu.org/licenses>.

### Additional permission under GNU GPL version 3 section 7

If you modify this Program, or any covered work, by linking or combining
it with the WCH CH32V20x software development kit (the startup code,
system and peripheral libraries and device headers published by Nanjing
Qinheng Microelectronics Co., Ltd.) (or a modified version of that
library), containing parts covered by the terms of the Nanjing Qinheng
Microelectronics software license, the licensors of this Program grant
you additional permission to convey the resulting work.

This text follows the template from the Free Software Foundation GPL FAQ
("What legal issues come up if I use GPL-incompatible libraries with GPL
software?"). The optional sentence that requires shipping the library
source is left out, because the redistribution terms of the WCH SDK are
not clear.

Every source file points to this permission with a header like:

```
SPDX-License-Identifier: GPL-3.0-or-later
SPDX-FileCopyrightText: 2026 The Disled contributors
Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
```

## Contributions

Contributions are accepted under the same terms, including the additional
permission above.

## USB identifiers

The [pid.codes test PIDs](https://pid.codes/1209/0001/) are meant for
private testing. Release builds still use `1209:0001` until pid.codes grants
the lamp its own PID, `1209:D1ED`, requested for them, and the lamps
published until then carry it too. A normal firmware update moves them to
`1209:D1ED` once it is granted, after a version of the desktop app that
knows it. Debug builds keep `1209:0002` for good: they serve private
testing only and never ship.
