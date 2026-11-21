# Plugins

A plugin lets another program drive the Disled lamp through the desktop
app: choose its color while the lamp follows the plugin, play an alert, and
read the lamp, its color, its brightness and the presses on its lid. Every
integration of Disled is a plugin. This section is for people who write
one; the [user guide](../user-guide.md#integrations) explains how to use
them.

A plugin is a folder with a [`plugin.json`](manifest.md) file. It talks to
Disled in one of two ways, or both:

- **As a program** ([programs](programs.md)): Disled runs the command of
  the manifest, reads one notification per line on its output, and can
  write the state of the lamp on its input.
- **With events** ([events](events.md)): a program that Disled does not
  run, such as Claude Code, sends JSON to a local address, and rules of the
  manifest turn each event into a state or an alert. Disled runs nothing
  of such a plugin.

| Page | What it covers |
|---|---|
| [Manifest](manifest.md) | `plugin.json`, field by field, and its [JSON Schema](plugin.schema.json) |
| [States and alerts](states.md) | how the states of a plugin become the color of the lamp |
| [Programs](programs.md) | the command, its environment, its output, its input, its life |
| [Reading the lamp](lamp.md) | the `lamp` capability: the state of the lamp and its presses |
| [Events](events.md) | the rules, the local address and its answers |
| [Sharing](sharing.md) | the catalog, and how to publish a plugin in it |
| [Changes](changelog.md) | what each version of the app adds to the API |

## A first plugin

This plugin switches between Available and Busy at each press on the lid,
and writes what the lamp shows in its log. It needs Node.js.

1. Open the integrations window (mode button of the popup, **Manage
   integrations**), then **Open the plugins folder**.
2. Make a folder `hello` there, with these two files.

`plugin.json`:

```json
{
  "$schema": "https://raw.githubusercontent.com/Oui-Dev/Disled/main/docs/plugins/plugin.schema.json",
  "api": 1,
  "id": "hello",
  "name": "Hello",
  "version": "0.1.0",
  "author": "you",
  "icon": "zap",
  "description": "Busy or available, one press on the lid at a time.",
  "capabilities": ["run", "color", "lamp"],
  "states": [
    { "id": "available", "label": "Available", "color": "green" },
    { "id": "busy", "label": "Busy", "color": "red" }
  ],
  "run": { "default": ["node", "index.js"] }
}
```

`index.js`:

```js
const readline = require('node:readline');

let busy = false;
const send = (method, params) =>
  process.stdout.write(`${JSON.stringify({ jsonrpc: '2.0', method, params })}\n`);

send('state', { state: 'available' });
readline.createInterface({ input: process.stdin }).on('line', (line) => {
  const { method, params } = JSON.parse(line);
  if (method === 'lamp' && params.connected) {
    console.error(`lamp ${params.on ? 'on' : 'off'}, ${params.color.name ?? params.color.hex}`);
  } else if (method === 'touch') {
    busy = !busy;
    send('state', { state: busy ? 'busy' : 'available' });
  }
});
```

3. In the integrations window, turn **Hello** on and accept what it asks.
4. In the mode menu of the popup, choose **Hello**. A short press on the
   lid now switches between Available and Busy, and the log of the plugin
   (`plugins/hello.log` in the log folder of the app) tells what the lamp
   shows at every change.

Presses reach a plugin with a lamp on firmware 2.1.0 or later, which the
app installs ([user guide](../user-guide.md#updates)).

## While you write it

Disled keeps the fingerprint of the files that the user approved: once a
file of the plugin changes, the plugin turns off. Turn it on again in the
integrations window, which asks again, to run the new files. The folder is
read again whenever the window gets the focus.

- Add `$schema` to `plugin.json`, as above: editors that read JSON Schema,
  such as VS Code, then complete the fields and underline the mistakes.
- A manifest that breaks a rule makes the plugin unreadable, and the
  integrations window gives the field and the reason.
- What the program writes on its error output goes to its log, with the
  lines of its output that Disled refused ([programs](programs.md#log)).
- A plugin never writes into its own folder, since that would change its
  fingerprint: `DISLED_PLUGIN_DATA` names a folder of its own for its data.

## Where plugins live

Each plugin is a folder named by its id in the `plugins` folder of the app
configuration:

| System | Folder |
|---|---|
| Windows | `%APPDATA%\io.github.oui-dev.disled\plugins` (roaming profile) |
| macOS | `~/Library/Application Support/io.github.oui-dev.disled/plugins` |
| Linux | `$XDG_CONFIG_HOME/io.github.oui-dev.disled/plugins`, `~/.config/…` by default |

**Open the plugins folder** in the integrations window opens it. **Import
a plugin** takes a zip file whose `plugin.json` sits at its root or in its
only folder, and the **Catalog** tab installs the plugins of the
[catalog](sharing.md). Folders whose name starts with a dot are ignored.
Under `tauri dev`, the app uses the same folder as the installed app, since
both have the same identifier.

On Windows the folder follows the roaming profile, and uninstalling with
"delete the app data" also deletes the plugins.

## Approval

A plugin arrives disabled. Turning it on asks a question that lists what it
will be able to do, taken from its manifest only
([capabilities](manifest.md#capabilities)). Disled keeps the fingerprint of
the files the user approved: if any file changes, the plugin turns off and
asks again. It turns back on by itself once its files are again the ones
approved, as when an edit is undone.

The fingerprint is the SHA-256 of one line per file, sorted by path: the
path inside the folder with `/`, a NUL byte, the size in decimal, a NUL
byte, the SHA-256 of the content in hexadecimal, and a newline. Links and
junctions are refused, and so is a plugin of more than 5 000 files or
100 MiB.
