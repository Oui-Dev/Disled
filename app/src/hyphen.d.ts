// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

// The hyphen package ships no types: what the app uses of it.

interface HyphenOptions {
  /** U+00AD, the soft hyphen, by default. */
  hyphenChar?: string;
  minWordLength?: number;
}

declare module 'hyphen/fr' {
  export function hyphenateSync(text: string, options?: HyphenOptions): string;
}

declare module 'hyphen/en-us' {
  export function hyphenateSync(text: string, options?: HyphenOptions): string;
}
