# Disled hardware

This folder holds the board and the enclosure of the lamp. Each has its own
versions, released with its files on the
[releases page](https://github.com/Oui-Dev/Disled/releases) as
`pcb-vX.Y.Z` and `enclosure-vX.Y.Z`
([docs/releasing.md](../docs/releasing.md#hardware-releases)).

| Path | Content |
|---|---|
| `pcb/` | The board: EasyEDA Pro project (`connected_status_light.epro2`), 3D model (`connected_status_light.step`) and [changelog](pcb/CHANGELOG.md) |
| `enclosure/` | The printed parts (3MF) and the tool that cuts the copper ring, with their [changelog](enclosure/CHANGELOG.md) |

Each folder holds the current version only, and a new version replaces the
files under the same names. The older versions are in their releases.
[docs/assembly.md](../docs/assembly.md) builds a lamp from these files.

## PCB

The board, `connected_status_light`, is pcb 1.0.0: 35 × 35 mm, 2 layers,
assembled by JLCPCB (Economic PCBA, top side only). The JST connector of
the touch electrode is soldered by hand.

### Pin map (CH32V203G6U6, QFN28)

| Signal | MCU pin | Function | Notes |
|---|---|---|---|
| LED_R | PA6 (12) | TIM3_CH1 | Gate of Q2, active high, 10 kΩ pull-down |
| LED_G | PA7 (13) | TIM3_CH2 | Gate of Q3, active high, 10 kΩ pull-down |
| LED_B | PB0 (14) | TIM3_CH3 | Gate of Q1, active high, 10 kΩ pull-down |
| TOUCH | PB1 (15) | ADC_IN9 / TouchKey 9 | CN1.2 through R6 (2.2 kΩ) |
| SCL / SDA | PB6 (27) / PB7 (28) | I2C1 | VEML7700 at 0x10, 2.2 kΩ pull-ups |
| USB D− / D+ | PA11 (19) / PA12 (20) | USBD | Pin 19 is also bonded to PA10 |
| SWDIO / SWCLK | PA13 (21) / PA14 (22) | SDI debug | Back-side test pads |
| OSC_IN / OSC_OUT | PD0 (2) / PD1 (3) | HSE | 8 MHz crystal, 18 pF |
| BOOT0 | PB8 / BOOT0 (1) | Boot select | 4.7 kΩ pull-down, BOOT button to 3.3 V |
| NRST | 4 | Reset | RESET button, 100 nF |
| Unused | PA0 to PA5, PA9, PA15, PB3 to PB5 | | Configure as analog inputs |

### Constraints the firmware must respect

They come from the design review of the board before production
([PDF, in French](https://github.com/Oui-Dev/Disled/blob/d5e8749603c0b48a2b3b314eaaab88630da2aa92/hardware/review/Review_PCB_v4_connected_status_light_2026-10-10.pdf),
in the git history):

- **N1, touch.** The touch trace runs next to LED_B. Synchronize TouchKey sampling with the LED PWM.
- **N2, LED power.** Typical currents at 100 % are R 75, G 52 and B 98 mA, and 86, 63 and 125 mA in the worst case. White at 100 % draws about 225 mA and 1.1 W inside a closed PLA cube, so the firmware caps the total LED current. Declare `bMaxPower` = 300 mA.
- **N3, shared pins.** Pin 19 bonds PA10 to PA11 (USB D−). Never drive PA10, never enable a pull-up on it and never route USART1 RX to it. Pin 1 bonds BOOT0 to PB8, so never drive PB8 at boot.

## Enclosure

The enclosure is a white cube clipped on a black base. The copper ring
under the lid of the cube is the touch electrode.

| File | Part |
|---|---|
| `top.3mf` | The cube (white PLA), printed top face down, with the recess of the copper ring under its lid |
| `bottom.3mf` | The base (black PETG): USB-C mask, light sensor window, clip pockets, feet recesses |
| `als_light_cover.3mf` | The hood over the light sensor (black PETG), so that it sees its window only, not the LED |
| `anti_slip_foot.3mf` | A foot (TPU), glued in a recess under the base. Print four. |
| `copper_ring_cutter.3mf` | A rotary head for a drill press that cuts the Ø28.5/Ø17.5 mm ring from copper tape, with two snap-off blade segments clamped by M3 screws in heat-set inserts |

## License

CERN-OHL-S-2.0, see [LICENSE](LICENSE).
