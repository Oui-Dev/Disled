<!--
  SPDX-License-Identifier: GPL-3.0-or-later
  SPDX-FileCopyrightText: 2026 The Disled contributors

  Brightness, automatic with a "darker / brighter" correction, or a fixed level, with
  an Auto switch. The slider moves only under the user's hand, and the level in use
  shows next to the switch. Settings, saved here and applied at every connection.
-->
<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { useHeldSlider } from '@/composables/useHeldSlider';
import { useCorrectionText } from '@/composables/useSignedPercent';
import { useLampStore } from '@/stores/lamp';
import { useSettingsStore } from '@/stores/settings';
import { AUTO_BIAS_MAX, AUTO_BIAS_MIN } from '@/utils/lamp';

/** The correction moves by this much, percent. */
const BIAS_STEP = 5;

const { t, n } = useI18n();
const lamp = useLampStore();
const settingsStore = useSettingsStore();
const correction = useCorrectionText();

const settings = computed(() => settingsStore.settings);
const auto = computed(() => settings.value?.brightness_mode !== 'manual');
/** A lamp the app does not drive, whose level the app does not know either. */
const locked = computed(() => !settings.value || lamp.locked);

/** The slider: the correction in automatic mode, the fixed level otherwise. */
const { value, move, release, drop } = useHeldSlider(
  () => (auto.value ? (settings.value?.auto_bias ?? 0) : (settings.value?.manual_level ?? 50)),
  (next) => (auto.value ? { auto_bias: next } : { brightness_mode: 'manual', manual_level: next }),
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
const biasText = computed(() => correction.text(value.value));

function setAuto(on: boolean): void {
  drop();
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
  return `${t('lamp.brightness.follows')} · ${correction.phrase(value.value)}`;
});
</script>

<template>
  <fieldset class="flex min-w-0 flex-col gap-3">
    <legend class="sr-only">{{ t('lamp.brightness.title') }}</legend>
    <div class="flex items-center justify-between gap-3">
      <span aria-hidden="true" class="text-highlighted text-sm font-semibold">
        {{ t('lamp.brightness.title') }}
      </span>
      <div class="flex items-center gap-3">
        <!-- In automatic mode, the level the lamp chose; a fixed level is read with its slider. -->
        <span
          v-if="levelText"
          :aria-hidden="!auto || undefined"
          class="text-muted text-sm tabular-nums"
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
        <span class="text-muted shrink-0 text-xs">{{ t('lamp.brightness.darker') }}</span>
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
        <span class="text-muted shrink-0 text-xs">{{ t('lamp.brightness.brighter') }}</span>
      </template>
      <template v-else>
        <UIcon name="i-lucide-sun-dim" class="text-muted size-4 shrink-0" />
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
        <UIcon name="i-lucide-sun" class="text-muted size-5 shrink-0" />
      </template>
    </div>

    <p class="text-muted text-xs">{{ hint }}</p>
  </fieldset>
</template>
