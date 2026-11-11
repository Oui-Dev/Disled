# Flashing the firmware

You flash Disled over USB, through the bootloader in the ROM of the
CH32V203. That bootloader stays in place whatever firmware you write, so a
bad firmware never bricks a board: the BOOT button always brings it back.
Write a `.hex` or a `.bin`, never an `.elf`: a broken ELF file was reported
to erase the bootloader once. The buttons sit on the board, so the
enclosure has to be open. The CH32V203 starts its bootloader only from a
reset with BOOT held, never from software.

## From the desktop app

The desktop app offers a new firmware when the connected lamp runs an older
one, and its **Update** button guides the update in a window of its own,
the Windows driver included ([user guide](user-guide.md#updates)). A
version 1 lamp gets the same button under "Update required". A lamp that
no longer starts, or one that should run another version, gets its
firmware from **Reinstall the firmware**, in the **⋮** menu of the settings
page of the popup, where you pick the version: 1.0.0 takes the lamp back to
Disled v1. The app empties the whole flash before it writes the firmware.

The rest of this page is for development and for flashing by hand.

## Enter the bootloader

1. Plug the board into the computer, directly or through a hub.
2. Hold **BOOT**, press and release **RESET**, then release **BOOT**.
   Holding BOOT while plugging the cable in works too.
3. The chip now shows up as a WCH device instead of Disled. Flash it
   right away.

## First time on Windows: install the WinUSB driver

PlatformIO and the desktop app flash with
[`wchisp`](https://github.com/ch32-rs/wchisp), which talks to the
bootloader through WinUSB.

1. Enter the bootloader (above).
2. Run [Zadig](https://zadig.akeo.ie/), then **Options**, **List All
   Devices**.
3. Select the WCH device, whose USB ID is `4348:55E0` or `1A86:55E0`.
4. Choose **WinUSB** and click **Replace Driver** or **Install Driver**.

This replaces the official WCH driver for that device, so WCH's own
WCHISPTool no longer sees it. That is expected.

macOS needs no driver.

## Flash

From `firmware/`:

```sh
pio run -e debug -t upload     # development firmware: HID + CDC console
pio run -e release -t upload   # production firmware: HID only
```

To flash a published build instead, download `disled-fw-X.Y.Z.hex`, or
`disled-fw-X.Y.Z-debug.hex`, from the
[releases page](https://github.com/Oui-Dev/Disled/releases) and run
[wchisp](https://github.com/ch32-rs/wchisp) directly. PlatformIO installs
it in `~/.platformio/packages/tool-wchisp`:

```sh
wchisp erase                   # the whole flash, as the desktop app does
wchisp flash disled-fw-X.Y.Z.hex
```

`wchisp flash` erases only the sectors the new firmware covers: after a
larger firmware, such as a debug build, the rest of the old one would stay
behind it. `wchisp erase` first empties the whole flash.

After flashing, press **RESET**, or unplug the board and plug it back in.
The lamp starts green and shows up as:

| Build | USB ID | Interfaces |
|---|---|---|
| debug | `1209:0002` | HID "Disled control" + serial port "Disled console" |
| release | `1209:0001` | HID "Disled control" |

Both are test PIDs of pid.codes. The release builds move to the lamp's own,
`1209:D1ED`, once pid.codes grants it, and the debug builds keep `1209:0002`
([LICENSING.md](../LICENSING.md#usb-identifiers)).

## Debug console

A debug build adds a serial port. Open it with any terminal, at any baud
rate:

```sh
pio device monitor          # PlatformIO picks the port
```

Type `help` for the list of commands:

| Command | Use |
|---|---|
| `state` | lamp, USB link and LED output, the duties to 1/16 tick |
| `set on\|off\|auto\|color <0-15>\|level <1-100>\|bias <-50..50>\|sensitivity <-50..50>` | change the lamp as the host does; a color slot must hold a color, so 4 to 15 need a host that set them |
| `watch <ms>\|off` | stream touch, light (lux, count, level) and current (`watch 20`: every touch window) |
| `cap <mA>\|off` | override the LED current cap, for the heat test of [bringup.md](bringup.md#8-heat) only |
| `alert [breaths] [period ms] [depth]` | breathe as for an alert from the host, 3 breaths of 1.5 s by default; a depth in per mille, 0 to 500, stays for the next alerts, to compare depths on a closed lamp; `alert 0` stops it |

The Diagnostics window of the desktop app shows the rest: chip, faults,
counters and the self-test.

## Without PlatformIO

Any WCH ISP tool can flash `firmware.bin` or `firmware.hex`, from
`firmware/.pio/build/<env>/` or from the CI artifacts: `wchisp erase` then
`wchisp flash firmware.bin`, WCHISPTool on Windows, or WCHISPStudio on
macOS.

For a batch of boards, `disled-factory` flashes and tests them one after
the other ([production.md](production.md)).

## Troubleshooting

| Symptom | Check |
|---|---|
| `wchisp` finds no device | The board is in its bootloader, Zadig installed WinUSB for the WCH device on Windows, and the cable carries data. |
| The lamp stays dark after flashing | Press RESET. The bootloader keeps running until a reset. |
| Windows shows an old interface set | Uninstall the device in Device Manager. Windows caches the interfaces per USB ID, which is why debug and release builds use different PIDs. |
| The console port does not appear | Only debug builds have it. |
