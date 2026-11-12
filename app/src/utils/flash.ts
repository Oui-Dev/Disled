// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

import type { Flash } from '@/bindings/Flash';

/** The steps of a lamp update, as the update window lists them. */
export const UPDATE_STEPS = ['download', 'bootloader', 'install', 'restart'] as const;

export type UpdateStep = (typeof UPDATE_STEPS)[number];

/**
 * Index in UPDATE_STEPS of the step an update is at, UPDATE_STEPS.length
 * once it is done, null when no update runs or the last one failed.
 */
export function stepOf(flash: Flash): number | null {
  switch (flash.step) {
    case 'downloading':
      return 0;
    case 'waiting_for_bootloader':
    case 'driver_missing':
    case 'no_permission':
    case 'several_bootloaders':
    case 'other_chip':
      return 1;
    case 'flashing':
      return 2;
    case 'waiting_for_lamp':
      return 3;
    case 'done':
      return UPDATE_STEPS.length;
    case 'idle':
    case 'failed':
      return null;
  }
}
