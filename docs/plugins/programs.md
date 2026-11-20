# Programs

A plugin with the `run` capability is a program that Disled runs while the
plugin is on. It writes notifications on its output, one per line, as the
stdio transport of [MCP](https://modelcontextprotocol.io) does, and, with
the `lamp` capability, reads the state of the lamp on its input.

## The command

```json
"run": { "default": ["node", "index.js"], "windows": ["node.exe", "index.js"] }
```

Disled takes the command of the system, `windows`, `macos` or `linux`, else
`default`; a plugin without either for a system cannot run there. The first
argument is the program:

- With a `/` or a `\`, it is a path inside the plugin folder, which it may
  not leave, such as `bin/tool`.
- Otherwise it is a program name looked for on the `PATH`, then, on macOS,
  in `/opt/homebrew/bin` and `/usr/local/bin`, and on macOS and Linux in
  `~/.local/bin`: an app started from the Finder does not get the `PATH` of
  the shell. On Windows a name without extension is looked for as `.exe`,
  then `.com`.

`.bat` and `.cmd` files are refused. A program that cannot be found leaves
the plugin with "not found" in the integrations window; Disled looks for it
again when the folder is read again.

The program runs in the plugin folder, without a window, with these
variables:

| Variable | Value |
|---|---|
| `DISLED_API` | `1`, the version of the plugin API |
| `DISLED_PLUGIN_ID` | the id of the plugin |
| `DISLED_PLUGIN_DATA` | a folder of its own for its data, outside the plugin folder, which Disled creates |
| `DISLED_LANG` | `fr` or `en`, the language of the app |

A program never writes into the plugin folder: its fingerprint would
change, and the plugin would turn off ([approval](README.md#approval)).

## Output

The program writes one [JSON-RPC 2.0](https://www.jsonrpc.org/specification)
notification per line on its standard output, without `id`, since Disled
never answers:

```json
{"jsonrpc":"2.0","method":"state","params":{"state":"focus"}}
{"jsonrpc":"2.0","method":"state","params":{"state":"focus","key":"timer-2"}}
{"jsonrpc":"2.0","method":"clear","params":{"key":"timer-2"}}
{"jsonrpc":"2.0","method":"clear"}
{"jsonrpc":"2.0","method":"alert"}
```

| Method | Params | Effect |
|---|---|---|
| `state` | `state`, a state of the manifest, and `key`, `default` without one | the key takes the state ([states](states.md#keys)) |
| `clear` | `key`, or none | the key leaves, or every key without one |
| `alert` | none | an [alert](states.md#alerts), with the `alert` capability |

A line holds 64 KiB at most, in UTF-8. Any other line is ignored, and goes
to the log as `refused line`. A state that the manifest does not define,
or an alert without the `alert` capability, is ignored without a word.

## Input

Without the `lamp` capability, Disled writes nothing on the standard input
of the program. With it, the program reads one JSON-RPC 2.0 notification per
line: the state of the lamp first, then its changes and the presses on the
lid ([reading the lamp](lamp.md)).

In both cases, the end of the input tells the program to stop.

## Life

- Disled starts the program when the plugin turns on, and when the app
  starts with the plugin on.
- It stops it when the plugin turns off or is removed, when its files
  change, and when the app quits: Disled closes its input, waits 2 seconds,
  then ends the program and every process it started. A job object on
  Windows, a process group elsewhere, and on Linux the death signal of its
  parent, take its children down with it.
- A program that ends by itself counts as a crash, and starts again after
  1, 5, 30, then 60 seconds. At its fifth crash in ten minutes it stays
  stopped until the user turns the plugin off and on.
- A program that stops, whatever the reason, loses its keys.

The integrations window shows how the program fares: running, stopped with
the last line of its error output, not found, or stopped for good.

## Log

The standard error output of the program is its log, with the lines of its
output that Disled refused: `plugins/<id>.log` in the log folder of the
app, 256 KiB at most, then once more in `<id>.log.1`.

| System | Log folder |
|---|---|
| Windows | `%LOCALAPPDATA%\io.github.oui-dev.disled\logs` |
| macOS | `~/Library/Logs/io.github.oui-dev.disled` |
| Linux | `$XDG_DATA_HOME/io.github.oui-dev.disled/logs`, `~/.local/share/…` by default |

## Systems

- On macOS, a binary that Archive Utility extracted keeps the quarantine
  flag, and Gatekeeper may block it; an import by Disled adds no
  quarantine, and a script run by `node` is not concerned.
- On Windows, Smart App Control may block an unsigned binary.
- The plugins of the [catalog](sharing.md) hold sources only, run by a
  program such as `node` or `python3` that the user installs.
