# Lamp behavior

This is the reference specification of what the lamp does. The firmware
state machine (`firmware/lib/disled_core/lamp_fsm`) and its unit tests
implement it, and the scenarios at the end verify it on a real board.

## State

| Item | Values |
|---|---|
| Power | `ON`, `OFF` |
| Color | a palette slot: 0 to 3 for the built-in colors, green, orange, red and warm white, 4 to 15 for custom colors that the host defines |
| Brightness mode | `AUTO` (follows the room light) or `MANUAL` (1 to 100 %) |
| Touch mode | `NORMAL` or `POWER_ONLY` |
| Touch cycle | the colors a short press steps through: 1 to 9 slots |
| USB state | `UNCONFIGURED`, `CONFIGURED`, `SUSPENDED` |

At power-up the lamp is `ON`, `GREEN`, `AUTO`, with normal touch, the touch
cycle green, orange, red, no custom color and no correction. The firmware
stores nothing in flash. The desktop app sends its custom colors, its touch
cycle and its settings again at every connection, and the lamp keeps them
until it restarts.

## Touch

| Gesture | Lamp on | Lamp off |
|---|---|---|
| Short press (released before 2 s) | Next color of the touch cycle, and the first one after the last. A color outside the cycle, such as warm white in the default cycle, goes to the first. Ignored in power-only mode. | Ignored |
| Long press (held for 2 s) | Turns off at 2 s, while the finger is still down | Turns on at 2 s, with the last color |
| Contact longer than 10 s | Taken for an object on the lid: no further action, and the baseline recalibrates | Same |

Each contact triggers at most one action. Releasing the touch re-arms it.

The power-only mode suits a host that chooses the color, as the desktop app
does while it follows an integration: a short press does nothing, and a long
press still turns the lamp on or off. The lamp goes back to normal touch by
itself when the USB link leaves the configured state, and after 10 s without
any command, as when the app stopped without a word. The app keeps the mode
with a command every 3 s.

During an alert the touch does nothing, in both modes: no color change and
no power change. A press still held when the alert ends does nothing
either, not even at its release.

## Host control (USB HID)

- The host reads the state and can set any subset of: power, color (a slot
  that holds a color), brightness mode, manual level (1 to 100 %) and touch
  mode.
- It defines the custom colors of slots 4 to 15, in linear light, and empties
  them. An emptied slot leaves the touch cycle, and a lamp that shows it
  takes the first color of the cycle.
- It sets the touch cycle: 1 to 9 slots that hold a color, none twice.
  Setting the cycle does not change the color shown.
- It starts an alert, 1 to 10 breaths over the lamp color
  ([Transitions](#transitions)).
- Every change, whatever its source (touch, host, automatic brightness, USB
  or the silence of the host), produces a state notification with an
  increasing sequence number.
- In `AUTO` mode the notification carries the level currently applied, so
  the app can show it next to its Auto switch.
- The host can shift the automatic curve ("brighter" or "darker", −50 to
  +50 %). Manual mode ignores it.
- The host can correct the touch sensitivity, from −50 % to +50 %: a positive
  value asks for a lighter touch. A change during a press waits for its
  release.
- The lamp forgets the custom colors, the cycle and both corrections when it
  restarts, so the app sends them again after each connection.
- The host can read diagnostics (resets, faults, stack margin, error
  counters, both corrections) and run the self-test below.

## Transitions

Color and power changes fade over about 300 ms. In `AUTO` mode the level
follows the room light on a logarithmic scale. The lamp follows a light
switched on or off in about 1.5 s, ignores a hand or a shadow passing in
front of the sensor, and follows small changes slowly, without ever
hunting.

An alert breathes over the lamp color, up from a dim lamp and down from a
bright one, for the number of breaths and the period the host asks, then
ends by itself and tells the host. The desktop app always asks for three
breaths. The lamp must be on: an alert never waits for it. Turning the lamp
off, a USB suspend and the self-test end it.

## USB states and LED power limits

| USB state | When | Lamp | Touch | LED current cap |
|---|---|---|---|---|
| `UNCONFIGURED` | Power-up, plain charger, hub without a host | Works locally | Active | 70 mA |
| `CONFIGURED` | Enumerated by a host | Normal | Active | 125 mA |
| `SUSPENDED` | Host asleep | Off at once | Ignored | 0 |

Suspend turns the LED off without a fade, because a suspended USB device
must drop its current within a few milliseconds. The touch and the light
sensor stop, and the chip slows down to 1 MHz. On resume, the lamp fades
back to its previous state, with normal touch.

The caps use the worst-case LED currents at 100 % duty: 86 mA for red,
63 mA for green and 125 mA for blue. When a color would exceed the cap,
the firmware scales the three channels down together, which keeps the hue.
A pure color fits within 125 mA. Warm white would draw about 140 mA at
100 %: it keeps about 90 % of its power with a host, and half without one.

## Self-test (production)

On request from the host, the lamp shows red, green, blue and warm white
for 1.5 s each, checks that the light sensor answers and that the idle
touch reading is plausible, then glows blue and waits up to 10 s for a
touch on the lid; a finger already on the lid must lift first. It then
restores its state and reports the results. A person checks the colors,
and the touch that ends the test does not change the lamp. During the
test the touch does nothing, and the lamp refuses the host's changes
("busy"). A USB suspend cuts the test short, and the lamp reports it
when the host wakes up.

## Out of scope

Any memory across restarts: the lamp stores no color, cycle or correction.
Integrations and calls run in the desktop app, which chooses the lamp color
through the same commands as any host.

## Test scenarios

Run them on a board connected to a computer, with the desktop app open.

1. Plug in: the lamp shows green; the app shows it on, green, in automatic
   brightness.
2. Short press three times: orange, red, green.
3. Set warm white from the app, then short press: green.
4. Long press while on: the lamp turns off at 2 s with the finger still
   down. Releasing does nothing more.
5. Short press while off: nothing happens.
6. Long press while off: the lamp turns on at 2 s with the last color.
7. Leave a phone on the lid for 15 s: one action at 2 s, then the lamp
   ignores the contact. Removing the phone does nothing, and the touch
   works again afterwards.
8. In the app, switch to manual at 30 %: the lamp dims. Switch back to auto:
   the level shown next to the switch follows the room light, and the
   slider sets the correction again.
9. Put the computer to sleep: the lamp turns off and the touch does
   nothing. Wake it up: the previous state comes back.
10. Plug the lamp into a phone charger: it works on its own and the LED
    current stays at or below 70 mA.
11. Every change, from the touch or from the app, shows up in the app with
    an increasing sequence number.
12. In auto mode, set the brightness correction to +30 % then −30 %: the
    applied level rises then falls by about that much.
13. Run the self-test: four colors, then blue; touch the lid. The result
    shows every check passed, and the lamp is back in its previous state.
14. In the app, create a custom color and choose it: the lamp shows it.
    Short press: the lamp goes to the first color of the touch cycle.
15. Set the touch cycle to red, then the custom color: short presses
    alternate between them.
16. Delete the custom color while the lamp shows it: the lamp fades to the
    first color of the cycle.
17. Follow an integration in the app: short presses do nothing, and a long
    press still turns the lamp off and on. End the app's process without
    quitting: after about 10 s, a short press changes the color again.
18. Start an alert from a plugin: three breaths, then the lamp stops by
    itself. Short and long presses during the breaths do nothing, and a
    press held past the end of the alert does nothing at its release.
19. Turn the lamp off and ask for an alert: nothing happens.
20. Set the touch sensitivity to +50 %: a light tap is enough, and a hand
    held 1 cm above the lid can count as a long press. At −50 %, light taps
    often miss.
21. Quit the app from the popup while the lamp shows a custom color: the
    lamp fades to green, short presses step through green, orange and red
    again, and the touch has its factory sensitivity.
