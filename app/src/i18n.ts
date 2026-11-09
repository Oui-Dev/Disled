// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

// Translations of the windows. The messages live in src/locales, shared
// with the backend (src-tauri/src/i18n.rs), which also picks the language
// and sends it as `locale-changed`.

import { en as nuxtUiEn, fr as nuxtUiFr } from '@nuxt/ui/locale';
import { createI18n } from 'vue-i18n';
import type { Locale } from '@/bindings/Locale';
import en from '@/locales/en.json';
import fr from '@/locales/fr.json';

/** English is the reference, so a message French lacks fails the typecheck. */
type MessageSchema = typeof en;

declare module 'vue-i18n' {
  // Typed keys for t() in every component.
  export interface DefineLocaleMessage extends MessageSchema {}
}

const numberFormats = {
  decimal: { minimumFractionDigits: 1, maximumFractionDigits: 1 },
  /** A chart value: 2, or 0.5 between whole ticks. */
  tick: { maximumFractionDigits: 1 },
  /** From a fraction: 0.37 is 37 %. */
  percent: { style: 'percent', maximumFractionDigits: 1 },
  /** A correction: +10 %, −20 %, 0 %. */
  signedPercent: { style: 'percent', signDisplay: 'exceptZero', maximumFractionDigits: 0 },
  milliseconds: { style: 'unit', unit: 'millisecond' },
  seconds: { style: 'unit', unit: 'second' },
  minutes: { style: 'unit', unit: 'minute', minimumFractionDigits: 1, maximumFractionDigits: 1 },
} as const;

const datetimeFormats = {
  short: { dateStyle: 'short', timeStyle: 'short' },
  /** The local time of a day, as "10:42". */
  time: { timeStyle: 'short' },
} as const;

/** French counts 0 as singular, as in "0 ligne"; messages are "one | other". */
function frenchPlural(choice: number, choicesLength: number): number {
  const singular = Math.abs(choice) < 2;
  if (choicesLength === 3) {
    return choice === 0 ? 0 : singular ? 1 : 2;
  }
  return singular ? 0 : 1;
}

export const i18n = createI18n<[MessageSchema], Locale, false>({
  legacy: false,
  locale: 'en',
  fallbackLocale: 'en',
  messages: { en, fr },
  numberFormats: { en: numberFormats, fr: numberFormats },
  datetimeFormats: { en: datetimeFormats, fr: datetimeFormats },
  pluralRules: { fr: frenchPlural },
});

/** The words of the Nuxt UI components, such as the close buttons and the empty lists. */
export const nuxtUiLocales = { en: nuxtUiEn, fr: nuxtUiFr } satisfies Record<Locale, unknown>;

/** Switches the messages, the formats and the language of the document. */
export function setLocale(locale: Locale): void {
  i18n.global.locale.value = locale;
  document.documentElement.lang = locale;
}
