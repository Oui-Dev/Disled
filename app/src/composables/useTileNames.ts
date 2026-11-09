// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

import { nextTick, type Ref, ref, watch } from 'vue';
import { lineWidths } from '@/utils/tileName';

/** The least space between two names, on each line they share. */
const MIN_GAP = 4;

/** Waits for the names to fit themselves again, which takes a frame or two. */
async function frames(count: number): Promise<void> {
  await nextTick();
  for (let frame = 0; frame < count; frame++) {
    await new Promise((resolve) => requestAnimationFrame(resolve));
  }
}

/** The space between the lines of two neighboring names, centered in cells this wide. */
function smallestGap(left: HTMLElement, right: HTMLElement, cell: number): number {
  const a = lineWidths(left);
  const b = lineWidths(right);
  let smallest = Number.POSITIVE_INFINITY;
  for (let line = 0; line < Math.min(a.length, b.length); line++) {
    smallest = Math.min(smallest, cell - (a[line] ?? 0) / 2 - (b[line] ?? 0) / 2);
  }
  return smallest;
}

/**
 * Keeps 4 px between neighboring names (annex C, C.4.3). Where two names
 * would nearly touch on a line they share, one of them goes into a box 8 px
 * narrower: first a name of several words, otherwise the one with the widest
 * line. The narrower box stays only if the name still fits in two lines and
 * leaves the gap. One pass, left to right, after each change of the names.
 * Returns, for each name, whether its box is narrower.
 */
export function useTileNames(row: Ref<HTMLElement | null>, names: Ref<string[]>): Ref<boolean[]> {
  const narrow = ref<boolean[]>([]);

  async function settle(): Promise<void> {
    // The names fit themselves first, after the font has loaded.
    await document.fonts.ready;
    await frames(2);
    const shown = [...(row.value?.querySelectorAll<HTMLElement>('[data-tile-name]') ?? [])];
    for (let left = 0; left + 1 < shown.length; left++) {
      const right = left + 1;
      const a = shown[left];
      const b = shown[right];
      const cell = a?.parentElement?.getBoundingClientRect().width ?? 0;
      if (!a || !b || cell === 0 || smallestGap(a, b, cell) >= MIN_GAP) {
        continue;
      }
      const words = (index: number) => (names.value[index] ?? '').trim().split(/\s+/).length;
      const widest = (element: HTMLElement) => Math.max(0, ...lineWidths(element));
      const order =
        words(left) > 1 !== words(right) > 1
          ? words(left) > 1
            ? [left, right]
            : [right, left]
          : widest(a) >= widest(b)
            ? [left, right]
            : [right, left];
      for (const index of order) {
        narrow.value[index] = true;
        await frames(2);
        const element = shown[index];
        if (element && lineWidths(element).length <= 2 && smallestGap(a, b, cell) >= MIN_GAP) {
          break;
        }
        narrow.value[index] = false;
      }
    }
  }

  watch(
    names,
    (next) => {
      narrow.value = next.map(() => false);
      void settle();
    },
    { immediate: true, deep: true },
  );

  return narrow;
}
