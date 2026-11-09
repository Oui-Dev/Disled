// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

// The colors of the lamp: the names of the four built-in colors, the custom
// colors and the touch cycle. The backend keeps and checks them and sends
// them to the lamp; every window keeps a copy, refreshed by `colors-changed`.

import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import type { Palette } from '@/bindings/Palette';
import { i18n } from '@/i18n';
import {
  colorsDelete,
  colorsGet,
  colorsImport,
  colorsRenameOriginal,
  colorsSaveCustom,
  colorsSetCycle,
} from '@/ipc/commands';
import { follow } from '@/ipc/events';
import { BUILT_IN_COLORS } from '@/utils/lamp';
import { useErrorStore } from './errors';

/** Most custom colors (D3). */
export const CUSTOM_MAX = 5;

/** A color as the windows show it: its slot, the name the user sees, its code on screen. */
export interface ShownColor {
  slot: number;
  name: string;
  hex: string;
  builtIn: boolean;
}

/** The default name of a built-in color, in the language of the app. */
export function defaultName(slot: number): string {
  const color = BUILT_IN_COLORS[slot];
  return color ? i18n.global.t(`lamp.colors.${color.key}`) : '';
}

export const useColorsStore = defineStore('colors', () => {
  const errors = useErrorStore();

  /** Null until the backend answers. */
  const palette = ref<Palette | null>(null);

  /** The four built-in colors, under the names the user gave them, or their default ones. */
  const builtIn = computed<ShownColor[]>(() =>
    BUILT_IN_COLORS.map((color, slot) => ({
      slot,
      name: palette.value?.names[slot] ?? defaultName(slot),
      hex: color.swatch,
      builtIn: true,
    })),
  );
  /** In the order the user made them. */
  const custom = computed<ShownColor[]>(() =>
    (palette.value?.custom ?? []).map((color) => ({
      slot: color.slot,
      name: color.name,
      hex: color.hex,
      builtIn: false,
    })),
  );
  const cycle = computed(() => palette.value?.cycle ?? [0, 1, 2]);
  const full = computed(() => custom.value.length >= CUSTOM_MAX);

  function color(slot: number): ShownColor | undefined {
    return builtIn.value[slot] ?? custom.value.find((shown) => shown.slot === slot);
  }

  /** Runs a change; a failure becomes the last error, a success clears it. */
  async function change<T>(action: () => Promise<T>): Promise<T | undefined> {
    try {
      const result = await action();
      errors.clear();
      return result;
    } catch (error) {
      errors.reportFailure(error);
      return undefined;
    }
  }

  return {
    palette,
    builtIn,
    custom,
    cycle,
    full,
    color,
    saveCustom: (color: Parameters<typeof colorsSaveCustom>[0]) =>
      change(() => colorsSaveCustom(color)),
    renameOriginal: (slot: number, name: string | null) =>
      change(() => colorsRenameOriginal(slot, name)),
    setCycle: (slots: number[]) => change(() => colorsSetCycle(slots)),
    remove: (slot: number) => change(() => colorsDelete(slot)),
    importCode: (text: string) => change(() => colorsImport(text)),
    /** Follows the backend events. Call once per window. */
    start: () =>
      follow(
        'colors-changed',
        (next) => {
          palette.value = next;
        },
        colorsGet,
      ),
  };
});
