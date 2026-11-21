// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

/**
 * The built-in colors, by palette slot (0 to 3): the key of their name
 * under `lamp.colors`, and their swatch, whatever the color mode, as the
 * tray icon of src-tauri/src/tray.rs shows them. Green, orange and red use
 * the LED's own primaries, more saturated than any screen: their swatches
 * are the nearest sRGB colors of the same hue. Warm white is 2700 K: the
 * lamp shows its swatch through its calibration, as it does a custom color.
 */
export const BUILT_IN_COLORS = [
  { key: 'green', swatch: '#00ffa2' },
  { key: 'orange', swatch: '#ff6f00' },
  { key: 'red', swatch: '#ff001c' },
  { key: 'warm_white', swatch: '#ffad59' },
] as const;

/**
 * Limits of the automatic brightness correction, in percent, as
 * AUTO_BIAS_MIN and AUTO_BIAS_MAX of the protocol crate.
 */
export const AUTO_BIAS_MIN = -50;
export const AUTO_BIAS_MAX = 50;
