# Changes of the plugin API

The plugin API keeps version 2 (`"api": 2`) as long as it grows in
compatible ways: a later app reads the plugins written for an earlier one,
and an earlier app ignores the fields and capabilities that it does not
know. Version 3 will come only with a change that breaks a plugin.

## App 3.0: version 2

The colors become CIE 1931 chromaticities, as Philips Hue, Zigbee and
Matter give them, and the app reads version 2 only: a plugin of version 1
shows as unreadable until its update, which the catalog offers.

- `"api": 2`.
- The color of a state ([states](states.md#colors)): `warm` becomes
  `warm_white`, and a state may have a color of the plugin's own,
  `{ "x": 0.17, "y": 0.7 }`.
- The `lamp` notifications ([reading the lamp](lamp.md)) give `color.x`
  and `color.y`, the chromaticity of the light of the lamp, in the place
  of `color.hex`. Slot 14 holds a color of a plugin itself, named after
  the label of its state.
- While the lamp follows a plugin, its color changes at most 4 times a
  second: a state that comes sooner waits for its turn, the last one only.

To move a plugin to version 2: `"api": 2`, `warm_white` for `warm`, and
`color.x` and `color.y` for `color.hex`.

## App 2.2

- The `color.hex` of the built-in colors in the `lamp` notifications
  ([reading the lamp](lamp.md)) follows the colors of the lamp: green
  `#00F57A`, orange `#FF6F00`, red `#FF001C` and warm white `#FFAD59`, as
  the app now shows them.
- The `color.hex` of a custom color is always the color the lamp shows,
  its largest channel at `FF`: a color saved with a darker code is
  brightened once, to the color the lamp already showed.

## App 2.1

- The `lamp` capability: the program of the plugin reads the state of the
  lamp and the presses on its lid on its standard input
  ([reading the lamp](lamp.md)). The presses need firmware 2.1.0 or later.
  An app 2.0 ignores the capability: the program then reads nothing.
- The catalog of plugins, in a tab of the integrations window
  ([sharing](sharing.md)).
- [`plugin.schema.json`](plugin.schema.json), the JSON Schema of the
  manifest.

## App 2.0

The first version of the API: the manifest, the `run`, `color`, `alert` and
`events` capabilities, the states by key, the local address of the events,
the notifications on the output of a program, and the approval by
fingerprint.
