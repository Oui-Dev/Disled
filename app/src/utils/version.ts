// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

/**
 * Compares an "X.Y.Z" version with the version a lamp reports: negative
 * when the version is older, 0 when it is the lamp's, positive when newer.
 */
export function compare(version: string, lamp: readonly number[]): number {
  const parts = version.split('.').map(Number);
  for (let i = 0; i < 3; i++) {
    if (parts[i] !== lamp[i]) {
      return parts[i] - lamp[i];
    }
  }
  return 0;
}
