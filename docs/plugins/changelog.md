# Changes of the plugin API

The plugin API keeps version 1 (`"api": 1`) as long as it grows in
compatible ways: a later app reads the plugins written for an earlier one,
and an earlier app ignores the fields and capabilities that it does not
know. Version 2 will come only with a change that breaks a plugin.

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
