// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

import { refAutoReset } from '@vueuse/core';
import { useErrorStore } from '@/stores/errors';

/** How long "Copied" shows, ms. */
const COPIED_MS = 2000;

/**
 * "Copy": writes a text to the clipboard, which the app never reads, and
 * says "Copied" for a while; a failure becomes the last error. Not VueUse's
 * useClipboard, whose fallback reports a success even when it fails.
 */
export function useCopy() {
  const errors = useErrorStore();
  const copied = refAutoReset(false, COPIED_MS);

  async function copy(text: string): Promise<void> {
    try {
      await navigator.clipboard.writeText(text);
      copied.value = true;
      errors.clear();
    } catch (error) {
      errors.reportFailure(error);
    }
  }

  return { copied, copy };
}
