// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

import type { LampColor } from '@/bindings/LampColor';

/** The lamp colors, in the order of the color picker. */
export const LAMP_COLORS = [
  'green',
  'orange',
  'red',
  'warm_white',
] as const satisfies readonly LampColor[];

/**
 * How the swatches show the lamp colors, as the tray icon of
 * src-tauri/src/tray.rs does, close to the LEDs whatever the color mode.
 */
export const SWATCH_COLORS: Record<LampColor, string> = {
  green: '#2ecc71',
  orange: '#f39c12',
  red: '#e74c3c',
  warm_white: '#ffe2b8',
};

/**
 * Limits of the automatic brightness correction, in percent, as
 * AUTO_BIAS_MIN and AUTO_BIAS_MAX of the protocol crate.
 */
export const AUTO_BIAS_MIN = -50;
export const AUTO_BIAS_MAX = 50;
