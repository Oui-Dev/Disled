// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

import { useThrottleFn } from '@vueuse/core';
import { computed, nextTick, ref } from 'vue';
import type { SettingsPatch } from '@/bindings/SettingsPatch';
import { useSettingsStore } from '@/stores/settings';
import { MOVE_UPDATE_MS } from '@/utils/lamp';

/**
 * A setting on a slider that moves only under the user's hand. The value
 * under the hand is held until it is saved, so that the thumb never jumps
 * back, and the setting follows it at most every MOVE_UPDATE_MS. `saved`
 * reads the setting; `patchOf` makes the patch of a value as the hand moves
 * it, so that a later change of meaning, as the Auto switch, cannot reach
 * a patch on its way.
 */
export function useHeldSlider(saved: () => number, patchOf: (value: number) => SettingsPatch) {
  const settings = useSettingsStore();
  const held = ref<number | null>(null);
  let moving = false;

  const value = computed(() => held.value ?? saved());

  const saveWhileMoving = useThrottleFn(
    (patch: SettingsPatch) => settings.update(patch),
    MOVE_UPDATE_MS,
    true,
  );

  function move(next: number | number[] | undefined): void {
    if (typeof next !== 'number') {
      return;
    }
    moving = true;
    held.value = next;
    void saveWhileMoving(patchOf(next));
  }

  async function release(): Promise<void> {
    // A key commits before it moves the slider, so wait for the move.
    await nextTick();
    moving = false;
    if (held.value === null) {
      return;
    }
    await settings.update(patchOf(held.value));
    // The settings hold the value now, unless the hand came back meanwhile.
    if (!moving) {
      held.value = null;
    }
  }

  /** Lets go of the value under the hand, for a slider that changes its meaning. */
  function drop(): void {
    held.value = null;
  }

  return { value, move, release, drop };
}
