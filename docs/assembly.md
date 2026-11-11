# Assembling a lamp

This guide builds one Disled lamp from its design files: the board, the
printed enclosure and the touch electrode, then the firmware and a last
test. For a batch, [production.md](production.md) flashes and tests the
boards one after the other.

The files come from a pair of releases, `pcb-vX.Y.Z` and
`enclosure-vX.Y.Z`, on the
[releases page](https://github.com/Oui-Dev/Disled/releases), or from the
`hardware/` folder of the repository
([hardware/README.md](../hardware/README.md)).

## What you need

| Item | Quantity | Use |
|---|---|---|
| Board, pcb 1.0.0 | 1 | Assembled by JLCPCB ([step 1](#1-order-the-board)) |
| JST B2B-XH-AM connector, LCSC C161869 | 1 | CN1, the touch connector, soldered by hand |
| JST-XH 2-pin housing with one crimped wire | 1 | From CN1 to the copper ring |
| Copper tape with adhesive, 30 mm wide or more | 1 piece | The touch electrode, a Ø28.5/Ø17.5 mm ring. A 60 mm tape gives two rings across its width. |
| White PLA, black PETG and TPU | | The printed parts ([step 2](#2-print-the-parts)) |
| Cyanoacrylate glue (super glue) | | The four feet |
| USB-C cable that carries data | 1 | |

Tools: a 3D printer, a soldering iron, a drill press, and a computer with
the [desktop app](user-guide.md#install-the-app). The ring cutter, a
printed tool made once, also takes two M3 heat-set inserts, two M3 × 6 mm
pan-head screws, two segments of a 9 mm snap-off cutter blade, and a nail
thin enough for the 2.6 mm bore along its axis.

## 1. Order the board

JLCPCB assembles the board, on its top side only (Economic PCBA). Open the
project in EasyEDA Pro, `disled-pcb-X.Y.Z.epro2` from the release or
`connected_status_light.epro2` in the repository, and order the board and
its assembly from there: every part carries its LCSC number. Leave CN1, a
through-hole part, out of the assembly: you solder it yourself.

A board of a new revision goes through [bringup.md](bringup.md) before any
lamp is built with it.

## 2. Print the parts

| Part | File | Material | Quantity | Layers |
|---|---|---|---|---|
| Cube | `top.3mf` | White PLA | 1 | 0.16 mm |
| Base | `bottom.3mf` | Black PETG | 1 | 0.16 mm |
| Sensor hood | `als_light_cover.3mf` | Black PETG | 1, the plate holds nine | 0.12 mm |
| Foot | `anti_slip_foot.3mf` | TPU | 4 | |
| Ring cutter | `copper_ring_cutter.3mf` | PETG | 1, a tool made once | 0.16 mm |

A release names them with a prefix, `disled-enclosure-X.Y.Z-top.3mf` for
instance. They are Bambu Studio projects, with the settings of the
released parts for a Bambu Lab A1 and a 0.4 mm nozzle: orientation, walls,
infill, and supports painted under the rounded edges only, so that they
come out clean. With another printer, keep the materials, the orientations
and the layer heights. The foot file holds the shape only.

- **The cube** prints lid down. The LED shines through 1 mm of it, and the
  colors were tuned through white PLA (the 3MF uses the Creality Hyper PLA
  profile). Another material or another white shifts them: check them on
  the closed lamp ([bringup.md](bringup.md#4-led)).
- **The clips.** Two hooks of the cube, on the left and right walls, go
  0.4 mm into the base, so the fit depends on the accuracy of the printer.
  Try the empty cube on its base: it clicks in on both sides, holds when
  the lamp is lifted by the cube, and comes off when pinched on its grip
  stripes.

## 3. Flash the board

The buttons sit on the board, so flash it before the assembly.

1. Solder CN1 onto the top side, turned as its outline on the silkscreen.
2. In the desktop app, open **Settings**, then the **⋮** menu, **Reinstall
   the firmware**. Keep the recommended version, plug the board in, and
   follow the window: hold **BOOT**, press and release **RESET**, then
   release **BOOT** ([user guide](user-guide.md#reinstall-the-firmware)).
   On Windows, the window asks once for the driver of the update mode.
3. The lamp fades in green, and the app finds it.

For a batch, `disled-factory` flashes the boards and runs the self-test
([production.md](production.md)).

The release firmware still uses `1209:0001`, a test PID of pid.codes, until
pid.codes grants the lamp its own, `1209:D1ED`. A firmware update then
moves the lamp to it ([LICENSING.md](../LICENSING.md#usb-identifiers)).

## 4. Cut the copper ring

The cutter holds two blade points at 8.75 and 14.25 mm from the axis of its
shank, the radii of the ring, and cuts both circles at once.

1. Press the two M3 heat-set inserts into the holes at the ends of the
   cutter with a soldering iron.
2. Snap two segments off the 9 mm blade. Slide one into each slot, point
   down and standing out of the bottom face, and clamp it with an M3 screw.
3. Chuck the shank, 10 mm across, in the drill press, with the nail through
   the bore along its axis, point down: the nail centers the cutter as it
   comes down.
4. Lay the copper tape on a flat board, copper up. With the drill press
   running, lower the cutter onto the tape until both blades have gone
   through the copper.
5. Lift the ring out of the tape, still on its liner.

## 5. Fit the electrode in the cube

The lead has one wire, on pin 2 of CN1 (touch). Pin 1, the ground, stays
empty.

1. Cut the wire about 1 cm longer than the cube is tall, and solder it onto
   the copper face of the ring. The adhesive need not conduct.
2. Peel the liner and stick the ring, copper face out, in the recess under
   the lid, which centers it. Press it flat.
3. Leave the wire free. With the cube lifted off its base, it reaches CN1,
   to be plugged in or out.

## 6. Assemble the base

1. Glue the four feet into their recesses under the base.
2. Lay the board in the base: the USB-C receptacle in the opening of the
   back wall, the light sensor (U3) against the window of the left wall.
   The board rests on a ledge, and the lip of the cube keeps it there.
3. Press the sensor hood over the light sensor. Its ribs hold it, and ribs
   of the base close the slot at the board edge under it, so that the
   sensor sees its window only, not the LED.
4. Plug the lead into CN1.

## 7. Close the lamp

The cube fits one way: the gaps in its lip go over the USB-C receptacle,
at the back, and over the sensor hood, on the left. Push it down until both
clips click, with the wire clear of the lip.

To open the lamp, pinch the cube on its grip stripes, over the clips, and
lift it.

## 8. Test the lamp

Plug the closed lamp into the computer.

1. The lamp fades in green, and the app finds it.
2. In the Diagnostics window, run the self-test: check the four colors,
   then touch the lid while it glows blue. Every check passes
   ([user guide](user-guide.md#diagnostics)).
3. A short touch on the lid changes the color, and a 2 s touch turns the
   lamp off.

For a batch, `disled-factory --no-flash` runs the same checks
([production.md](production.md#second-pass)).
