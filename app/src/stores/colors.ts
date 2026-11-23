// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

// The colors of the lamp: the names of the four built-in colors, the custom
// colors and the touch cycle. The backend keeps and checks them and sends
// them to the lamp; every window keeps a copy, refreshed by `colors-changed`.

import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import type { LampColor } from '@/bindings/LampColor';
import type { Palette } from '@/bindings/Palette';
import type { Xy } from '@/bindings/Xy';
import { i18n } from '@/i18n';
import {
  colorsDelete,
  colorsGet,
  colorsImport,
  colorsRenameOriginal,
  colorsSaveCustom,
  colorsSetCycle,
} from '@/ipc/commands';
import { mirror } from '@/ipc/events';
import { cssOf } from '@/utils/color';
import { BUILT_IN_COLORS, CUSTOM_MAX } from '@/utils/lamp';
import { useErrorStore } from './errors';

/** A color as the windows show it: its slot, the name the user sees and its color on screen. */
export interface ShownColor {
  slot: number;
  name: string;
  /** A CSS color. */
  swatch: string;
  builtIn: boolean;
  /** The chromaticity of a custom color, null for a built-in one. */
  xy: Xy | null;
}

/** The default name of a built-in color, in the language of the app. */
export function defaultName(slot: number): string {
  const color = BUILT_IN_COLORS[slot];
  return color ? i18n.global.t(`lamp.colors.${color.key}`) : '';
}

/**
 * The four built-in colors of a palette, under the names the user gave
 * them, or their default ones.
 */
export function builtInColors(palette: Palette | null): ShownColor[] {
  return BUILT_IN_COLORS.map((color, slot) => ({
    slot,
    name: palette?.names[slot] ?? defaultName(slot),
    swatch: color.swatch,
    builtIn: true,
    xy: null,
  }));
}

/** The custom colors of a palette, in the order the user made them. */
export function customColors(palette: Palette | null): ShownColor[] {
  return (palette?.custom ?? []).map((color) => ({
    slot: color.slot,
    name: color.name,
    swatch: cssOf(color.xy),
    builtIn: false,
    xy: color.xy,
  }));
}

/** A cycle in words: "Green › Orange › Red", where a name never breaks inside. */
export function cycleWords(names: readonly string[]): string {
  return names.map((name) => name.replaceAll(' ', '\u00a0')).join(' › ');
}

export const useColorsStore = defineStore('colors', () => {
  const errors = useErrorStore();

  /** Null until the backend answers. */
  const palette = ref<Palette | null>(null);

  const builtIn = computed(() => builtInColors(palette.value));
  const custom = computed(() => customColors(palette.value));
  const cycle = computed(() => palette.value?.cycle ?? []);
  const full = computed(() => custom.value.length >= CUSTOM_MAX);

  function color(slot: number): ShownColor | undefined {
    return builtIn.value[slot] ?? custom.value.find((shown) => shown.slot === slot);
  }

  /** The color on screen of a color that the app gives the lamp: a slot, or the chromaticity of a plugin. */
  function swatchOf(lampColor: LampColor): string | undefined {
    return 'xy' in lampColor ? cssOf(lampColor.xy) : color(lampColor.slot)?.swatch;
  }

  return {
    palette,
    builtIn,
    custom,
    cycle,
    full,
    color,
    swatchOf,
    saveCustom: (color: Parameters<typeof colorsSaveCustom>[0]) =>
      errors.attempt(colorsSaveCustom(color)),
    renameOriginal: (slot: number, name: string | null) =>
      errors.attempt(colorsRenameOriginal(slot, name)),
    setCycle: (slots: number[]) => errors.attempt(colorsSetCycle(slots)),
    remove: (slot: number) => errors.attempt(colorsDelete(slot)),
    importCode: (text: string) => errors.attempt(colorsImport(text)),
    /** Follows the backend events. Call once per window. */
    start: () => mirror(palette, 'colors-changed', colorsGet),
  };
});
