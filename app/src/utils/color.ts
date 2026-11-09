// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

// Color codes as the editor handles them. The code the user typed or picked
// stays as it is; only the sliders make a new one, at full brightness.

/**
 * Reads a code as the user types it: 3 or 6 hexadecimal digits, with or
 * without "#". It gives the same color as `#RRGGBB` in upper case, and null
 * for anything else and for black, which has no light: the rule of
 * src-tauri/src/colors.rs.
 */
export function parseHex(input: string): string | null {
  const digits = input.trim().replace(/^#/, '');
  const full = digits.length === 3 ? [...digits].map((digit) => digit + digit).join('') : digits;
  if (!/^[0-9a-f]{6}$/i.test(full)) {
    return null;
  }
  const hex = `#${full.toUpperCase()}`;
  return hex === '#000000' ? null : hex;
}

/**
 * The hue (0 to 359 degrees) and the vividness (0 to 100, the HSV
 * saturation) of a `#RRGGBB` code. A grey or a white has no hue: null.
 */
export function hueAndVividness(hex: string): { hue: number | null; vivid: number } {
  const [r, g, b] = [1, 3, 5].map((at) => Number.parseInt(hex.slice(at, at + 2), 16) / 255) as [
    number,
    number,
    number,
  ];
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  const range = max - min;
  const vivid = max === 0 ? 0 : Math.round((range / max) * 100);
  if (range === 0) {
    return { hue: null, vivid };
  }
  let hue: number;
  if (max === r) {
    hue = ((g - b) / range) % 6;
  } else if (max === g) {
    hue = (b - r) / range + 2;
  } else {
    hue = (r - g) / range + 4;
  }
  return { hue: Math.round((hue * 60 + 360) % 360) % 360, vivid };
}

/** The code of a hue and a vividness at full brightness (HSV, value 100 %). */
export function hexOf(hue: number, vivid: number): string {
  const saturation = vivid / 100;
  const sector = (((hue % 360) + 360) % 360) / 60;
  const chroma = saturation;
  const x = chroma * (1 - Math.abs((sector % 2) - 1));
  const [r, g, b] = (
    [
      [chroma, x, 0],
      [x, chroma, 0],
      [0, chroma, x],
      [0, x, chroma],
      [x, 0, chroma],
      [chroma, 0, x],
    ] as const
  )[Math.floor(sector) % 6] ?? [0, 0, 0];
  const m = 1 - chroma;
  return `#${[r, g, b]
    .map((channel) =>
      Math.round((channel + m) * 255)
        .toString(16)
        .padStart(2, '0'),
    )
    .join('')
    .toUpperCase()}`;
}
