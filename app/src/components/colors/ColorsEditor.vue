<!--
  SPDX-License-Identifier: GPL-3.0-or-later
  SPDX-FileCopyrightText: 2026 The Disled contributors

  The editor of a custom color: its name, seven suggestions, a square with
  the hues across and the vividness up and down, at full brightness, and
  the preview on the lamp. The lamp shows such full colors as they are
  (light_hex in src-tauri/src/colors.rs). The draft keeps the code of the
  color until the thumb moves or a suggestion is picked. The page saves
  through `save`, which the popup footer calls.
-->
<script setup lang="ts">
import {
  type Color,
  ColorAreaArea,
  ColorAreaRoot,
  ColorAreaThumb,
  colorToHex,
  convertToHsb,
  type HSBColor,
  parseColor,
} from 'reka-ui';
import { computed, onBeforeUnmount, onMounted, ref, useTemplateRef, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { colorsPreview } from '@/ipc/commands';
import { useColorsStore } from '@/stores/colors';
import { useLampStore } from '@/stores/lamp';
import { useSettingsStore } from '@/stores/settings';
import { MOVE_UPDATE_MS } from '@/utils/lamp';

const props = defineProps<{
  /** The color to change, or null for a new one. */
  slot: number | null;
  /** Saved from the "My colors" popover, a new color stays lit. */
  fromPopover: boolean;
}>();
const emit = defineEmits<{ done: [] }>();

/** Codes that the square makes: the thumb back on one checks it again. */
const SUGGESTIONS = [
  { key: 'yellow', hex: '#FFDC2E' },
  { key: 'cyan', hex: '#26E2FF' },
  { key: 'blue', hex: '#3D87FF' },
  { key: 'violet', hex: '#9061FF' },
  { key: 'magenta', hex: '#FF45FF' },
  { key: 'pink', hex: '#FF70BA' },
  { key: 'coolWhite', hex: '#EDF5FF' },
] as const;

const { t, n } = useI18n();
const colors = useColorsStore();
const lamp = useLampStore();
const settings = useSettingsStore();

const existing = colors.custom.find((color) => color.slot === props.slot);

/** The page opens with the focus in the name. */
const nameInput = useTemplateRef<{ inputRef: HTMLInputElement | null }>('nameInput');
onMounted(() => nameInput.value?.inputRef?.focus());

/**
 * The thumb on a code, at full brightness, as HSB: Reka reads the
 * saturation of a code in HSL, where every full color is at 100 %. A grey
 * or a white stands at hue 0, at the bottom.
 */
function pointOf(code: string): HSBColor {
  const { h, s } = convertToHsb(parseColor(code));
  return { space: 'hsb', h: Math.round(h) % 360, s: Math.round(s), b: 100, alpha: 1 };
}

/** A new color starts on the first suggestion that no color of the user has. */
const start =
  existing?.hex
  ?? (
    SUGGESTIONS.find((suggestion) => !colors.custom.some((color) => color.hex === suggestion.hex))
    ?? SUGGESTIONS[0]
  ).hex;

const name = ref(existing?.name ?? '');
const hex = ref(start);
const point = ref(pointOf(start));
const canSave = computed(() => name.value.trim().length > 0);
const suggestion = computed(
  () => SUGGESTIONS.find((candidate) => candidate.hex === hex.value)?.hex,
);
// Null among the values, which the group takes when the color is none of
// the suggestions.
const suggestions = computed<{ value: string | null; label: string }[]>(() =>
  SUGGESTIONS.map((item) => ({
    value: item.hex,
    label: t(`colors.editor.suggestion.${item.key}`),
  })),
);

/** What screen readers say of the thumb, as "Hue 210 degrees, vividness 60%". */
const valueText = computed(() =>
  t('colors.editor.square.value', {
    hue: n(point.value.h, 'degrees'),
    vivid: n(point.value.s / 100, 'percent'),
  }),
);

/** A suggestion: its code, and the thumb on it. */
function pick(code: unknown): void {
  if (typeof code !== 'string') {
    return;
  }
  hex.value = code;
  point.value = pointOf(code);
}

/**
 * The thumb moved, under the pointer or a key. Reka emits the axes as the
 * `h` and `s` of an HSL color, so its code is not this color: the draft
 * takes the rounded axes, the right edge (hue 360) being hue 0.
 */
function move(color: Color): void {
  // Never RGB, for a hue and a saturation.
  if (color.space === 'rgb') {
    return;
  }
  const h = Math.round(color.h) % 360;
  point.value = { space: 'hsb', h, s: Math.round(color.s), b: 100, alpha: 1 };
  hex.value = colorToHex(point.value).toUpperCase();
}

// The preview on the lamp: with the setting on and a lamp the app drives.
const previewing = computed(() => (settings.settings?.color_preview ?? true) && lamp.ready);
let lastSent = 0;
let pending: ReturnType<typeof setTimeout> | undefined;

/**
 * Shows a code on the lamp, at most every MOVE_UPDATE_MS, or ends the
 * preview with null. Not useThrottleFn: a pending call must be dropped when
 * the preview ends or the color is saved.
 */
function preview(code: string | null): void {
  clearTimeout(pending);
  pending = undefined;
  if (code === null) {
    void colorsPreview(null);
    return;
  }
  const wait = lastSent + MOVE_UPDATE_MS - Date.now();
  if (wait > 0) {
    pending = setTimeout(() => preview(code), wait);
    return;
  }
  lastSent = Date.now();
  void colorsPreview(code);
}

watch(hex, (code) => {
  if (previewing.value) {
    preview(code);
  }
});
watch(previewing, (on) => preview(on ? hex.value : null), { immediate: true });
// Cancel, Escape and a hidden popup all leave the page: the lamp goes back
// to its color. After a save, the backend has ended the preview already.
onBeforeUnmount(() => preview(null));

function setPreview(on: boolean): void {
  void settings.update({ color_preview: on });
}

async function save(): Promise<void> {
  if (!canSave.value) {
    return;
  }
  clearTimeout(pending);
  const slot = await colors.saveCustom({
    slot: props.slot,
    name: name.value,
    hex: hex.value,
    keepLit: props.fromPopover,
  });
  if (slot !== undefined) {
    emit('done');
  }
}

defineExpose({ save, canSave });
</script>

<template>
  <div class="flex flex-col gap-4">
    <h2
      data-page-heading
      tabindex="-1"
      class="text-sm font-semibold text-highlighted focus:outline-none"
    >
      {{ slot === null ? t('colors.editor.new') : t('colors.editor.edit') }}
    </h2>

    <div class="flex items-center gap-3">
      <span
        aria-hidden="true"
        class="size-8 shrink-0 rounded-[20%] border border-accented"
        :style="{ background: hex }"
      />
      <UInput
        ref="nameInput"
        v-model="name"
        maxlength="16"
        :aria-label="t('colors.editor.name')"
        :placeholder="t('colors.editor.name')"
        class="flex-1"
        @keydown.enter="save"
      />
    </div>

    <!-- Never without a value: a radio group mounted without one keeps its
         own selection, through Reka's passive v-model, and one mounted again
         at each choice would lose the focus of the arrow keys. -->
    <URadioGroup
      :model-value="suggestion ?? null"
      :items="suggestions"
      :legend="t('colors.editor.suggestions')"
      orientation="horizontal"
      indicator="hidden"
      :ui="{
        legend: 'sr-only',
        fieldset: 'flex-nowrap justify-between gap-x-0',
        item: 'group p-0',
        label: 'cursor-pointer',
      }"
      @update:model-value="pick($event)"
    >
      <template #label="{ item }">
        <span
          :title="item.label"
          class="block size-6 rounded-full border border-accented ring-2 ring-transparent ring-offset-2 ring-offset-bg transition-shadow group-has-data-[state=checked]:ring-inverted"
          :style="{ background: String(item.value) }"
        />
        <span class="sr-only">{{ item.label }}</span>
      </template>
    </URadioGroup>

    <!-- Reka's screen-reader texts are English: the attributes set here
         replace them. The background is ours (style below): Reka's, for
         these axes, turns grey at the bottom. -->
    <ColorAreaRoot :model-value="point" x-channel="hue" y-channel="saturation" @update:color="move">
      <ColorAreaArea
        :aria-roledescription="t('colors.editor.square.role')"
        class="color-square relative h-36 rounded-md ring ring-accented select-none"
      >
        <!-- A white ring with a dark line shows on every point, in both themes. -->
        <ColorAreaThumb
          :aria-roledescription="t('colors.editor.square.thumb')"
          :aria-label="t('colors.editor.square.label')"
          :aria-valuetext="valueText"
          class="size-5 rounded-full border-2 border-white shadow-[0_0_0_1px_rgb(0_0_0/0.45)] focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-2 focus-visible:ring-offset-bg"
          :style="{ background: hex }"
        />
      </ColorAreaArea>
    </ColorAreaRoot>

    <USwitch
      :model-value="settings.settings?.color_preview ?? true"
      :label="t('colors.editor.preview')"
      size="sm"
      @update:model-value="setPreview"
    />
  </div>
</template>

<style scoped>
/*
 * The hues across, under white from transparent to opaque downwards. CSS
 * mixes these colors in gamma-encoded sRGB, so a point at vividness s
 * shows s * hue + (1 - s) * white: the code that the square makes there.
 */
.color-square {
  background:
    linear-gradient(#fff0, #fff),
    linear-gradient(to right, #f00, #ff0, #0f0, #0ff, #00f, #f0f, #f00);
}
</style>
