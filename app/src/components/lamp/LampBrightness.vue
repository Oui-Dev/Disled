<!--
  SPDX-License-Identifier: GPL-3.0-or-later
  SPDX-FileCopyrightText: 2026 The Disled contributors

  Brightness, automatic with a "darker / brighter" correction, or a fixed level, with
  an Auto switch. The slider moves only under the user's hand, and the level in use
  shows next to the switch. Settings, saved here and applied at every connection.
-->
<script setup lang="ts">
import { useThrottleFn } from '@vueuse/core';
import { computed, nextTick, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import type { SettingsPatch } from '@/bindings/SettingsPatch';
import { useLampStore } from '@/stores/lamp';
import { useSettingsStore } from '@/stores/settings';
import { useSignedPercent } from '@/utils/format';
import { AUTO_BIAS_MAX, AUTO_BIAS_MIN } from '@/utils/lamp';

/** How often a moving slider updates the lamp. */
const MOVE_UPDATE_MS = 120;
/** The correction moves by this much, percent. */
const BIAS_STEP = 5;

const { t, n } = useI18n();
const lamp = useLampStore();
const settingsStore = useSettingsStore();
const signedPercent = useSignedPercent();

const settings = computed(() => settingsStore.settings);
const auto = computed(() => settings.value?.brightness_mode !== 'manual');
/** A lamp the app does not drive, whose level the app does not know either. */
const locked = computed(() => !settings.value || lamp.locked);

/** The value under the user's hand, kept until saved so that the thumb never jumps back. */
const held = ref<number | null>(null);
let moving = false;

/** The slider: the correction in automatic mode, the fixed level otherwise. */
const value = computed(
  () =>
    held.value
    ?? (auto.value ? (settings.value?.auto_bias ?? 0) : (settings.value?.manual_level ?? 50)),
);

/**
 * The level in use, in percent. In automatic mode it is the one the lamp
 * chose, null until the lamp reports it, and never shown for a locked lamp.
 */
const level = computed(() => {
  if (lamp.locked) {
    return null;
  }
  return auto.value ? (lamp.state?.applied_level ?? null) : value.value;
});
const levelText = computed(() =>
  level.value === null ? undefined : n(level.value / 100, 'percent'),
);
const biasText = computed(() =>
  value.value === 0 ? t('lamp.brightness.noCorrection') : signedPercent(value.value),
);

function patchFor(next: number): SettingsPatch {
  return auto.value ? { auto_bias: next } : { brightness_mode: 'manual', manual_level: next };
}

const saveWhileMoving = useThrottleFn(
  (patch: SettingsPatch) => settingsStore.update(patch),
  MOVE_UPDATE_MS,
  true,
);

function move(next: number | number[] | undefined): void {
  if (typeof next !== 'number') {
    return;
  }
  moving = true;
  held.value = next;
  void saveWhileMoving(patchFor(next));
}

async function release(): Promise<void> {
  // A key commits before it moves the slider, so wait for the move.
  await nextTick();
  moving = false;
  if (held.value === null) {
    return;
  }
  await settingsStore.update(patchFor(held.value));
  // The settings hold the value now, unless the hand came back meanwhile.
  if (!moving) {
    held.value = null;
  }
}

function setAuto(on: boolean): void {
  held.value = null;
  if (on) {
    void settingsStore.update({ brightness_mode: 'auto' });
  } else {
    // Keep the brightness the eye sees, by starting the fixed level at the level in use.
    const start = Math.max(1, lamp.state?.applied_level ?? settings.value?.manual_level ?? 50);
    void settingsStore.update({ brightness_mode: 'manual', manual_level: start });
  }
}

const hint = computed(() => {
  if (!auto.value) {
    return t('lamp.brightness.fixed');
  }
  const correction =
    value.value === 0 ? biasText.value : t('lamp.brightness.correction', { value: biasText.value });
  return `${t('lamp.brightness.follows')} · ${correction}`;
});
</script>

<template>
  <fieldset class="flex min-w-0 flex-col gap-3">
    <legend class="sr-only">{{ t('lamp.brightness.title') }}</legend>
    <div class="flex items-center justify-between gap-3">
      <span aria-hidden="true" class="text-sm font-semibold text-highlighted">
        {{ t('lamp.brightness.title') }}
      </span>
      <div class="flex items-center gap-3">
        <!-- In automatic mode, the level the lamp chose; a fixed level is read with its slider. -->
        <span
          v-if="levelText"
          :aria-hidden="!auto || undefined"
          class="text-sm text-muted tabular-nums"
        >
          {{ levelText }}
        </span>
        <USwitch
          :model-value="auto"
          :label="t('lamp.brightness.auto')"
          :disabled="locked"
          size="sm"
          @update:model-value="setAuto"
        />
      </div>
    </div>

    <!-- One row for both modes, as tall as the larger sun, so the slider and the hint stay put when the mode changes. -->
    <div class="flex h-5 items-center gap-2.5">
      <template v-if="auto">
        <span class="shrink-0 text-xs text-muted">{{ t('lamp.brightness.darker') }}</span>
        <USlider
          :model-value="value"
          :min="AUTO_BIAS_MIN"
          :max="AUTO_BIAS_MAX"
          :step="BIAS_STEP"
          :disabled="locked"
          :tooltip="{ text: biasText }"
          :aria-label="t('lamp.brightness.correctionLabel')"
          :aria-valuetext="biasText"
          class="flex-1"
          @update:model-value="move"
          @change="release"
        />
        <span class="shrink-0 text-xs text-muted">{{ t('lamp.brightness.brighter') }}</span>
      </template>
      <template v-else>
        <UIcon name="i-lucide-sun-dim" class="size-4 shrink-0 text-muted" />
        <USlider
          :model-value="value"
          :min="1"
          :max="100"
          :disabled="locked"
          :aria-label="t('lamp.brightness.title')"
          :aria-valuetext="levelText"
          class="flex-1"
          @update:model-value="move"
          @change="release"
        />
        <UIcon name="i-lucide-sun" class="size-5 shrink-0 text-muted" />
      </template>
    </div>

    <p class="text-xs text-muted">{{ hint }}</p>
  </fieldset>
</template>
