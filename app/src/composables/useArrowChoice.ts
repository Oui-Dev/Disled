// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

import type { Ref } from 'vue';

/** The keys that move the choice of a radio group, which follows the focus. */
const MOVES = new Set(['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown', 'Home', 'End']);

function onRadio(event: Event): boolean {
  return event.target instanceof Element && event.target.getAttribute('role') === 'radio';
}

/**
 * The colors of a popover, as a radio group that the keyboard can browse.
 * The arrow keys move the choice and leave the popover open; a click, Space
 * or Enter on a color closes it, as Escape does. `listeners` go on the
 * content of the popover, and `closes` tells a choice whether to close it.
 */
export function useArrowChoice(open: Ref<boolean>) {
  let moved = false;
  const listeners = {
    // Before the radio group, which moves the choice on the same key.
    onKeydownCapture(event: KeyboardEvent): void {
      moved = MOVES.has(event.key);
      // Enter chooses nothing in a radio group: the arrows did.
      if (event.key === 'Enter' && onRadio(event)) {
        open.value = false;
      }
    },
    onPointerdownCapture(): void {
      moved = false;
    },
    // Space chooses on its release; on the chosen color, it only closes.
    onKeyup(event: KeyboardEvent): void {
      if (event.key === ' ' && onRadio(event)) {
        open.value = false;
      }
    },
  };
  return { listeners, closes: (): boolean => !moved };
}
