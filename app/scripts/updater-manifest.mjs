// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

// Writes the latest.json of an app release from the signatures of its installers:
//   node scripts/updater-manifest.mjs <version> <download URL> <signature dir> [notes file]
// <download URL> is that of the installers, .../releases/download/app-vX.Y.Z.

import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

// Installer name endings and their updater targets. The app looks for
// `{os}-{arch}-{bundle}` first, then `{os}-{arch}`.
const TARGETS = [
  { ending: '_x64-setup.exe', os: 'windows', arch: 'x86_64', bundle: 'nsis' },
  { ending: '_aarch64.app.tar.gz', os: 'darwin', arch: 'aarch64', bundle: 'app' },
  { ending: '_amd64.deb', os: 'linux', arch: 'x86_64', bundle: 'deb' },
];

const [version, downloadUrl, signatureDir, notesFile] = process.argv.slice(2);
if (!version || !downloadUrl || !signatureDir) {
  console.error(
    'usage: updater-manifest.mjs <version> <download URL> <signature dir> [notes file]',
  );
  process.exit(2);
}

const platforms = {};
for (const file of readdirSync(signatureDir).sort()) {
  if (!file.endsWith('.sig')) {
    continue;
  }
  const installer = file.slice(0, -'.sig'.length);
  const target = TARGETS.find((t) => installer.endsWith(t.ending));
  if (!target) {
    continue;
  }
  const entry = {
    signature: readFileSync(join(signatureDir, file), 'utf8').trim(),
    url: `${downloadUrl}/${encodeURIComponent(installer)}`,
  };
  platforms[`${target.os}-${target.arch}`] = entry;
  platforms[`${target.os}-${target.arch}-${target.bundle}`] = entry;
}
if (Object.keys(platforms).length === 0) {
  console.error(`no installer signature in ${signatureDir}`);
  process.exit(1);
}

const manifest = {
  version,
  notes: notesFile ? readFileSync(notesFile, 'utf8').trim() : '',
  pub_date: new Date().toISOString(),
  platforms,
};
process.stdout.write(`${JSON.stringify(manifest, null, 2)}\n`);
