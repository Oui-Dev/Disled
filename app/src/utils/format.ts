// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

// Numbers as the screen shows them, whatever the engine of the window. Edge
// writes "-15 %" with a hyphen-minus, and WebKit may write another space
// before the percent sign.

import { useI18n } from 'vue-i18n';

/**
 * A hyphen-minus, a hyphen or a dash (U+002D, U+2010 to U+2013) before a
 * digit becomes the true minus sign, U+2212.
 */
export function trueMinus(text: string): string {
  return text.replace(/[\u002D\u2010-\u2013](?=\d)/g, '\u2212');
}

/** In French, the space before "%" (U+0020 or U+202F) is a no-break space, U+00A0. */
export function percentSpace(text: string, locale: string): string {
  return locale === 'fr' ? text.replace(/[\u0020\u202F](?=%)/g, '\u00A0') : text;
}

/** Formats a correction in percent with its sign: "+10 %", "−15 %", "0 %". */
export function useSignedPercent(): (percent: number) => string {
  const { n, locale } = useI18n();
  return (percent) => percentSpace(trueMinus(n(percent / 100, 'signedPercent')), locale.value);
}
