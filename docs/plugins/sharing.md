# Sharing a plugin

The plugins that people share live in the catalog,
[Oui-Dev/Disled-plugins](https://github.com/Oui-Dev/Disled-plugins): one
folder a plugin, under the MIT license, which its maintainers review at
every version. Disled 2.1 and later install them from the **Catalog** tab
of the integrations window.

## The Catalog tab

The tab reads the catalog when it opens, and only then: Disled reaches the
network for it at no other time. Each plugin shows its name, its author,
its version, its description and what it can do, with **Install**,
**Update to …** when the catalog has a later version than the one
installed, or **Installed**. **Sources** opens its folder on GitHub.

An install:

1. downloads the zip of the plugin, and checks its size and its SHA-256
   against the index;
2. unpacks it, and checks that it holds the plugin of that id with the
   files of the index, by their [fingerprint](README.md#approval);
3. puts it in the plugins folder, in place of the plugin of the same id,
   whose token and colors stay;
4. asks for the approval, as for any plugin: the plugin stays off until the
   user accepts what it will be able to do.

Disled refuses an index that the catalog key did not sign: the
`index.json.sig` of the catalog, in the format of `tauri signer`, signs
`index.json`, its file name included. The catalog key is not the update
key of the app. Every text of the catalog shows as plain text, and its
icons come from the list that the app bundles.

## The index

The catalog publishes on GitHub Pages, at
`https://oui-dev.github.io/Disled-plugins/`:

| File | Content |
|---|---|
| `index.json` | `{ "schema": 1, "plugins": [...] }`, a plugin being `{ "manifest", "size", "sha256", "fingerprint" }` |
| `index.json.sig` | the signature of `index.json` with the catalog key |
| `zips/<id>-<version>.zip` | the files of the plugin, `plugin.json` at the root, and the license of the catalog |

`manifest` is the `plugin.json` of the plugin, which Disled reads as it
reads the manifests of the plugins folder: a plugin that this version of
the app cannot read, such as one of a later API, stays out of the tab.
`size` and `sha256` are those of the zip, `fingerprint` that of its files.
An index of a later `schema` asks for a later app.

## Publish a plugin

1. Write the plugin, and try it in the plugins folder
   ([a first plugin](README.md#a-first-plugin)).
2. Fork the catalog, and add its folder as `plugins/<id>/`, with a
   `README.md` for its users.
3. Open a pull request. Its checks run the manifest against the
   [schema](plugin.schema.json) and the rules of the catalog, and list what
   the plugin asks for; then a maintainer reads every file.
4. Once merged, the plugin shows in the Catalog tab of every Disled 2.1 and
   later, within minutes.

The rules, in short
([CONTRIBUTING.md](https://github.com/Oui-Dev/Disled-plugins/blob/main/CONTRIBUTING.md)
of the catalog):

- **MIT** for every plugin.
- **Sources only**, which the review reads: no binary, no minified code, no
  dependency to install.
- The command starts with a runtime that the user installs: `node`,
  `deno`, `bun`, `python3`, `python` or `pwsh`.
- The description and the README say what the plugin does beyond the lamp,
  the network above all.
- A new version for any change of its files: Disled offers an update only
  for a later version.

## Without the catalog

A plugin also travels as a zip file: **Import a plugin**, in the
integrations window, takes a zip whose `plugin.json` sits at its root or in
its only folder ([where plugins live](README.md#where-plugins-live)). Its
approval follows, as for a plugin of the catalog, but no one reviewed it:
only import plugins from people you trust.
