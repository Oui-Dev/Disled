// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

import { describe, expect, it } from 'vitest';
import type { Xy } from '@/bindings/Xy';
import { DEFAULT_GAMUT, HUES, pointOf, screenOf, VIVIDNESS_MAX, xyOf } from './color';

const D65 = { x: 20493, y: 21561 };

/** The distance of two chromaticities in CIE 1976 u'v', where 0.004 is about the least the eye sees. */
function distance(a: Xy, b: Xy): number {
  const uv = ({ x, y }: Xy) => {
    const d = -2 * x + 12 * y + 3 * 65536;
    return [(4 * x) / d, (9 * y) / d] as const;
  };
  const [ua, va] = uv(a);
  const [ub, vb] = uv(b);
  return Math.hypot(ua - ub, va - vb);
}

describe('the square of the editor', () => {
  it('starts from the white of the lamp, which keeps the hue', () => {
    for (const hue of [0, 120, 240]) {
      expect(xyOf(DEFAULT_GAMUT, hue, 0)).toEqual(D65);
    }
    expect(pointOf(DEFAULT_GAMUT, D65, 42)).toEqual({ hue: 42, vividness: 0 });
  });

  it('puts the thumb back on the color of each of its points', () => {
    // A point a degree off near a primary gives the same color to the eye.
    let worst = 0;
    for (let hue = 0; hue < HUES; hue++) {
      for (const vividness of [1, 2, 5, 10, 25, 50, 75, VIVIDNESS_MAX]) {
        const xy = xyOf(DEFAULT_GAMUT, hue, vividness);
        const back = pointOf(DEFAULT_GAMUT, xy);
        expect(back.vividness, `${hue} ${vividness}`).toBe(vividness);
        worst = Math.max(worst, distance(xy, xyOf(DEFAULT_GAMUT, back.hue, back.vividness)));
      }
    }
    expect(worst).toBeLessThan(0.001);
  });

  it('reaches the LEDs themselves at full vividness, beyond sRGB', () => {
    for (const led of [DEFAULT_GAMUT.red, DEFAULT_GAMUT.green, DEFAULT_GAMUT.blue]) {
      const { hue, vividness } = pointOf(DEFAULT_GAMUT, led);
      expect(vividness).toBe(VIVIDNESS_MAX);
      expect(distance(xyOf(DEFAULT_GAMUT, hue, vividness), led)).toBeLessThan(0.01);
    }
    // The screen shows the green LED as near as it can: no red at all.
    expect(screenOf(DEFAULT_GAMUT.green, 'srgb')[0]).toBe(0);
  });
});

describe('a chromaticity on screen', () => {
  it('shows the white of the lamp as white', () => {
    for (const space of ['srgb', 'display-p3'] as const) {
      for (const channel of screenOf(D65, space)) {
        expect(channel).toBeCloseTo(1, 3);
      }
    }
  });

  it('shows the warm white as its swatch in sRGB', () => {
    const [r, g, b] = screenOf({ x: 30097, y: 26924 }, 'srgb').map((c) => Math.round(c * 255));
    expect([r, g, b]).toEqual([0xff, 0xad, 0x59]);
  });
});
