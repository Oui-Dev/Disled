// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

/**
 * Colors as the lamp takes them: CIE 1931 chromaticities (`Xy`, x and y
 * times 65536, as the protocol carries them), the brightness set apart.
 *
 * - On screen, a chromaticity shows as near as the screen can: its light in
 *   Display P3 where the window can show it, otherwise in sRGB, a channel
 *   below 0 brought to 0, the largest one at full. The lamp shows colors
 *   beyond both; srgb_of of src-tauri/src/colors.rs does the same for the
 *   tray icon.
 * - The square of the editor has the hues of sRGB across, in their usual
 *   places, and the vividness upwards: from the white of the lamp, D65, to
 *   the edge of the gamut of its LEDs, beyond sRGB, in steps even to the eye
 *   (CIE 1976 u'v'). A point of the square gives a chromaticity, and a
 *   chromaticity its point, to the nearest degree and percent.
 */

import type { Gamut } from '@/bindings/Gamut';
import type { Xy } from '@/bindings/Xy';

/** The coordinates are x and y times this, as `Xy::ONE`. */
const ONE = 65536;

/** Coordinates, from 0 to 1. */
interface Point {
  x: number;
  y: number;
}

type Rgb = [number, number, number];

/** The white of the lamp. */
const D65: Point = { x: 0.3127, y: 0.329 };

/**
 * The gamut of the lamps of the first board (`app_led_gamut` of the
 * firmware), until a lamp tells its own.
 */
export const DEFAULT_GAMUT: Gamut = {
  red: { x: 45170, y: 20347 },
  green: { x: 9748, y: 46140 },
  blue: { x: 9839, y: 2091 },
};

/** The square: hues 0 to 359 across, vividness 0 to 100 upwards. */
export const HUES = 360;
export const VIVIDNESS_MAX = 100;

const SRGB_TO_XYZ = [
  [0.4124564, 0.3575761, 0.1804375],
  [0.2126729, 0.7151522, 0.072175],
  [0.0193339, 0.119192, 0.9503041],
];
const XYZ_TO_SRGB = [
  [3.2404542, -1.5371385, -0.4985314],
  [-0.969266, 1.8760108, 0.041556],
  [0.0556434, -0.2040259, 1.0572252],
];
const XYZ_TO_DISPLAY_P3 = [
  [2.4934969, -0.9313836, -0.4027108],
  [-0.829489, 1.7626641, 0.0236247],
  [0.0358458, -0.0761724, 0.9568845],
];

function times(matrix: number[][], v: Rgb): Rgb {
  const row = (i: number) => matrix[i][0] * v[0] + matrix[i][1] * v[1] + matrix[i][2] * v[2];
  return [row(0), row(1), row(2)];
}

/** The transfer function of sRGB, also that of Display P3. */
function encode(linear: number): number {
  return linear <= 0.0031308 ? 12.92 * linear : 1.055 * linear ** (1 / 2.4) - 0.055;
}

function decode(encoded: number): number {
  return encoded <= 0.04045 ? encoded / 12.92 : ((encoded + 0.055) / 1.055) ** 2.4;
}

function pointOfXy(xy: Xy): Point {
  return { x: xy.x / ONE, y: xy.y / ONE };
}

function xyOfPoint(point: Point): Xy {
  return { x: Math.round(point.x * ONE), y: Math.round(point.y * ONE) };
}

/** Whether the windows show Display P3, wider than sRGB. */
const p3 =
  typeof CSS !== 'undefined'
  && typeof CSS.supports === 'function'
  && CSS.supports('color', 'color(display-p3 1 0 0)');

/** The color space of the screen colors: Display P3 where the window shows it. */
const SCREEN_SPACE: PredefinedColorSpace = p3 ? 'display-p3' : 'srgb';

/**
 * A chromaticity on screen, at full brightness: the gamma-encoded channels
 * of `space`, 0 to 1.
 */
export function screenOf(xy: Xy, space: PredefinedColorSpace = SCREEN_SPACE): Rgb {
  const { x, y } = pointOfXy(xy);
  if (y <= 0) {
    return [0, 0, 0];
  }
  const linear = times(space === 'display-p3' ? XYZ_TO_DISPLAY_P3 : XYZ_TO_SRGB, [
    x / y,
    1,
    (1 - x - y) / y,
  ]).map((channel) => Math.max(0, channel)) as Rgb;
  const largest = Math.max(...linear);
  return largest > 0 ? (linear.map((channel) => encode(channel / largest)) as Rgb) : [0, 0, 0];
}

/** The CSS color of a chromaticity, for a swatch. */
export function cssOf(xy: Xy): string {
  const [r, g, b] = screenOf(xy);
  if (SCREEN_SPACE === 'display-p3') {
    return `color(display-p3 ${r.toFixed(4)} ${g.toFixed(4)} ${b.toFixed(4)})`;
  }
  const byte = (channel: number) => Math.round(channel * 255);
  return `rgb(${byte(r)} ${byte(g)} ${byte(b)})`;
}

/** CIE 1976 u'v', where equal steps look about equal. */
function uv({ x, y }: Point): Point {
  const d = -2 * x + 12 * y + 3;
  return { x: (4 * x) / d, y: (9 * y) / d };
}

function fromUv({ x: u, y: v }: Point): Point {
  const d = 6 * u - 16 * v + 12;
  return { x: (9 * u) / d, y: (4 * v) / d };
}

/** The chromaticity of the sRGB color of a hue at full saturation: the edge of sRGB. */
function srgbEdge(hue: number): Point {
  const h = (((hue % HUES) + HUES) % HUES) / 60;
  const ramp = 1 - Math.abs((h % 2) - 1);
  const sextants: Rgb[] = [
    [1, ramp, 0],
    [ramp, 1, 0],
    [0, 1, ramp],
    [0, ramp, 1],
    [ramp, 0, 1],
    [1, 0, ramp],
  ];
  const encoded = sextants[Math.floor(h) % 6];
  const [X, Y, Z] = times(SRGB_TO_XYZ, encoded.map(decode) as Rgb);
  const sum = X + Y + Z;
  return { x: X / sum, y: Y / sum };
}

/** The angle around the white of each hue of the square. */
const HUE_ANGLES = Array.from({ length: HUES }, (_, hue) => {
  const edge = srgbEdge(hue);
  return Math.atan2(edge.y - D65.y, edge.x - D65.x);
});

/** Where the ray from the white through `toward` leaves the triangle of the gamut. */
function edgeOf(gamut: Gamut, toward: Point): Point {
  const corners = [gamut.red, gamut.green, gamut.blue].map(pointOfXy);
  const dx = toward.x - D65.x;
  const dy = toward.y - D65.y;
  let nearest = Number.POSITIVE_INFINITY;
  corners.forEach((a, i) => {
    const b = corners[(i + 1) % corners.length];
    const ex = b.x - a.x;
    const ey = b.y - a.y;
    const den = dx * ey - dy * ex;
    if (Math.abs(den) < 1e-12) {
      return;
    }
    const t = ((a.x - D65.x) * ey - (a.y - D65.y) * ex) / den;
    const s = ((a.x - D65.x) * dy - (a.y - D65.y) * dx) / den;
    if (t > 0 && s >= -1e-9 && s <= 1 + 1e-9) {
      nearest = Math.min(nearest, t);
    }
  });
  return { x: D65.x + nearest * dx, y: D65.y + nearest * dy };
}

/** The chromaticity of a point of the square, `vividness` from 0 to 100. */
export function xyOf(gamut: Gamut, hue: number, vividness: number): Xy {
  const white = uv(D65);
  const edge = uv(edgeOf(gamut, srgbEdge(hue)));
  const t = vividness / VIVIDNESS_MAX;
  return xyOfPoint(
    fromUv({ x: white.x + (edge.x - white.x) * t, y: white.y + (edge.y - white.y) * t }),
  );
}

/**
 * The point of the square nearest to a chromaticity, to the degree and the
 * percent. The white has no hue of its own: it keeps `hue`. Near a primary,
 * neighbor hues give nearly the same color, as in any HSV square, so a
 * point may come back a degree off, the color the same to the eye.
 */
export function pointOf(gamut: Gamut, xy: Xy, hue = 0): { hue: number; vividness: number } {
  const point = pointOfXy(xy);
  const white = uv(D65);
  const at = uv(point);
  const distance = Math.hypot(at.x - white.x, at.y - white.y);
  const angle = Math.atan2(point.y - D65.y, point.x - D65.x);
  const gap = (other: number) => {
    const turn = Math.abs(angle - other) % (2 * Math.PI);
    return Math.min(turn, 2 * Math.PI - turn);
  };
  let nearest = 0;
  HUE_ANGLES.forEach((other, candidate) => {
    if (gap(other) < gap(HUE_ANGLES[nearest])) {
      nearest = candidate;
    }
  });
  const edge = uv(edgeOf(gamut, srgbEdge(nearest)));
  const full = Math.hypot(edge.x - white.x, edge.y - white.y);
  const vividness = Math.min(VIVIDNESS_MAX, Math.round((distance / full) * VIVIDNESS_MAX));
  return vividness === 0 ? { hue, vividness } : { hue: nearest, vividness };
}

/**
 * Paints the square on a canvas of one pixel per point, `HUES` by
 * `VIVIDNESS_MAX` + 1, the vividness upwards, in the color space of the
 * screen where the canvas has it.
 */
export function paintSquare(canvas: HTMLCanvasElement, gamut: Gamut): void {
  const width = HUES;
  const height = VIVIDNESS_MAX + 1;
  canvas.width = width;
  canvas.height = height;
  let space = SCREEN_SPACE;
  let context: CanvasRenderingContext2D | null = null;
  try {
    context = canvas.getContext('2d', { colorSpace: space });
  } catch {
    space = 'srgb';
    context = canvas.getContext('2d');
  }
  if (!context) {
    return;
  }
  const image = context.createImageData(width, height, { colorSpace: space });
  for (let vividness = 0; vividness <= VIVIDNESS_MAX; vividness++) {
    for (let hue = 0; hue < width; hue++) {
      const at = ((VIVIDNESS_MAX - vividness) * width + hue) * 4;
      const [r, g, b] = screenOf(xyOf(gamut, hue, vividness), space);
      image.data[at] = Math.round(r * 255);
      image.data[at + 1] = Math.round(g * 255);
      image.data[at + 2] = Math.round(b * 255);
      image.data[at + 3] = 255;
    }
  }
  context.putImageData(image, 0, 0);
}
