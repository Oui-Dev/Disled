<!--
  SPDX-License-Identifier: GPL-3.0-or-later
  SPDX-FileCopyrightText: 2026 The Disled contributors

  The editor of a custom color: its name, seven suggestions, a square with
  the hues across and the vividness up and down, and the preview on the
  lamp. The lamp lights every color with its largest channel at full, the
  brightness being set apart, so the square makes only such full colors,
  which the lamp shows as they are. The draft keeps the code of the color
  until the thumb moves or a suggestion is picked. The page saves through
  `save`, which the popup footer calls.
-->
<script setup lang="ts">
import { type Color, ColorAreaArea, ColorAreaRoot, ColorAreaThumb } from 'reka-ui';
import { computed, onBeforeUnmount, onMounted, ref, useTemplateRef, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { colorsPreview } from '@/ipc/commands';
import { useColorsStore } from '@/stores/colors';
import { useLampStore } from '@/stores/lamp';
import { useSettingsStore } from '@/stores/settings';
import { hexOf, hueAndVividness } from '@/utils/color';

const props = defineProps<{
  /** The color to change, or null for a new one. */
  slot: number | null;
  /** Saved from the "My colors" popover, a new color stays lit. */
  fromPopover: boolean;
}>();
const emit = defineEmits<{ done: [] }>();

/**
 * The suggestions of app 2.1, whose codes the comments keep, made full: the
 * light the lamp makes of each (light_hex in src-tauri/src/colors.rs), then
 * the code of its hue and vividness (hexOf of hueAndVividness). The square
 * makes these codes exactly, so the thumb lands on them.
 */
const SUGGESTIONS = [
  { key: 'yellow', hex: '#FFDC2E' }, // #F6D32D
  { key: 'cyan', hex: '#26E2FF' }, // #22D3EE
  { key: 'blue', hex: '#3D87FF' }, // #3B82F6
  { key: 'violet', hex: '#9061FF' }, // #8B5CF6
  { key: 'magenta', hex: '#FF45FF' }, // #E83FE8
  { key: 'pink', hex: '#FF70BA' }, // #FB6FB7
  { key: 'coolWhite', hex: '#EDF5FF' }, // #EEF5FF
] as const;

/** The preview follows the draft at most this often, as the brightness does. */
const PREVIEW_MS = 120;

const { t, n } = useI18n();
const colors = useColorsStore();
const lamp = useLampStore();
const settings = useSettingsStore();

const existing = colors.custom.find((color) => color.slot === props.slot);

/** The page opens with the focus in the name. */
const nameInput = useTemplateRef<{ inputRef: HTMLInputElement | null }>('nameInput');
onMounted(() => nameInput.value?.inputRef?.focus());

/**
 * The code that the square makes at the hue and vividness of a code. A
 * color saved from a suggestion of app 2.1 loads as its light, a step from
 * the code of the suggestion here, and lands on it so.
 */
function squareCode(code: string): string {
  const shade = hueAndVividness(code);
  return hexOf(shade.hue ?? 0, shade.vivid);
}
/** A new color starts on the first suggestion that no color of the user has on the square. */
const start =
  existing?.hex
  ?? (
    SUGGESTIONS.find(
      (suggestion) => !colors.custom.some((color) => squareCode(color.hex) === suggestion.hex),
    ) ?? SUGGESTIONS[0]
  ).hex;
/** A grey or a white has no hue: its thumb stands at hue 0, at the bottom. */
const startShade = hueAndVividness(start);

const name = ref(existing?.name ?? '');
const hex = ref(start);
const hue = ref(startShade.hue ?? 0);
const vivid = ref(startShade.vivid);
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

/**
 * The thumb of the square, for Reka: the hue and the vividness at full
 * brightness, in its HSB. A code would not place it, since Reka reads the
 * saturation of a code in HSL, where every full color is at 100 %.
 */
const point = computed<Color>(() => ({
  space: 'hsb',
  h: hue.value,
  s: vivid.value,
  b: 100,
  alpha: 1,
}));
/** What screen readers say of the thumb, as "Hue 210 degrees, vividness 60%". */
const valueText = computed(() =>
  t('colors.editor.square.value', {
    hue: n(hue.value, 'degrees'),
    vivid: n(vivid.value / 100, 'percent'),
  }),
);

/** A suggestion: its code, and the thumb on it. */
function pick(code: unknown): void {
  if (typeof code !== 'string') {
    return;
  }
  hex.value = code;
  const shade = hueAndVividness(code);
  hue.value = shade.hue ?? 0;
  vivid.value = shade.vivid;
}

/**
 * The thumb moved, under the pointer or a key. Reka gives its point as a
 * color whose `h` and `s` are the values of the two axes as they are, in
 * HSL for these channels whatever the color space, so its own code is not
 * this color. The draft rounds them, so that its code is always one that
 * hexOf makes, as the suggestions are, at full brightness. The hue runs to
 * 360 at the right edge, red again: hue 0.
 */
function move(color: Color): void {
  // Never RGB, for a hue and a saturation.
  if (color.space === 'rgb') {
    return;
  }
  hue.value = Math.round(color.h) % 360;
  vivid.value = Math.round(color.s);
  hex.value = hexOf(hue.value, vivid.value);
}

// The preview on the lamp: with the setting on and a lamp the app drives.
const previewing = computed(() => (settings.settings?.color_preview ?? true) && lamp.ready);
let lastSent = 0;
let pending: ReturnType<typeof setTimeout> | undefined;

function preview(code: string | null): void {
  clearTimeout(pending);
  pending = undefined;
  if (code === null) {
    void colorsPreview(null);
    return;
  }
  const wait = lastSent + PREVIEW_MS - Date.now();
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
watch(previewing, (on) => preview(on ? hex.value : null));
onMounted(() => {
  if (previewing.value) {
    preview(hex.value);
  }
});
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

    <!-- The square: the color area of Reka UI, hue 0 to 360 from left to
         right, both edges red, and vividness 100 at the top to 0 at the
         bottom. Reka moves the thumb with the pointer, which it keeps past
         the edges, and with the keys: the arrows by a degree or a percent,
         by ten with Shift, and Page Up, Page Down, Home and End by ten. Its
         texts for screen readers are in English, so ours replace them: Vue
         gives an attribute set here the last word over the one the part sets
         on its own element. The background is ours, below. The square has a
         height of its own, the thumb being out of the flow, and the thumb
         overflows only into the gaps around. -->
    <ColorAreaRoot
      :model-value="point"
      color-space="hsb"
      x-channel="hue"
      y-channel="saturation"
      @update:color="move"
    >
      <ColorAreaArea
        :aria-roledescription="t('colors.editor.square.role')"
        class="color-square relative h-36 rounded-md ring ring-accented select-none"
      >
        <!-- As the thumbs of the earlier sliders, a disc of the draft color
             in a 2px ring: white here, with a thin dark line, so that it
             shows on vivid and pale points alike, and past the edges in both
             themes. The focus ring keeps a band of the page color inside,
             which makes it show on every color too. -->
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
 * Every point shows hexOf(hue, vivid): the six hues across, under white
 * that goes from transparent at the top to opaque at the bottom. CSS mixes
 * these legacy colors in gamma-encoded sRGB, so a point at vividness s is
 * s * hue + (1 - s) * white, the HSV formula at value 100 % of hexOf.
 * Checked against hexOf: within one code step at every pixel, but for the
 * faint noise that the browser adds to gradients against banding. Reka's
 * own background for these axes is made for HSL, grey at the bottom.
 */
.color-square {
  background:
    linear-gradient(#fff0, #fff),
    linear-gradient(to right, #f00, #ff0, #0f0, #0ff, #00f, #f0f, #f00);
}
</style>
