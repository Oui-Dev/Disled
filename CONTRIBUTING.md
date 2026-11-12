# Contributing to Disled

The Disled repository holds the firmware, the desktop app, the shared Rust
crates and the hardware design files. This page explains how to propose a
change and what the checks expect. For a bug or an idea, open an
[issue](https://github.com/Oui-Dev/Disled/issues) first, unless the fix is
small. The [user guide](docs/user-guide.md#get-help) lists what a bug report
needs.

## Before you start

Clone with the TinyUSB submodule:

```sh
git clone --recurse-submodules https://github.com/Oui-Dev/Disled.git
```

In an existing clone, `git submodule update --init` fetches it. Then set up
the part you work on:

| Part | Tools | Setup |
|---|---|---|
| Firmware | PlatformIO and clang-format through [uv](https://docs.astral.sh/uv/), a host C compiler for the unit tests | [firmware/README.md](firmware/README.md) |
| Desktop app | Node.js 26, Rust through `rustup`, the build tools of your system | [app/README.md](app/README.md) |
| Production tool, shared crates | Rust through `rustup` | [docs/production.md](docs/production.md) |
| Hardware | EasyEDA Pro (PCB), a 3D editor for the 3MF parts | [hardware/README.md](hardware/README.md) |

npm accepts only the Node.js versions that `engines` lists in
`app/package.json`. `rust-toolchain.toml` pins Rust: `rustup toolchain
install`, run in the repository, installs that version with clippy and
rustfmt.

[docs/architecture.md](docs/architecture.md) describes the design,
[docs/behavior.md](docs/behavior.md) the lamp behavior (the reference
specification) and [protocol/README.md](protocol/README.md) the USB
protocol.

## Branches and pull requests

- Work on one topic per branch, started from an up-to-date `main` and named
  `<type>/<topic>`, for example `feat/auto-brightness` or
  `fix/macos-adhoc-signing`.
- Open a pull request into `main` and keep it a draft while you work. The
  checks run once it is ready for review, then on every push.
- Say what changes and why, and how you tested it: the board and firmware
  for a firmware change, the system for the app, screenshots for a change
  that shows.
- Update the documentation in the same pull request: the README of the
  part, [docs/behavior.md](docs/behavior.md) when the lamp behaves
  differently, the user guide when the app does.
- Maintainers merge pull requests with a merge commit, so every commit
  lands on `main` as written and goes into the changelogs. Tidy the history
  before the review.

## Commit messages

Commit messages follow [Conventional Commits](https://www.conventionalcommits.org/),
in English: `type(scope): summary`. The summary is in lower case and
describes the result, for example `fix(app): sign the macOS app ad hoc so
that it opens on Apple Silicon`. The body says why, wrapped at 72
characters.

- `feat`, `fix`, `perf`, `refactor` and `revert` release the part whose
  files the commit changes, and appear in its changelog. `docs`, `style`,
  `build`, `chore`, `test` and `ci` release nothing
  ([docs/releasing.md](docs/releasing.md)).
- Scopes in use: `app`, `firmware`, `protocol`, `factory` (the production
  tool), `hardware`, `deps`.
- `feat!` or a `BREAKING CHANGE:` footer marks a breaking change. Version 2
  of the USB protocol is frozen at 2.0 and only grows in compatible ways
  ([protocol/README.md](protocol/README.md#compatibility)).

## Checks

The pull request checks run these commands. Run them before you push.

Firmware, from `firmware/`:

```sh
uv sync --locked
uv run clang-format --dry-run --Werror $(git ls-files 'lib/disled_core/*.[ch]' 'src/*.[ch]' 'test/*.c' 'tools/*.c')
uv run pio test -e native            # unit tests on the build machine
uv run pio run -e debug -e release
uv run pio check -e debug            # static analysis
```

Desktop app, from `app/`, then the Rust workspace from the root:

```sh
npm ci
npm run typecheck                    # vue-tsc, French messages included
npm run check                        # Biome; npm run fix applies the fixes
npm run build
cd ..
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
rm -r app/src/bindings               # the tests write the TypeScript types again
cargo test --workspace               # protocol codec, messages of both languages, TypeScript types
git status --porcelain -- app/src/bindings
```

If the last command lists files, a Rust type that the front end receives
has changed: commit the new `app/src/bindings` (`npm run bindings` writes
them too).

Some changes can only be checked on a board: the lamp behavior
([test scenarios](docs/behavior.md#test-scenarios)), the touch, the light
sensor, USB sleep. [docs/bringup.md](docs/bringup.md) lists the checks.

## Code and files

- The checks enforce the formatting: clang-format for the firmware, Biome
  for the front end, rustfmt for Rust, and [`.editorconfig`](.editorconfig)
  for the rest.
- The project is named Disled. The name stays in capitals only in the
  identifiers written in capitals, such as `DISLED_FIRMWARE_HEX`, in the
  `DISLED2-` prefix of the share codes and in the changelogs: a check of
  the pull requests looks for it everywhere else.
- Every source file starts with an SPDX header
  ([LICENSING.md](LICENSING.md)). The firmware's headers also point to the
  permission for the WCH SDK.
- Code, comments, documentation and commit messages are in English. The
  app speaks English and French, so every new message goes into both
  `app/src/locales/en.json` and `fr.json` ([app/README.md](app/README.md#translations)).
- The firmware follows its own rules, listed in
  [firmware/README.md](firmware/README.md#conventions).
- A protocol change updates the specification, the shared test vectors
  (`protocol/vectors`) and both codecs, the firmware's and
  `crates/disled-protocol`, in the same pull request.

## Dependencies

Dependabot opens update pull requests every week
([`.github/dependabot.yml`](.github/dependabot.yml)). Some versions stay
pinned on purpose:

- `@tauri-apps/cli` stays on 2.11. From 2.12, the Windows installer can
  abort an update started from the app with "Failed to kill Disled"
  ([tauri-apps/tauri#16277](https://github.com/tauri-apps/tauri/issues/16277)).
  Close Dependabot's pull requests for 2.12 and later until that issue is
  fixed.
- The release workflow builds wchisp, the flasher bundled with the app, at
  the version of `WCHISP_VERSION` (`.github/workflows/release.yml`). wchisp
  has no machine interface, so the app and the production tool read its
  log lines (`crates/disled-flash/src/wchisp.rs`). Before a new version,
  check those lines against it, then update the `cargo install
  wchisp@0.3.0` commands of the documentation.

## Releases

The maintainers release the firmware and the app with release-please, and
the hardware by hand ([docs/releasing.md](docs/releasing.md)). The update
key that signs the app and the firmware lives in the repository secrets.
Never commit a key.

## License

The project accepts contributions under its own terms
([LICENSING.md](LICENSING.md)): GPL-3.0-or-later with the additional
permission for the WCH SDK for the software, and CERN-OHL-S-2.0 for the
hardware design files.
