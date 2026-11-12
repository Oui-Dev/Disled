// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

// The names under the color tiles, in 11 px Montserrat on two lines at most.
// A word wider than its box takes soft hyphens at the break points of the
// TeX patterns of the language. A word that fits takes none, or the browser
// would rather break "Ne pas déran-ger" than "Ne pas / déranger".

import type { Locale } from '@/bindings/Locale';

const FONT = '400 11px "Montserrat Variable"';

type Hyphenate = (text: string) => string;

const hyphenators = new Map<Locale, Promise<Hyphenate>>();

/** The patterns of a language, loaded the first time a name needs them. */
function hyphenator(locale: Locale): Promise<Hyphenate> {
  let loading = hyphenators.get(locale);
  if (!loading) {
    const patterns = locale === 'fr' ? import('hyphen/fr') : import('hyphen/en-us');
    loading = patterns.then(
      ({ hyphenateSync }) =>
        (text: string) =>
          hyphenateSync(text, { minWordLength: 4 }),
    );
    hyphenators.set(locale, loading);
  }
  return loading;
}

let fontLoaded: Promise<unknown> | null = null;
let canvas: CanvasRenderingContext2D | null = null;

/**
 * The width of a text in the font of the names. It waits for that font:
 * `document.fonts.ready` may answer before the font is even asked for.
 */
async function measurer(): Promise<(text: string) => number> {
  fontLoaded ??= document.fonts.load(FONT);
  await fontLoaded;
  canvas ??= document.createElement('canvas').getContext('2d');
  const context = canvas;
  if (context) {
    context.font = FONT;
  }
  return (text) => context?.measureText(text).width ?? 0;
}

/** The name as it shows in a box this wide: hyphens only in the words that do not fit. */
export async function fitName(name: string, box: number, locale: Locale): Promise<string> {
  const width = await measurer();
  const words = name.split(' ');
  if (words.every((word) => width(word) <= box)) {
    return name;
  }
  const hyphenate = await hyphenator(locale);
  return words.map((word) => (width(word) > box ? hyphenate(word) : word)).join(' ');
}

/** The widths of the lines of a text element, in order. */
export function lineWidths(element: HTMLElement): number[] {
  const range = document.createRange();
  range.selectNodeContents(element);
  const lines = new Map<number, number>();
  for (const rect of range.getClientRects()) {
    const top = Math.round(rect.top);
    lines.set(top, (lines.get(top) ?? 0) + rect.width);
  }
  return [...lines.entries()].sort(([a], [b]) => a - b).map(([, width]) => width);
}
