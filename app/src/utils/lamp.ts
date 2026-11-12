// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

/** The protocol version that this app drives, as PROTOCOL_MAJOR of the protocol crate. */
export const PROTOCOL_MAJOR = 2;

/**
 * The built-in colors, by palette slot (0 to 3): the key of their name
 * under `lamp.colors`, and their swatch. The swatches show the lamp colors
 * as the tray icon of src-tauri/src/tray.rs does, close to the LEDs whatever
 * the color mode.
 */
export const BUILT_IN_COLORS = [
  { key: 'green', swatch: '#2ecc71' },
  { key: 'orange', swatch: '#f39c12' },
  { key: 'red', swatch: '#e74c3c' },
  { key: 'warm_white', swatch: '#ffe2b8' },
] as const;

/**
 * Limits of the automatic brightness correction, in percent, as
 * AUTO_BIAS_MIN and AUTO_BIAS_MAX of the protocol crate.
 */
export const AUTO_BIAS_MIN = -50;
export const AUTO_BIAS_MAX = 50;
