# Plugins

A plugin lets another program drive the lamp through the desktop app: pick
its color while the lamp follows it, and play an alert. Every integration of
Disled is a plugin, and the app ships none. This page is for people who
write one.

A plugin is a folder with a `plugin.json` file. It talks to Disled in one of
two ways:

- **With events**: a program that Disled does not run, such as Claude Code
  ([its plugin](../examples/plugins/claude-code/)), sends JSON to a local
  address, and rules of the manifest turn each event into a state or an
  alert. Disled runs nothing of such a plugin.
- **As a program**: Disled runs the command of the manifest and reads one
  notification per line on its output.

A plugin may do both.

## Where plugins live

Each plugin is a folder named by its id in the `plugins` folder of the app
configuration:

| System | Folder |
|---|---|
| Windows | `%APPDATA%\io.github.oui-dev.disled\plugins` (roaming profile) |
| macOS | `~/Library/Application Support/io.github.oui-dev.disled/plugins` |
| Linux | `$XDG_CONFIG_HOME/io.github.oui-dev.disled/plugins`, `~/.config/…` by default |

**Open the plugins folder** in the integrations window (mode menu of the
popup › **Manage integrations**) opens it. **Import a plugin** takes a zip
file whose `plugin.json` sits at its root or in its only folder. Folders
whose name starts with a dot are ignored. Under `tauri dev`, the app uses
the same folder as the installed app, since both have the same identifier.

On Windows the folder follows the roaming profile, and uninstalling with
"delete the app data" also deletes the plugins.

## Approval

A plugin arrives disabled. Turning it on asks a question that lists what it
will be able to do, taken from its manifest only. Disled keeps the
fingerprint of the files the user approved: if any file changes, the plugin
turns off and asks again. A plugin therefore never writes into its own
folder; it gets a data folder of its own (see "Programs").

The fingerprint is the SHA-256 of one line per file, sorted by path: the
path inside the folder with `/`, a NUL byte, the size in decimal, a NUL
byte, the SHA-256 of the content in hexadecimal, and a newline. Links and
junctions are refused, and so is a plugin of more than 5 000 files or
100 MiB.

## `plugin.json`

Version 1 of the plugin API. Unknown fields are ignored, so that version 1
reads richer manifests. A manifest that breaks a rule makes the plugin
unreadable, and the integrations window gives the reason.

| Field | Type | Required | Rule |
|---|---|---|---|
| `api` | integer | yes | `1`. A larger value makes the plugin unreadable. |
| `id` | text | yes | `^[a-z0-9][a-z0-9-]{0,31}$`, the name of its folder |
| `name` | text | yes | 1 to 32 characters |
| `version` | text | yes | SemVer |
| `author` | text | yes | 1 to 40 characters, shown after "by" |
| `icon` | text | no | one of the icons below, `puzzle` otherwise |
| `description` | localized text | yes | 300 characters at most |
| `capabilities` | list | yes | among `run`, `color`, `alert`, `events`; at least `run` or `events` |
| `states` | list | with `color` | 1 to 8 states |
| `idle` | state id | no | the state shown when no key is alive |
| `events` | object | with `events` only | `from`, `key`, `rules` |
| `run` | object | with `run` only | the command, by system |
| `note` | object | no | the installation note |

A localized text is a string, or an object by language such as
`{ "fr": "…", "en": "…" }`. Disled takes the language of the app, else
English, else French.

Icons: `terminal`, `timer`, `bot`, `code`, `git-branch`, `calendar`, `bell`,
`mail`, `message-square`, `music`, `headphones`, `coffee`, `zap`, `puzzle`.
A plugin never brings an image or HTML: every text shows as plain text.

### Capabilities

| Capability | What the approval says | What it allows |
|---|---|---|
| `run` | "Run a program on this computer, with your rights:" and the command | Disled runs the command of the manifest |
| `color` | "Choose the lamp color, when the lamp follows it" | the plugin joins the integrations of the mode menu |
| `alert` | "Make an alert on the lamp" | the plugin may ask for an alert |
| `events` | "Receive events from …, locally" | a local address and a token |

A plugin without `run` also gets "Nothing else: no program, no Internet". A
program, on the other hand, can do anything that the user can.

### States

A state has an `id` (`^[a-z0-9-]{1,32}$`), a localized `label` (40
characters at most), a default `color` among `green`, `orange`, `red` and
`warm`, a `priority` from 1 to 9 (1 by default) and an optional `ttl`, its
time to live in seconds, from 1 to 86 400. The user can give each state
another color in the integrations window.

States are kept by key, such as one key for each session of a program. The
plugin shows the state of its live key of the highest priority, the most
recent among equals. A key dies when its time to live runs out without a new
event. Without a live key the plugin shows its `idle` state, once it has
heard of something, and before that no state: the lamp keeps its color. A
plugin keeps 32 keys at most; past that, the key used least recently goes.

## Events

```json
"events": {
  "from": "My tool",
  "key": "/session",
  "rules": [
    { "when": { "/event": "end" }, "clear": true },
    { "when": { "/event": ["question", "permission"] }, "state": "waiting", "alert": true },
    { "when": { "/event": "done", "/background": { "nonEmpty": true } }, "state": "busy" },
    { "when": { "/event": "done" }, "state": "idle" }
  ]
}
```

The program sends each event as JSON in a POST request to
`http://127.0.0.1:47831/v1/hooks/<id>`, with the token of the plugin in the
`X-Disled-Token` header. The installation note of the plugin carries both:
`{{url}}` and `{{token}}` in its block become the address and the token.

- `from` names the program, in the approval and in the window.
- `key` is a JSON pointer (RFC 6901) to the value that tells the sources
  apart. Without it, or when the value is missing, the key is `default`.
- `when` holds conditions that must all be true. Each key is a JSON pointer;
  its value is a text, a number or a boolean (equal), a list (one of them),
  `{ "nonEmpty": true }` (a text or a list with something in it) or
  `{ "exists": false }`.
- The actions are `state` (the key takes that state), `alert`, alone or
  with `state`, `clear` (the key leaves) and `ignore`.
- The first rule that matches wins. An event that none matches does nothing.
- With `state`, `alert: true` alerts only when the key really changes state:
  a repeated event does not alert again.
- 64 rules at most, 8 conditions per rule.

The server answers at once with `204` and an empty body, then handles the
event. Its other answers are short texts: `401` for a wrong token, `403` for
a request that names another host or carries an `Origin` header, as every
browser request does, `404` for an unknown plugin, `405` for another method,
`413` past 8 MiB, `400` for a body that is not JSON, and `503` when 256
events already wait. A plugin turned off gets its events dropped with a
`204`, so that its program shows no error.

Disled reads only the fields that the rules name, then drops the event:
nothing of it is kept or written to a log.

The server listens on `127.0.0.1` only, and only while an approved plugin
has events. If another program holds port 47831, Disled tries again every
30 s, and the plugin says so.

### Behind a proxy

A program that sends its events through an HTTP proxy, set with
`HTTPS_PROXY` or `HTTP_PROXY`, must not send `127.0.0.1` to that proxy. If
the events of a plugin never arrive behind a proxy, add `127.0.0.1` to
`NO_PROXY` in the environment of that program.

## Programs

```json
"run": { "default": ["node", "index.js"], "windows": ["node.exe", "index.js"] }
```

Disled takes the command of the system, `windows`, `macos` or `linux`, else
`default`. A first argument that holds `/` or `\` is a path inside the
plugin folder, which it may not leave. Otherwise it is a program name looked
for on the `PATH`, then, on macOS, in `/opt/homebrew/bin` and
`/usr/local/bin`, and on macOS and Linux in `~/.local/bin`: an app started
from the Finder does not get the `PATH` of the shell. `.bat` and `.cmd`
files are refused.

The program runs in the plugin folder, with these variables:

| Variable | Value |
|---|---|
| `DISLED_API` | `1` |
| `DISLED_PLUGIN_ID` | the id of the plugin |
| `DISLED_PLUGIN_DATA` | a folder of its own for its data, outside the plugin folder |
| `DISLED_LANG` | `fr` or `en` |

It writes one JSON-RPC 2.0 notification per line on its standard output, as
the stdio transport of MCP does:

```json
{"jsonrpc":"2.0","method":"state","params":{"state":"work"}}
{"jsonrpc":"2.0","method":"state","params":{"state":"work","key":"timer-2"}}
{"jsonrpc":"2.0","method":"clear","params":{"key":"timer-2"}}
{"jsonrpc":"2.0","method":"clear"}
{"jsonrpc":"2.0","method":"alert"}
```

`state` without `key` sets the key `default`; `clear` without `key` clears
every key. A line holds 64 KiB at most, in UTF-8; any other line is ignored.
The standard error output is the log of the plugin, with the lines Disled
refused: `plugins/<id>.log` in the log folder of the app, 256 KiB at most,
then once more in `<id>.log.1`.

Disled writes nothing on the standard input. When it closes, the program
stops: Disled closes it when the plugin turns off or the app quits, waits 2
seconds, then ends the program and every process it started. A program that
crashes starts again after 1, 5, 30, then 60 seconds; at its fifth crash in
ten minutes it stays stopped until the user turns the plugin off and on.

On macOS, a binary that Archive Utility extracted keeps the quarantine flag,
and Gatekeeper may block it; an import by Disled adds no quarantine, and a
script run by `node` is not concerned. On Windows, Smart App Control may
block an unsigned binary.

## The installation note

| Field | Content |
|---|---|
| `note.text` | a localized list of paragraphs, 500 characters each at most. A passage between backquotes shows as code; nothing else is read: no HTML, no Markdown, no link. |
| `note.file` | the name of the file the block goes into, such as `settings.json` |
| `note.block` | the block to copy: a text, or JSON, shown with 2 spaces of indentation. 16 KiB at most. |

**Copy** puts the filled block in the clipboard. For a plugin with events,
the window adds that Disled never changes the settings of the program: the
user pastes the block. The note folds at the first event.

## Alerts

An alert is three soft breaths of the lamp, which stop by themselves. Only a
plugin with `alert`, approved and turned on, plays one, in every mode. There
is no alert while the lamp is off, and no queue: a request that cannot play
is lost. A request during an alert is ignored, and a plugin plays one alert
every 10 seconds at most. The touch of the lid does nothing during an alert.

## An example

A program plugin, for a timer:

```json
{
  "api": 1,
  "id": "pomodoro",
  "name": "Pomodoro",
  "version": "1.2.0",
  "author": "you",
  "icon": "timer",
  "description": { "en": "Do not disturb while the timer runs, Available during breaks." },
  "capabilities": ["run", "color", "alert"],
  "states": [
    { "id": "focus", "label": { "en": "Focus" }, "color": "red", "priority": 2 },
    { "id": "break", "label": { "en": "Break" }, "color": "green" }
  ],
  "run": { "default": ["node", "index.js"] }
}
```

`index.js` writes `{"jsonrpc":"2.0","method":"state","params":{"state":"focus"}}`
when a session begins, the same with `break` at the pause, and
`{"jsonrpc":"2.0","method":"alert"}` when it ends.

A whole plugin with events, for Claude Code, is in
[examples/plugins/claude-code](../examples/plugins/claude-code/): its
states, its rules, and its installation note with the block of hooks.
