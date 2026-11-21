# Events

A plugin with the `events` capability gets its events from a program that
Disled does not run, such as Claude Code: that program sends JSON to a
local address, and rules of the manifest turn each event into a state or
an alert. Disled runs nothing of such a plugin.

## Rules

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

- `from` names the program, a localized text of 40 characters at most, in
  the approval and in the window.
- `key` is a [JSON pointer](https://www.rfc-editor.org/rfc/rfc6901) to the
  value that tells the sources apart, such as a session
  ([keys](states.md#keys)). Without it, or when the value is missing, the
  key is `default`.
- `when` holds conditions that must all hold, 8 at most. Each key is a JSON
  pointer, `""` for the whole event; its value is a text, a number or a
  boolean (equal), a list of them (one of them), `{ "nonEmpty": true }` (a
  text or a list with something in it) or `{ "exists": false }`.
- The actions are `state` (the key takes that state, one of the manifest),
  `alert` alone or with `state`, `clear` (the key leaves) and `ignore`.
- The first rule that matches wins. An event that none matches does
  nothing. 64 rules at most.
- With `state`, `alert: true` alerts only when the key really changes
  state: a repeated event does not alert again.

Disled reads only the fields that the rules name, then drops the event:
nothing of it is kept or written to a log.

## The address

The program sends each event as JSON in a `POST` request to
`http://127.0.0.1:47831/v1/hooks/<id>`, with the token of the plugin in the
`X-Disled-Token` header:

```sh
curl -X POST http://127.0.0.1:47831/v1/hooks/my-tool \
  -H "X-Disled-Token: dl_…" -H "Content-Type: application/json" \
  -d '{"event":"done","session":"a1"}'
```

Disled makes the token when the user approves the plugin, and the details
of the plugin offer to make a new one, which ends the old one. The
[installation note](manifest.md#the-installation-note) carries both:
`{{url}}` and `{{token}}` in its block become the address and the token.

The server answers at once with `204` and an empty body, then handles the
event. Its other answers are short texts:

| Status | When |
|---|---|
| `204` | the event is taken, or dropped because the plugin is off, so that its program shows no error |
| `400` | the body is not JSON |
| `401` | wrong or missing token |
| `403` | a request that names another host, or carries an `Origin` header, as every browser request does |
| `404` | no approved plugin with events of that id |
| `405` | another method than `POST` |
| `408` | a body that takes more than 5 s to arrive |
| `413` | a body over 8 MiB |
| `503` | 256 events already wait |

The server listens on `127.0.0.1` only, and only while an approved plugin
has events. If another program holds port 47831, Disled tries again every
30 s, and the integrations window says so.

## Behind a proxy

A program that sends its events through an HTTP proxy, set with
`HTTPS_PROXY` or `HTTP_PROXY`, must not send `127.0.0.1` to that proxy. If
the events of a plugin never arrive behind a proxy, add `127.0.0.1` to
`NO_PROXY` in the environment of that program.

## An example

The [Claude Code plugin](https://github.com/Oui-Dev/Disled-plugins/tree/main/plugins/claude-code)
of the catalog turns the hooks of Claude Code into three states: its
rules, and its installation note with the block of hooks.
