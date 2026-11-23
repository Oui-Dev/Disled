# Reading the lamp

A plugin with the `lamp` capability knows what the lamp shows, its color,
its brightness, whether it is on, and when someone presses the lid. Its
program reads it on its standard input, so `lamp` needs `run`
([programs](programs.md)). The approval says: "Know the color and
brightness of the lamp, and when you touch it". Only the programs that
Disled runs read the lamp: there is no address for other programs.

```json
"capabilities": ["run", "color", "lamp"]
```

## Notifications

The program reads one [JSON-RPC 2.0](https://www.jsonrpc.org/specification)
notification per line, without `id`: it answers none.

```json
{"jsonrpc":"2.0","method":"lamp","params":{"connected":true,"on":true,"color":{"slot":2,"name":"Do not disturb","x":0.6892,"y":0.3105},"brightness":{"mode":"auto","level":40},"source":"touch","follows":"hello"}}
{"jsonrpc":"2.0","method":"touch","params":{"press":"short"}}
```

- `lamp` comes first at every start of the program, then at every change
  of what it holds. The same state never comes twice in a row.
- `touch` comes at every short press on the lid ([presses](#presses)).
- A program ignores the methods and the fields that it does not know: later
  versions of the app may add some ([changes](changelog.md)).

## `lamp`

| Field | Value |
|---|---|
| `connected` | `true` while the app drives a lamp. Without one, the other fields but `follows` are missing. |
| `on` | the lamp is on |
| `color.slot` | the slot of the color: 0 to 3 for the built-in colors (green, orange, red, warm white), then the custom colors of the user, and 14 for a color of a plugin itself |
| `color.name` | the name the user sees, such as `Do not disturb` or a name of their own, in the language of the app, and the label of the state for a color of a plugin; `null` for a slot without one |
| `color.x`, `color.y` | the CIE 1931 chromaticity of the light of the lamp, to 4 decimals, as a plugin gives its own colors ([states](states.md#colors)) |
| `brightness.mode` | `auto`, after the light of the room, or `manual` |
| `brightness.level` | the level in use, in percent |
| `source` | what made the latest change: `touch`, the lid; `app`, the user in the app, a plugin or a call; `light`, the automatic brightness; `lamp`, the lamp itself, at its start or when the USB link changes |
| `follows` | the id of the integration the lamp follows, `null` in manual mode |

The color keeps its slot while the lamp is off: it is the one the lamp
shows when it lights again. The default names of the built-in colors are
those of the popup, in the language of the app, and a new `lamp` comes when
the user renames a color or changes the language. While the color editor of
the app previews a color on the lamp, the program reads the state the lamp
gets back after it.

In automatic brightness, `brightness.level` follows the room light: the
lamp tells of it at most every 500 ms.

A program tells whether the lamp follows its own plugin by comparing
`follows` with `DISLED_PLUGIN_ID`.

## Presses

A short press is a press on the lid released before 2 s. The program hears
of every one, whatever the lamp does with it:

| The lamp | What a short press does on the lamp | The program reads |
|---|---|---|
| in manual mode, on | the next color of the touch cycle | `touch`, then `lamp` with the new color and `"source":"touch"` |
| follows an integration | nothing | `touch` |
| off | nothing | `touch` |
| breathes an alert | nothing, and the breaths go on | `touch` |

A plugin can so use the lid as a button, for instance to start a timer or
to tell a program that you saw its alert, even while the lamp follows it.
A long press turns the lamp off or on: the program reads it as a `lamp`
with `"source":"touch"`, and no `touch`.

- Presses come only from a lamp with firmware 2.1.0 or later, which the
  app installs ([user guide](../user-guide.md#updates)). With an earlier
  one, no `touch` comes, and the rest works.
- During the self-test of the Diagnostics window, the touch belongs to the
  test: no `touch` comes.
- A press is news only while it is fresh: a program that does not run when
  it happens never reads it, and one that reads late gets the last 16.

## Reading the input

Read the input line by line, and leave when it ends: that is how Disled
stops the program. A program that reads nothing does no harm either:
while it does not read, the latest state takes the place of the one that
waits.

Node.js:

```js
const readline = require('node:readline');

readline.createInterface({ input: process.stdin })
  .on('line', (line) => {
    const { method, params } = JSON.parse(line);
    if (method === 'lamp' && params.connected) {
      console.error(`${params.on ? 'on' : 'off'}: ${params.color.name ?? `${params.color.x} ${params.color.y}`}`);
    } else if (method === 'touch') {
      console.error('a press on the lid');
    }
  })
  .on('close', () => process.exit(0));
```

Python:

```python
import json
import sys

for line in sys.stdin:
    message = json.loads(line)
    params = message.get("params", {})
    if message["method"] == "lamp" and params["connected"]:
        print(params["on"], params["color"]["name"], file=sys.stderr)
    elif message["method"] == "touch":
        print("a press on the lid", file=sys.stderr)
```

Write on the output with a flush at every line, as `print(…, flush=True)`
does in Python: a program whose output waits in a buffer changes the lamp
late.
