# Disled desktop app

The desktop app is a Tauri 2 app that lives in the tray, with a popup to
control the lamp, a Diagnostics window, two windows that update the lamp
and one for the integrations. The
[user guide](../docs/user-guide.md) describes what it does for its users,
and [docs/architecture.md](../docs/architecture.md#desktop-app) how it
works. This page is for working on it.

## Run it

You need Node.js 24 or 26 and Rust through `rustup`, at the pinned versions
([CONTRIBUTING.md](../CONTRIBUTING.md#before-you-start)), plus the Visual
Studio Build Tools with the C++ workload on Windows, or the Xcode command
line tools on macOS.

```sh
npm install
npm run tauri dev
```

The app finds the lamp by itself, by its USB IDs `1209:0001` and
`1209:0002`, and reconnects when you plug it back in. It needs no driver.
`tauri dev` never checks for updates by itself, and never turns on start at
login, which would register the debug executable. It shares the
configuration folder of the installed app, since both have the same
identifier: the same settings, colors and plugins
([docs/plugins.md](../docs/plugins.md#where-plugins-live)).

On Linux, the calls read the sound server through `libpulse.so.0`, which the
app loads when it starts, if the system has it (`libpulse0` on Debian and
Ubuntu). Nothing links to it, so the build needs no `libpulse-dev`, and
without it the app runs with no **Calls** group.

To try a lamp update from `tauri dev`, install wchisp on the PATH
(`cargo install wchisp@0.3.0 --locked`). The app offers only published
firmware signed with the update key, but a debug build also takes a local
file, unsigned. Start it with `DISLED_FIRMWARE_HEX` set to the path of a
`disled-fw-X.Y.Z.hex`, whose name gives the version, then use **Check for
updates** in the settings. Release builds ignore that variable.

## The udev rule on Linux

By default, only root may open a USB HID device on Linux. The `.deb`
installs a udev rule, `/usr/lib/udev/rules.d/70-disled.rules`
([source](src-tauri/linux/70-disled.rules)), that gives the logged-in user
access to the lamp, `1209:0001` and `1209:0002`, and to the WCH bootloader,
for `wchisp`. When the release firmware moves to the lamp's own PID,
`1209:D1ED`, a version of the app adds it to the rule first, and keeps the
other two for the lamps not moved yet. To run the app from source, install
the rule once by hand:

```sh
sudo cp src-tauri/linux/70-disled.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules && sudo udevadm trigger
```

## Front end

The front end is Vue 3 with [Nuxt UI](https://ui.nuxt.com) 4 through its
Vite plugin, without Nuxt, and uses its components wherever one fits,
charts included. The plugin imports the components by itself and writes
`src/components.d.ts`. A component name starts with its folder:
`lamp/LampColorPicker.vue` is `<LampColorPicker>`. Functions are imported
explicitly, through the `@/` alias of `src/`. Vite finds the Nuxt UI
components in use when it starts, and only those get their styles, so
restart `npm run tauri dev` after using one for the first time.

- **State.** One Pinia store per domain lives in `src/stores`, and each
  window starts the ones it shows (`src/App.vue`). The stores talk to the
  backend through `src/ipc`: typed wrappers of the Tauri commands and a
  typed map of the events.
- **Types from Rust.** ts-rs writes the types exchanged with the backend to
  `src/bindings` when the Rust tests run. After a change to one of those
  Rust types, run `npm run bindings`. The checks fail when the committed
  files are out of date ([CONTRIBUTING.md](../CONTRIBUTING.md#checks)).
- **Accessibility.** The windows aim at WCAG 2.2 AA, except for the
  contrast of the primary color (below).

### Design system

`src/assets/css/main.css` brings Tailwind, Nuxt UI, the self-hosted
Montserrat font and the token overrides. `src/assets/css/palettes.css`
holds the two palettes. `ink`, the neutral one, goes from the brand white
`#fefcf7`, the light background, to the brand black `#060907`, the dark
one. `brand`, the primary one, is built on the logo green `#2ecc71`.
Nothing checks the contrasts, so a change there must keep neutral text at
4.5:1 and the primary at about 3.8:1.

### Translations

The messages live in `src/locales/en.json` and `src/locales/fr.json`. The
windows read them with [vue-i18n](https://vue-i18n.intlify.dev), compiled
at build time. The backend reads the `tray` and `window` sections for the
tray menu and the window titles (`src-tauri/src/i18n.rs`), so these must
stay plain text. English is the reference: TypeScript rejects a French file
that lacks a message, and `cargo test` checks that both files have the same
keys. Errors from the libraries of the backend stay in English, inside a
translated sentence.

To add a language:

- **Messages.** `src/locales/<code>.json`, with every message of `en.json`.
- **Rust.** A variant of `Language` (`src-tauri/src/settings.rs`) and of
  `Locale` (`src-tauri/src/i18n.rs`). In `i18n.rs`, its file in `MESSAGES`,
  in the order of `Locale`, its arm in `Locale::of`, its system tag in
  `from_tag` and its keys in the test that compares the files. Then
  `npm run bindings`.
- **Windows.** In `src/i18n.ts`, its messages, number and date formats,
  plural rule when the default of vue-i18n does not fit, and Nuxt UI words
  (`nuxtUiLocales`). In `src/components/popup/PopupSettings.vue`, its entry
  in the language choices, named in that language.

## Layout

| Path                                | Content                                                                                                        |
| ----------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| `../crates/disled-protocol`         | Protocol codec (shared with the tools), tested against `protocol/vectors/v2.json`                              |
| `../crates/disled-flash`            | Signed firmware files and wchisp, run as a separate program (shared with the tools)                            |
| `src-tauri/src/device.rs`           | USB worker thread: discovery, reconnection, settings applied on INFO, heartbeat                                |
| `src-tauri/src/control.rs`          | What the app sets on the lamp: the source it follows, calls, alerts, Quit                                      |
| `src-tauri/src/colors.rs`           | Color names, custom colors, touch cycle, deletion fallbacks, share codes                                       |
| `src-tauri/src/plugins/`            | Plugins: manifests, approvals, rules and states, local server, tokens, programs                                |
| `src-tauri/src/calls/`              | Calls told by the microphone in use, one reader per system, and their debounce                                 |
| `src-tauri/src/settings.rs`         | Settings saved on the computer (store plugin) and the commands that apply them                                 |
| `src-tauri/src/i18n.rs`             | Languages: "System" resolved, the messages of the tray and of the window titles                                |
| `src-tauri/src/tray.rs`             | Tray icon (drawn at run time), menu and clicks                                                                 |
| `src-tauri/src/windows.rs`          | Popup placement, height and hiding, the other windows                                                          |
| `src-tauri/src/updates.rs`          | Update checks (start-up, then daily) and install                                                               |
| `src-tauri/src/firmware.rs`         | Lamp updates: signed firmware releases, download, erase, update steps                                          |
| `src-tauri/src/main.rs`             | Plugins, start-up, Tauri commands                                                                              |
| `src/App.vue`                       | One root for every window: its view, its stores, its color mode and language                                   |
| `src/views/`                        | One view per window: popup, Diagnostics, lamp update, reinstall, integrations                                  |
| `src/components/`                   | Their parts, by domain: `lamp/`, `colors/`, `popup/`, `diagnostics/`, `update/`, `integrations/`               |
| `src/stores/`                       | Pinia stores: lamp, settings, colors, control, plugins, telemetry, diagnostics, updates, firmware, errors      |
| `src/ipc/`                          | Typed Tauri commands and events                                                                                |
| `src/bindings/`                     | Types generated from the Rust ones (ts-rs, `npm run bindings`)                                                 |
| `src/locales/`, `src/i18n.ts`       | Messages in English and French (shared with the backend), vue-i18n setup                                       |
| `src/composables/`                  | Popup height, names under the tiles, update status, error, fault and plugin wording                            |
| `src/utils/`                        | Lamp colors and limits, color math, number formats, Health tab checks, update steps, plugin icons, CSV, charts |
| `src/assets/css/`                   | Tailwind, Nuxt UI, the font and the palettes                                                                   |
| `src-tauri/tauri.updater.conf.json` | Release builds: also create the signed update files                                                            |
| `src-tauri/tauri.wchisp.conf.json`  | Release builds: bundle wchisp, its license and notice (`src-tauri/third-party`)                                |
| `scripts/updater-manifest.mjs`      | Writes the `latest.json` of a release (release workflow)                                                       |
| `app-icon.png`                      | The logo, source of the app icons (`npx tauri icon`)                                                           |
| `src-tauri/installer/`              | Windows installer bitmaps and macOS disk image background                                                      |

The Rust code is one Cargo workspace at the repository root, with this app
(`src-tauri`), the shared crates (`../crates`) and the production tool
(`../tools`). Its build output goes to `../target`.

## Logo

The logo is the lamp in isometric view, at the enclosure's dimensions, lit
green. `app-icon.png` is the source of the app icons. After a change,
regenerate them:

```sh
npx tauri icon
rm -r src-tauri/icons/android src-tauri/icons/ios src-tauri/icons/64x64.png
```

The tray icon is not an image. `src-tauri/src/tray.rs` draws it at run time
in the color of the lamp.

## Checks

The checks of the app and of the Rust workspace are in
[CONTRIBUTING.md](../CONTRIBUTING.md#checks).
