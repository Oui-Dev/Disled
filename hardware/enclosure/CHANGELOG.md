# Changelog

The versions of the DISLED enclosure, written by hand
([docs/releasing.md](../../docs/releasing.md#hardware-releases)). The
dates are those of the designs.

## 1.1.2 (2026-11-14)

### 🚀 Enhancements

* **ALS cover:** two ears at its top, one on each end, lift it out with
  tweezers or a small flat screwdriver.

### 🩹 Fixes

* **ALS cover:** its front fills the gap between the wall of the base and
  the face of the sensor, on both sides of a 4.4 mm light channel, so it
  can no longer move back. Held by its crush ribs alone, it could be
  pushed back by the wall of the cube, which bends in when the box is
  closed or pinched open, and the LED then lit the sensor.
* **ALS cover:** the edge of the step that sits under the cube has a 45°
  chamfer: a wall bent in at closing slides over it instead of landing on
  the cover.

### ⚠ Compatibility

* The base and the cube do not change: a lamp already built only needs
  the new cover.

## 1.1.1 (2026-10-31)

### 🚀 Enhancements

* **bottom:** variable layer heights, as for the top. The part itself does
  not change.

## 1.1.0 (2026-10-31)

Enclosure V3, with clips that keep their hold.

### 🚀 Enhancements

* **top, bottom:** scarf seams on the contours and the holes hide the
  seam line.

### 🩹 Fixes

* **clips:** the hooks and their pockets hold by a 45° face, up from 8°.
  The 8° faces printed as unsupported overhangs, and the 0.05 mm preload
  could only be taken by crushing their edges: the box got loose with
  use. At 45°, the faces print clean, and the same preload bends the
  skirt by 0.05 mm.
* **clips:** the hooks stand out 0.55 mm, up from 0.5, and go 0.4 mm into
  the base, up from 0.35. They sit 0.2 mm higher on the skirt. The wall
  behind the pockets keeps its 1.1 mm, which the slicer prints with three
  lines, so the pockets leave no mark on the outside of the base.
* **clips:** the cube still comes off when pinched on its grip stripes,
  and now also with a firm pull.

### ⚠ Compatibility

* A 1.1.0 cube fits a 1.0.0 base, with a firmer hold. A 1.0.0 cube
  barely holds in a 1.1.0 base.

## 1.0.0 (2026-10-24)

Enclosure V2, reworked after the first prototype with the board.

### 🚀 Enhancements

* **top:** grip stripes over the clips show where to pinch to open the
  box.
* **top:** the recess takes a Ø28.5/Ø17.5 mm copper ring, down from
  Ø30/Ø20.
* **bottom:** a 3 × 1.6 mm light sensor window, up from 2 × 2, with its
  flare matched to the light rays.
* **copper ring cutter:** cuts the new ring, with its blades clamped by
  M2.5 screws in heat-set inserts; the press-fit plug (blade holder) is
  gone.

### 🩹 Fixes

* **ALS cover:** no more pins, they broke at the root. Crush ribs hold it
  on the sensor, and ribs on the base close the PCB-edge slot under it.
* **bottom:** the USB-C recess is 0.3 mm shallower: the receptacle face
  stood proud.
* **bottom:** the feet recesses are 1 mm deep: at 0.3 mm they could
  hardly be felt.
* **clips:** the hooks and their pockets have an 8° face, down from 10°,
  and a 0.05 mm preload: the box got easier to open with use.

## 0.1.0 (2026-10-20)

Enclosure V1, the first prototype, printed and tried with the first board
(pcb 1.0.0).

* **top:** the cube, printed top face down, with a recess for a Ø30/Ø20 mm
  copper ring, the touch electrode, under its lid.
* **bottom:** the base: USB-C mask, 2 × 2 mm light sensor window,
  ventilation slots, clip pockets and 0.3 mm recesses for the feet.
* **ALS cover:** a hood over the light sensor, located by two pins in the
  PCB-edge slot.
* **anti-slip foot:** four, glued under the base.
* **copper ring cutter:** a rotary head for a drill press that cuts the
  ring from copper tape, with its blades held by a press-fit plug (blade
  holder).
