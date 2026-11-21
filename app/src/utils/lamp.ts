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
  { key: 'green', swatch: '#00f57a' },
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

/**
 * Limits of the touch sensitivity, in percent, as TOUCH_SENSITIVITY_MIN
 * and TOUCH_SENSITIVITY_MAX of the protocol crate.
 */
export const TOUCH_SENSITIVITY_MIN = -50;
export const TOUCH_SENSITIVITY_MAX = 50;

/** Most custom colors, as CUSTOM_MAX in src-tauri/src/colors.rs. */
export const CUSTOM_MAX = 5;

/** The code of SELF_TEST, which the ERROR that rejects it names: Command::SelfTest.code(). */
export const CMD_SELF_TEST = 0x08;

/** How often a moving control updates the lamp, ms. */
export const MOVE_UPDATE_MS = 120;
