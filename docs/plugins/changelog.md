# Changes of the plugin API

The plugin API keeps version 1 (`"api": 1`) as long as it grows in
compatible ways: a later app reads the plugins written for an earlier one,
and an earlier app ignores the fields and capabilities that it does not
know. Version 2 will come only with a change that breaks a plugin.

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
