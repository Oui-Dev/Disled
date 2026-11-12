// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

import { useMutationObserver, useResizeObserver } from '@vueuse/core';
import { type Ref, ref } from 'vue';
import { popupResize } from '@/ipc/commands';

/** The floating parts of Nuxt UI (popovers, menus), rendered at the end of the body. */
const FLOATING = '[data-reka-popper-content-wrapper]';
/** The space kept under a floating part. */
const FLOATING_MARGIN = 8;

/**
 * Asks for a popup window as tall as its parts stacked, within the limits the backend
 * keeps. A popover or a menu that opens lower than that, as "My colors" with names on
 * two lines, takes the window down to its bottom and a margin, and gives it back when
 * it closes. The page scrolls only on a short screen, or a scrollbar would flash while
 * the window catches up. `fit` measures again at once, for a page changed while hidden.
 */
export function usePopupAutoSize(...parts: Readonly<Ref<HTMLElement | null>>[]): {
  fit: () => Promise<void>;
} {
  const page = document.documentElement;
  page.style.overflowY = 'hidden';
  let sent = 0;
  const floating = ref<HTMLElement[]>([]);

  async function fit(): Promise<void> {
    const content = parts.reduce(
      (sum, part) => sum + (part.value?.getBoundingClientRect().height ?? 0),
      0,
    );
    const below = floating.value.reduce(
      (lowest, element) =>
        Math.max(lowest, element.getBoundingClientRect().bottom + window.scrollY + FLOATING_MARGIN),
      0,
    );
    const height = Math.ceil(Math.max(content, below));
    if (height <= 0 || height === sent) {
      return;
    }
    sent = height;
    const got = await popupResize(height);
    // A later request has the last word.
    if (height === sent) {
      page.style.overflowY = got < height ? 'auto' : 'hidden';
    }
  }

  for (const part of parts) {
    useResizeObserver(part, () => void fit());
  }
  // The floating parts come and go at the end of the body, and grow as
  // their content does.
  useMutationObserver(
    document.body,
    () => {
      floating.value = [...document.querySelectorAll<HTMLElement>(FLOATING)];
      void fit();
    },
    { childList: true },
  );
  useResizeObserver(floating, () => void fit());
  return { fit };
}
