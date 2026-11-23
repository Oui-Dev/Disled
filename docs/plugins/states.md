# States and alerts

A plugin with the `color` capability announces states, such as "working"
or "waiting", each with a color. While the lamp follows the plugin, in the
mode menu of the popup, it shows the color of the state of the plugin. A
plugin with `alert` can also make the lamp breathe, whatever it follows.

## States

```json
"states": [
  { "id": "waiting", "label": { "en": "Waits for you", "fr": "Vous attend" }, "color": "orange", "priority": 3, "ttl": 1800 },
  { "id": "working", "label": "Working", "color": "red", "priority": 2, "ttl": 300 },
  { "id": "done", "label": "Done", "color": "green" }
],
"idle": "done"
```

| Field | Rule |
|---|---|
| `id` | `^[a-z0-9-]{1,32}$`, unique in the plugin |
| `label` | a localized text of 40 characters at most, which the popup shows on the card of the integration |
| `color` | its default color: `green`, `orange`, `red`, `warm_white`, or a color of the plugin's own, `{ "x": 0.17, "y": 0.7 }` ([colors](#colors)) |
| `priority` | 1 to 9, 1 by default |
| `ttl` | its time to live, 1 to 86 400 seconds: a key keeps the state that long without a new event |

A plugin has 1 to 8 states. The user can give each state another color,
built in or of their own, in the integrations window, and give it back the
color of the plugin. While the lamp follows the plugin, a new state changes
the color of the lamp, never its power: a lamp turned off stays off. The
lamp changes color at most 4 times a second: a state that comes sooner
waits for its turn, in the place of the one that waited, so that the lamp
always ends on the last state.

## Colors

A built-in color goes by its name: `green`, `orange`, `red` and
`warm_white`, the colors that the lamp shows best, its own LEDs for green
and red. A color of the plugin's own is a CIE 1931 chromaticity, `x` from
0 to 1, `y` above 0, `x + y` at most 1, as Philips Hue, Zigbee and Matter
give them: `{ "x": 0.6892, "y": 0.3105 }` is the red LED, and
`{ "x": 0.3127, "y": 0.329 }` the white of the lamp, D65. The lamp shows
the light of that very color, as far as its LEDs go: a color beyond them
comes as near as it can, with the same hue, less saturated. The brightness
of the lamp stays the user's.

## Keys

A plugin keeps its states by key, such as one key for each session of a
program, so that several sources share the lamp:

- A program sets the state of a key, or of the key `default` without one
  ([programs](programs.md#output)). The rules of a plugin with events take
  the key from the event ([events](events.md#rules)).
- The plugin shows the state of its live key of the highest priority, the
  most recent among equals.
- A key whose state has a `ttl` dies when it runs out without a new state
  for that key. A `clear` makes a key leave at once.
- Without a live key, the plugin shows its `idle` state once it has heard
  of something, and before that no state: the lamp keeps its color.
- A plugin keeps 32 keys at most; past that, the key used least recently
  goes.

A program that stops loses its keys, and so does a plugin turned off.

## Alerts

An alert is three soft breaths of the lamp over its color, which stop by
themselves after 4.5 s. Only a plugin with `alert`, approved and turned on,
plays one, whether or not the lamp follows it:

- There is no alert while the lamp is off, and no queue: a request that
  cannot play is lost.
- A request during an alert is ignored, and a plugin plays one alert every
  10 seconds at most.
- During an alert, the touch changes nothing on the lamp. A plugin with
  `lamp` still hears of a short press ([reading the lamp](lamp.md#presses)),
  and the breaths go on.

A program asks for an alert with a notification
([programs](programs.md#output)); a rule asks for one with `alert`
([events](events.md#rules)).
