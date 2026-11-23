# The manifest: `plugin.json`

Every plugin has a `plugin.json` at the root of its folder: who made it,
what it can do, its states, how it talks to Disled and its installation
note. This page describes version 2 of the plugin API, which Disled 3.0
and later read ([changes](changelog.md)).

A manifest that breaks a rule makes the plugin unreadable, and the
integrations window gives the field and the reason. Unknown fields are
ignored, so that the app reads manifests written for a later version.

## JSON Schema

[`plugin.schema.json`](plugin.schema.json) describes the manifest for
editors and checks. Name it in the manifest:

```json
"$schema": "https://raw.githubusercontent.com/Oui-Dev/Disled/main/docs/plugins/plugin.schema.json"
```

The schema follows the rules of this page, with two differences, which a
test of the app keeps true:

- It refuses the fields and values that the app ignores, such as an
  unknown field, icon or capability: these are typos most of the time.
- It cannot see the links between fields: an `idle` state, or a rule
  toward a state, that the manifest does not define, or two states of the
  same id. The app refuses them.

## Fields

| Field | Type | Required | Rule |
|---|---|---|---|
| `api` | integer | yes | `2`. Any other value makes the plugin unreadable: a plugin of API 1 needs an update ([changes](changelog.md)). |
| `id` | text | yes | `^[a-z0-9][a-z0-9-]{0,31}$`, the name of its folder |
| `name` | text | yes | 1 to 32 characters |
| `version` | text | yes | [SemVer](https://semver.org), such as `1.2.0` |
| `author` | text | yes | 1 to 40 characters, shown after "by" |
| `icon` | text | no | one of the [icons](#icons), `puzzle` otherwise |
| `description` | localized text | yes | 300 characters at most |
| `capabilities` | list | yes | among `run`, `color`, `alert`, `events`, `lamp` ([capabilities](#capabilities)) |
| `states` | list | with `color` | 1 to 8 [states](states.md) |
| `idle` | state id | no | with `color`: the state shown when no key is alive ([states](states.md#keys)) |
| `events` | object | with `events` only | `from`, `key`, `rules` ([events](events.md)) |
| `run` | object | with `run` only | the command, by system ([programs](programs.md)) |
| `note` | object | no | the [installation note](#the-installation-note) |

A **localized text** is a string, or an object by language such as
`{ "fr": "…", "en": "…" }`. Disled takes the language of the app, else
English, else French, else the first one.

### Icons

`terminal`, `timer`, `bot`, `code`, `git-branch`, `calendar`, `bell`,
`mail`, `message-square`, `music`, `headphones`, `coffee`, `zap`,
`puzzle`: the [Lucide](https://lucide.dev/icons/) icons of these names,
which the app bundles. A plugin never brings an image or HTML: every text
shows as plain text.

## Capabilities

What a plugin can do comes from its manifest only, and the approval lists
it, in these words:

| Capability | What the approval says | What it allows |
|---|---|---|
| `run` | "Run a program on this computer, with your permissions:" and the command | Disled runs the command of `run` ([programs](programs.md)) |
| `color` | "Choose the lamp color, when the lamp follows it" | the plugin joins the integrations of the mode menu, with its `states` |
| `alert` | "Show an alert on the lamp" | the plugin may ask for an [alert](states.md#alerts) |
| `lamp` | "Know the color and brightness of the lamp, and when you touch it" | its program reads the lamp on its input ([reading the lamp](lamp.md)) |
| `events` | "Receive events from …, locally", with `events.from` | a local address and a token ([events](events.md)) |

A plugin has `run` or `events`, or both, and `lamp` needs `run`. A plugin
without `run` also gets "Nothing else: no program, no Internet". A program,
on the other hand, can do anything that the user can.

The app ignores a capability it does not know, and never grants it.

## The installation note

Some plugins need a step of the user, such as a block to paste into the
settings of their program. The note shows it in the details of the
plugin:

| Field | Content |
|---|---|
| `note.text` | a localized list of paragraphs, 500 characters each at most. A passage between backquotes shows as code; nothing else is read: no HTML, no Markdown, no link. |
| `note.file` | the name of the file the block goes into, such as `settings.json`, 64 characters at most |
| `note.block` | the block to copy: a text, or JSON, shown with 2 spaces of indentation. 16 KiB at most. |

For a plugin with events, `{{url}}` and `{{token}}` in the block become its
address and its token ([events](events.md#the-address)). **Copy** puts the
filled block in the clipboard. The window adds that Disled never changes
the settings of the program: the user pastes the block. The note folds at
the first event, and unfolds again with a new token.

## Examples

- [A first plugin](README.md#a-first-plugin): a program that reads the
  presses on the lid, with `run`, `color` and `lamp`.
- The [Claude Code plugin](https://github.com/Oui-Dev/Disled-plugins/tree/main/plugins/claude-code)
  of the [catalog](sharing.md): events, rules and an installation note,
  with `color`, `alert` and `events`.
