# Releasing

[release-please](https://github.com/googleapis/release-please) releases the
firmware and the desktop app separately, each with its own version, tag and
changelog. The maintainers release the hardware by hand
([Hardware releases](#hardware-releases)).

| Component | Path | Tag | Changelog | Version source |
|---|---|---|---|---|
| Firmware | `firmware/` | `fw-vX.Y.Z` | [firmware/CHANGELOG.md](../firmware/CHANGELOG.md) | the tag (`scripts/version.py`); `version.txt` only tracks it for release-please |
| Desktop app | `app/` | `app-vX.Y.Z` | [app/CHANGELOG.md](../app/CHANGELOG.md) | `app/package.json`, read by `tauri.conf.json` |

## Flow

1. Work on a branch and merge it into `main` with a pull request. The
   commit types decide what gets released
   ([CONTRIBUTING.md](../CONTRIBUTING.md#commit-messages)), and
   `changelog-sections` in `release-please-config.json` lists them.
2. On each push to `main`, the `release` workflow opens or updates a single
   release pull request. It bumps only the components that have releasable
   commits, with their next version, computed from the commits, and their
   changelog entries. release-please assigns each commit to a component by
   the files it changes.
3. Just before merging, you may write a few lines on what the release
   brings at the top of a component's section, in the body of the release
   pull request. release-please writes the body again on each push to
   `main`, and takes the release notes from it.
4. Merge the release pull request when you want to release. release-please
   then tags the merge commit for each bumped component and creates one
   **draft** release per component, with its changelog. Only the
   maintainers see drafts. The workflow builds each tag, attaches the
   binaries to its draft, and publishes the release only once all of them
   are attached:
   - **Firmware.** Release and debug builds as `.hex`, `.bin` and `.elf`,
     with `SHA256SUMS`, and the signature of the release build for the app
     (`disled-fw-X.Y.Z.hex.sig`).
   - **App.** Windows installer (`.exe`), macOS disk image for Apple
     Silicon (`.dmg`), Linux `.deb` (Debian, Ubuntu and derivatives). For
     the updater, the macOS app archive (`.app.tar.gz`), one signature
     (`.sig`) per installer and `latest.json`. The source of the wchisp
     inside the installers and its notice (`wchisp-X.Y.Z-source.tar.gz`,
     `wchisp-NOTICE.md`).

   Published releases are immutable, a repository setting (Settings,
   General, Releases). Their files and tag can no longer change, only their
   title and notes.
5. When both components release, the firmware is published only once the
   app installers are attached too: a failed app build must not leave a new
   firmware alone, which the app before it would offer to every lamp. The
   app release follows as soon as the firmware is published, so that an
   app never goes out before the firmware it installs.

Both components are past 1.0.0: a breaking change raises the major
version, `feat` the minor version and the other releasing types the patch
version.

The notes of a published release can still change, but the installed apps
have already read those of the app in `latest.json`. Anything to add goes
in the release pull request (step 3).

## When a release build fails

The release stays a draft, with its tag, and nothing is published. When
both components release, a failed job of one also holds the other back.

- If the cause is outside the code (runner, secret), re-run the failed
  jobs, and the publication follows. Not all the jobs: release-please
  would run again, find the release already created, and skip every
  build.
- If an app build can no longer be re-run, because all the jobs were
  re-run or GitHub has retired the runner, run the `release` workflow by
  hand (Actions, release, Run workflow) with the version of the draft. It
  builds the tag again with the workflow of `main`, replaces the files
  already attached and publishes the release as the latest one, so only
  for the latest app version.
- If the code must change, fix it on `main`, and the fix goes out with the
  next version. Leave the unpublished draft as it is.
- Never delete a release or a tag made by release-please. It finds the
  previous release of each component through them, drafts included, and
  without them the next changelog would take the whole history. A deleted
  published release also leaves its name unusable.

## Hardware releases

The PCB and the enclosure have their own versions, tags and changelogs,
written by hand, because a hardware version is a decision (a board order,
a print), not a sum of commits.

| Design | Path | Tag | Changelog |
|---|---|---|---|
| PCB | `hardware/pcb/` | `pcb-vX.Y.Z` | [hardware/pcb/CHANGELOG.md](../hardware/pcb/CHANGELOG.md) |
| Enclosure | `hardware/enclosure/` | `enclosure-vX.Y.Z` | [hardware/enclosure/CHANGELOG.md](../hardware/enclosure/CHANGELOG.md) |

1. Merge the new files into `main`, over the old ones, with a section for
   the version at the top of the changelog: `## X.Y.Z (YYYY-MM-DD)`, the
   date of the design, then the changes. Links in a section must be
   absolute, because the section becomes the notes of the release. A new
   board revision carries its version on the silkscreen, "Rev 1.1" for
   pcb 1.1.0.
2. Run the **hardware release** workflow on `main` (Actions, hardware
   release, Run workflow) with the design and the version. Tick
   pre-release for a design that will not be manufactured.
3. The workflow tags the commit, attaches the files of the folder (its
   changelog aside) as `disled-<design>-X.Y.Z.<ext>`, or
   `disled-<design>-X.Y.Z-<name>.<ext>` when the folder has several of
   that type, and publishes the release, never as the "Latest" one.

A failed run may leave a draft. Delete it, then run the workflow again.

The workflow releases the commit that runs. For an older commit whose
workflow files differ from those of `main`, GitHub refuses the workflow
token both the tag and the release (HTTP 403), even with the tag pushed by
hand. Release such a version by hand on the releases page, with its files
named as above, its changelog section as notes, and **Set as the latest
release** unticked.

## The "Latest" release

GitHub marks one release per repository as "Latest", whatever the
component. It is always the newest app release, because the installed apps
read `releases/latest/download/latest.json`. The workflows publish firmware
and hardware releases without the flag, and the app finds the newest
firmware by its `fw-v*` tag.

## App updates

The installed apps update themselves from the latest GitHub release:

- the `app` job signs the installers with the update key (the secrets
  below) and attaches a `.sig` next to each one;
- once the three installers are attached, and the firmware published when
  it releases too, the `app-publish` job writes `latest.json` from these
  signatures (`app/scripts/updater-manifest.mjs`), attaches it, and
  publishes the release as the latest;
- the app installs an update only if its signature names the version that
  `latest.json` announces (`requireSignedVersion` in `tauri.conf.json`), so
  a forged manifest cannot pair a newer version number with an older signed
  installer.

Local builds (`npm run tauri build`) need no key, because only the release
workflow adds `src-tauri/tauri.updater.conf.json`, which asks for the
update files.

### Update key (once)

1. Create the key pair outside the repository, with a password:

   ```sh
   cd app
   npm run tauri signer generate -- -w ~/.tauri/disled-updater.key
   ```

   In PowerShell, write the path `$env:USERPROFILE\.tauri\disled-updater.key`.

2. In the repository settings (Secrets and variables, Actions), add two
   repository secrets: `TAURI_SIGNING_PRIVATE_KEY`, the content of
   `disled-updater.key`, and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`, its
   password.
3. Put the content of `disled-updater.key.pub` in `plugins.updater.pubkey`
   of `app/src-tauri/tauri.conf.json`.
4. Keep a backup of the private key and its password, outside the computer.
   The installed apps accept no other key, for their own updates and for
   the lamp firmware, and a new key means a manual reinstall for every
   user.

## Lamp updates

The app updates the lamp firmware from the newest published `fw-v*`
release:

- the `firmware` job signs the release build with the same update key, the
  version included in the signature (`tauri signer sign --app-version`),
  and the app refuses any other file;
- each `app` job builds wchisp (`WCHISP_VERSION` at the top of the
  workflow) and bundles it with `src-tauri/tauri.wchisp.conf.json`, with
  its license and notice (`src-tauri/third-party/wchisp`);
- `app-publish` attaches the crates.io package of that wchisp, checked
  against the crates.io index, and the notice. wchisp is GPL-2.0, so its
  source goes with the installers.

A local installer with wchisp needs the binary first, named after the
target. Install it with `cargo install wchisp@0.3.0 --locked`, copy it to
`app/src-tauri/binaries/wchisp-<target triple>` (`.exe` on Windows), then
run `npm run tauri build -- --config src-tauri/tauri.wchisp.conf.json`.

The app lists every signed release of the firmware major that it drives,
newest first, then the last one of the major before, which takes a lamp
back. So a firmware release stays installable from the app as long as it
lives on the releases page.

## A major version

Version 2 of the firmware and of the app came out together, since each
needs the other. The same steps suit the next major version:

1. The work goes on a long-lived branch, `v2`, in ordinary commits. Its pull
   request into `main` opens with the first push, ready for review and
   never as a draft, since the checks run only on such a pull request. It
   stays open until the release. Meanwhile `main` stays free for a fix of
   the released version and for Dependabot, and `main` is merged into the
   branch after each of them, then once more before the release.
2. One breaking commit per component, in its own paths, raises its major
   version: `feat(firmware)!:` and `feat(app)!:`. No `Release-As` footer is
   needed. Should the release pull request still read another version, add
   `"release-as": "X.0.0"` to the component in
   `release-please-config.json` ([Notes](#notes)).
3. The branch goes into `main` with a merge commit, never a squash: with a
   squash, only its title would count for release-please.
4. Write the notes of both components in the release pull request (step 3
   of the [flow](#flow)), then merge it.
5. The firmware release is published first, then the app, right after it
   in the same run. In between, the app of the previous major may offer the
   new firmware to a lamp, and a lamp updated then works on its own but no
   longer takes colors from that app, until the new app is installed. The
   previous app offers its own update first once the new app is out.

## Notes

- The installers carry no publisher certificate, since the update
  signatures above only serve the updater. Windows SmartScreen and macOS
  Gatekeeper warn on first launch ([user guide](user-guide.md#install-the-app)).
- The macOS app is signed ad hoc (`signingIdentity` `-` in
  `tauri.conf.json`), because Apple Silicon calls a downloaded app without
  any signature "damaged" and offers only to bin it
  ([issue #19](https://github.com/Oui-Dev/Disled/issues/19)). The release
  workflow checks that signature, and that `Info.plist` declares no use of
  the microphone or of the camera: the calls come from the microphone
  without any permission, and such a declaration would have macOS ask for
  one.
- The disk image window has a background,
  `app/src-tauri/installer/dmg-background.png`, with an arrow toward
  Applications and the first-launch steps
  ([issue #36](https://github.com/Oui-Dev/Disled/issues/36)). It also
  shows the address of the "Install the app" section of the user guide,
  so keep that heading. Tauri lays the window out with an AppleScript
  that it skips on CI, unless `TAURI_BUNDLER_DMG_IGNORE_CI` is set. The
  release workflow sets it and checks the layout.
- The repository must allow the workflow to open pull requests, with
  **Allow GitHub Actions to create and approve pull requests** in Settings,
  Actions, General, Workflow permissions. The default token permissions can
  stay read-only. The release workflow asks for write access itself, and
  the other workflows only need to read.
- To force a version, add `"release-as": "X.Y.Z"` to the component in
  `release-please-config.json`, and remove it once that version is out. As
  long as it stays, release-please proposes that version again. A
  `Release-As: X.Y.Z` footer in the body of a commit that touches the
  component does the same for one release.
- Dependabot opens weekly update pull requests (`.github/dependabot.yml`)
  with Conventional Commit messages. `fix(deps)` covers what the app ships
  (npm production dependencies, Rust crates) and releases the app.
  `chore(deps-dev)` (app tooling), `build(deps)` (firmware tools, Rust
  version) and `ci(deps)` (actions) release nothing. An update of the root
  `Cargo.lock` alone belongs to no component and goes out with the next app
  release.
- `protocol/`, `crates/`, `tools/`, `hardware/` and the root `Cargo.*`
  files belong to no component. A change there goes out with the component
  that the same commit touches, and a protocol change touches the firmware,
  the app or both. The production tool has no release and runs from the
  repository ([production.md](production.md)), and the maintainers release
  the hardware by hand (above).
- Both components share one release pull request
  (`separate-pull-requests: false`). With one per component, both edit
  `.release-please-manifest.json`, so merging one leaves the other in
  conflict, and release-please does not rebuild a pull request whose
  content did not change. The cost is that both components are released
  together when both have pending changes.
