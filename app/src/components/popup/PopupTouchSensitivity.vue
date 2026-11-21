<!--
  SPDX-License-Identifier: GPL-3.0-or-later
  SPDX-FileCopyrightText: 2026 The Disled contributors

  The touch sensitivity, a correction of the thresholds that the lamp
  calibrates, from -50 to +50 % by steps of 5. A setting, saved here and sent
  to the lamp at every connection and every change; the lamp keeps it until
  it restarts. As the brightness correction, the slider moves only under the
  user's hand. Without a lamp, the value waits for the next one.
-->
<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { useHeldSlider } from '@/composables/useHeldSlider';
import { useCorrectionText } from '@/composables/useSignedPercent';
import { useLampStore } from '@/stores/lamp';
import { useSettingsStore } from '@/stores/settings';
import { TOUCH_SENSITIVITY_MAX, TOUCH_SENSITIVITY_MIN } from '@/utils/lamp';

/** The correction moves by this much, percent. */
const STEP = 5;

const { t } = useI18n();
const lamp = useLampStore();
const settingsStore = useSettingsStore();
const correction = useCorrectionText();

const settings = computed(() => settingsStore.settings);
/** Not before the settings are read, and not for a lamp the app does not drive. */
const locked = computed(() => !settings.value || lamp.locked);

const { value, move, release } = useHeldSlider(
  () => settings.value?.touch_sensitivity ?? 0,
  (percent) => ({ touch_sensitivity: percent }),
);
const valueText = computed(() => correction.text(value.value));
const hint = computed(() => `${t('settings.touch.hint')} · ${correction.phrase(value.value)}`);
</script>

<template>
  <fieldset class="flex min-w-0 flex-col gap-2">
    <legend class="text-default mb-2 text-sm font-medium">{{ t('settings.touch.title') }}</legend>
    <div class="flex h-5 items-center gap-2.5">
      <span class="text-muted shrink-0 text-xs">{{ t('settings.touch.less') }}</span>
      <USlider
        :model-value="value"
        :min="TOUCH_SENSITIVITY_MIN"
        :max="TOUCH_SENSITIVITY_MAX"
        :step="STEP"
        :disabled="locked"
        :tooltip="{ text: valueText }"
        :aria-label="t('settings.touch.label')"
        :aria-valuetext="valueText"
        aria-describedby="touch-sensitivity-hint"
        class="flex-1"
        @update:model-value="move"
        @change="release"
      />
      <span class="text-muted shrink-0 text-xs">{{ t('settings.touch.more') }}</span>
    </div>
    <p id="touch-sensitivity-hint" class="text-muted text-xs">{{ hint }}</p>
  </fieldset>
</template>
