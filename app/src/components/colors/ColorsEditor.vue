<!--
  SPDX-License-Identifier: GPL-3.0-or-later
  SPDX-FileCopyrightText: 2026 The Disled contributors

  The editor of a custom color: its name, seven suggestions, a square with
  the hues across and the vividness up and down, at full brightness, and
  the preview on the lamp. The square goes from the white of the lamp up to
  the colors of its LEDs themselves, beyond those of a screen, which shows
  them as near as it can (src/utils/color.ts). The draft keeps the
  chromaticity of the color until the thumb moves or a suggestion is
  picked. The page saves through `save`, which the popup footer calls.
-->
<script setup lang="ts">
import { type Color, ColorAreaArea, ColorAreaRoot, ColorAreaThumb, type HSBColor } from 'reka-ui';
import { computed, onBeforeUnmount, onMounted, ref, useTemplateRef, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import type { Xy } from '@/bindings/Xy';
import { colorsPreview } from '@/ipc/commands';
import { useColorsStore } from '@/stores/colors';
import { useLampStore } from '@/stores/lamp';
import { useSettingsStore } from '@/stores/settings';
import { cssOf, HUES, paintSquare, pointOf, xyOf } from '@/utils/color';
import { MOVE_UPDATE_MS } from '@/utils/lamp';

const props = defineProps<{
  /** The color to change, or null for a new one. */
  slot: number | null;
  /** Saved from the "My colors" popover, a new color stays lit. */
  fromPopover: boolean;
}>();
const emit = defineEmits<{ done: [] }>();

/**
 * Points of the square, nearest to the colors that the suggestions of app
 * 2.2 showed on the lamp: the thumb back on one checks it again.
 */
const SUGGESTIONS = [
  { key: 'yellow', hue: 50, vividness: 96 },
  { key: 'cyan', hue: 185, vividness: 86 },
  { key: 'blue', hue: 209, vividness: 78 },
  { key: 'violet', hue: 273, vividness: 55 },
  { key: 'magenta', hue: 308, vividness: 79 },
  { key: 'pink', hue: 323, vividness: 52 },
  { key: 'coolWhite', hue: 199, vividness: 5 },
] as const;

type Suggestion = (typeof SUGGESTIONS)[number];

const { t, n } = useI18n();
const colors = useColorsStore();
const lamp = useLampStore();
const settings = useSettingsStore();

const existing = colors.custom.find((color) => color.slot === props.slot);

/** The page opens with the focus in the name. */
const nameInput = useTemplateRef<{ inputRef: HTMLInputElement | null }>('nameInput');
onMounted(() => nameInput.value?.inputRef?.focus());

function same(a: Xy | null, b: Xy): boolean {
  return a !== null && a.x === b.x && a.y === b.y;
}

/** A new color starts on the first suggestion that no color of the user has. */
const start: Suggestion =
  SUGGESTIONS.find((suggestion) => {
    const xy = xyOf(lamp.gamut, suggestion.hue, suggestion.vividness);
    return !colors.custom.some((color) => same(color.xy, xy));
  }) ?? SUGGESTIONS[0];

const name = ref(existing?.name ?? '');
/** The thumb, at full brightness, as HSB: Reka's axes, hue and saturation. */
const point = ref<HSBColor>({ space: 'hsb', h: 0, s: 0, b: 100, alpha: 1 });
const xy = ref<Xy>(existing?.xy ?? xyOf(lamp.gamut, start.hue, start.vividness));
const swatch = computed(() => cssOf(xy.value));
const canSave = computed(() => name.value.trim().length > 0);

/** The thumb on a point, and the draft on its color unless told to keep it. */
function place(hue: number, vividness: number, { keep = false } = {}): void {
  point.value = { space: 'hsb', h: hue, s: vividness, b: 100, alpha: 1 };
  if (!keep) {
    xy.value = xyOf(lamp.gamut, hue, vividness);
  }
}

const placed = pointOf(lamp.gamut, xy.value);
place(placed.hue, placed.vividness, { keep: true });

const suggestion = computed(
  () =>
    SUGGESTIONS.find(
      (candidate) => candidate.hue === point.value.h && candidate.vividness === point.value.s,
    )?.key,
);
// Null among the values, which the group takes when the color is none of
// the suggestions.
const suggestions = computed<{ value: string | null; label: string; swatch: string }[]>(() =>
  SUGGESTIONS.map((item) => ({
    value: item.key,
    label: t(`colors.editor.suggestion.${item.key}`),
    swatch: cssOf(xyOf(lamp.gamut, item.hue, item.vividness)),
  })),
);

/** What screen readers say of the thumb, as "Hue 210 degrees, vividness 60%". */
const valueText = computed(() =>
  t('colors.editor.square.value', {
    hue: n(point.value.h, 'degrees'),
    vivid: n(point.value.s / 100, 'percent'),
  }),
);

/** A suggestion: the thumb on it, and its color. */
function pick(key: unknown): void {
  const chosen = SUGGESTIONS.find((item) => item.key === key);
  if (chosen) {
    place(chosen.hue, chosen.vividness);
  }
}

/**
 * The thumb moved, under the pointer or a key. Reka emits the axes as the
 * `h` and `s` of an HSL color: the draft takes the rounded axes, the right
 * edge (hue 360) being hue 0.
 */
function move(color: Color): void {
  // Never RGB, for a hue and a saturation.
  if (color.space === 'rgb') {
    return;
  }
  place(Math.round(color.h) % HUES, Math.round(color.s));
}

// The square, painted again when another lamp tells its gamut.
const square = useTemplateRef<HTMLCanvasElement>('square');
watch(
  () => [square.value, lamp.gamut] as const,
  ([canvas, gamut]) => {
    if (canvas) {
      paintSquare(canvas, gamut);
    }
  },
  { immediate: true },
);

// The preview on the lamp: with the setting on and a lamp the app drives.
const previewing = computed(() => (settings.settings?.color_preview ?? true) && lamp.ready);
let lastSent = 0;
let pending: ReturnType<typeof setTimeout> | undefined;

/**
 * Shows a color on the lamp, at most every MOVE_UPDATE_MS, or ends the
 * preview with null. Not useThrottleFn: a pending call must be dropped when
 * the preview ends or the color is saved.
 */
function preview(color: Xy | null): void {
  clearTimeout(pending);
  pending = undefined;
  if (color === null) {
    void colorsPreview(null);
    return;
  }
  const wait = lastSent + MOVE_UPDATE_MS - Date.now();
  if (wait > 0) {
    pending = setTimeout(() => preview(color), wait);
    return;
  }
  lastSent = Date.now();
  void colorsPreview(color);
}

watch(xy, (color) => {
  if (previewing.value) {
    preview(color);
  }
});
watch(previewing, (on) => preview(on ? xy.value : null), { immediate: true });
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
    xy: xy.value,
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
      class="text-highlighted text-sm font-semibold focus:outline-none"
    >
      {{ slot === null ? t('colors.editor.new') : t('colors.editor.edit') }}
    </h2>

    <div class="flex items-center gap-3">
      <span aria-hidden="true" class="swatch size-8 shrink-0" :style="{ background: swatch }" />
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
      }"
      @update:model-value="pick($event)"
    >
      <template #label="{ item }">
        <span
          :title="item.label"
          class="swatch-ring border-accented block size-6 rounded-full border"
          :style="{ background: item.swatch }"
        />
        <span class="sr-only">{{ item.label }}</span>
      </template>
    </URadioGroup>

    <!-- Reka's screen-reader texts are English: the attributes set here
         replace them. The square is ours, a canvas under the thumb: the
         colors of the lamp, whose gamut Reka does not know. -->
    <ColorAreaRoot :model-value="point" x-channel="hue" y-channel="saturation" @update:color="move">
      <ColorAreaArea
        :aria-roledescription="t('colors.editor.square.role')"
        class="ring-accented relative h-36 rounded-md ring select-none"
      >
        <canvas
          ref="square"
          aria-hidden="true"
          class="pointer-events-none absolute inset-0 size-full rounded-md"
        />
        <!-- A white ring with a dark line shows on every point, in both themes. -->
        <ColorAreaThumb
          :aria-roledescription="t('colors.editor.square.thumb')"
          :aria-label="t('colors.editor.square.label')"
          :aria-valuetext="valueText"
          class="focus-visible:ring-offset-bg focus-visible:ring-primary size-5 rounded-full border-2 border-white shadow-[0_0_0_1px_rgb(0_0_0/0.45)] focus-visible:ring-2 focus-visible:ring-offset-2 focus-visible:outline-hidden"
          :style="{ background: swatch }"
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
