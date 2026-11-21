// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

import { createPinia, setActivePinia } from 'pinia';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { ref } from 'vue';
import type { SettingsPatch } from '@/bindings/SettingsPatch';
import { useSettingsStore } from '@/stores/settings';
import { MOVE_UPDATE_MS } from '@/utils/lamp';
import { useHeldSlider } from './useHeldSlider';

/** A touch sensitivity slider over settings whose saves the test records. */
function slider() {
  const saved = ref(0);
  const patches: SettingsPatch[] = [];
  useSettingsStore().update = async (patch) => {
    patches.push(patch);
  };
  const held = useHeldSlider(
    () => saved.value,
    (percent) => ({ touch_sensitivity: percent }),
  );
  return { saved, patches, ...held };
}

describe('useHeldSlider', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.useFakeTimers();
  });
  afterEach(() => vi.useRealTimers());

  it('holds the value under the hand until the settings have it', async () => {
    const { saved, patches, value, move, release } = slider();
    move(10);
    expect(value.value).toBe(10);
    await release();
    expect(patches.at(-1)).toEqual({ touch_sensitivity: 10 });
    // The hand let go: the slider shows the settings again.
    saved.value = 15;
    expect(value.value).toBe(15);
  });

  it('saves a moving value at most every MOVE_UPDATE_MS, the last one included', () => {
    const { patches, move } = slider();
    move(5);
    move(10);
    move(15);
    expect(patches).toEqual([{ touch_sensitivity: 5 }]);
    vi.advanceTimersByTime(MOVE_UPDATE_MS);
    expect(patches).toEqual([{ touch_sensitivity: 5 }, { touch_sensitivity: 15 }]);
  });
});
