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
import { useThrottleFn } from '@vueuse/core';
import { computed, nextTick, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { useLampStore } from '@/stores/lamp';
import { useSettingsStore } from '@/stores/settings';
import { useSignedPercent } from '@/utils/format';

/** How often a moving slider updates the lamp, as the brightness does. */
const MOVE_UPDATE_MS = 120;
/** The correction moves by this much, percent. */
const STEP = 5;
/** Limits of SET_TOUCH_SENSITIVITY, in percent. */
const MIN = -50;
const MAX = 50;

const { t } = useI18n();
const lamp = useLampStore();
const settingsStore = useSettingsStore();
const signedPercent = useSignedPercent();

const settings = computed(() => settingsStore.settings);
/** Not before the settings are read, and not for a lamp the app does not drive. */
const locked = computed(() => !settings.value || lamp.locked);

/** The value under the user's hand, kept until saved so that the thumb never jumps back. */
const held = ref<number | null>(null);
let moving = false;

const value = computed(() => held.value ?? settings.value?.touch_sensitivity ?? 0);
const valueText = computed(() =>
  value.value === 0 ? t('lamp.brightness.noCorrection') : signedPercent(value.value),
);
const hint = computed(() => {
  const correction =
    value.value === 0
      ? valueText.value
      : t('lamp.brightness.correction', { value: valueText.value });
  return `${t('settings.touch.hint')} · ${correction}`;
});

const saveWhileMoving = useThrottleFn(
  (percent: number) => settingsStore.update({ touch_sensitivity: percent }),
  MOVE_UPDATE_MS,
  true,
);

function move(next: number | number[] | undefined): void {
  if (typeof next !== 'number') {
    return;
  }
  moving = true;
  held.value = next;
  void saveWhileMoving(next);
}

async function release(): Promise<void> {
  // A key commits before it moves the slider, so wait for the move.
  await nextTick();
  moving = false;
  if (held.value === null) {
    return;
  }
  await settingsStore.update({ touch_sensitivity: held.value });
  // The settings hold the value now, unless the hand came back meanwhile.
  if (!moving) {
    held.value = null;
  }
}
</script>

<template>
  <fieldset class="flex min-w-0 flex-col gap-2">
    <legend class="mb-2 text-sm font-medium text-default">{{ t('settings.touch.title') }}</legend>
    <div class="flex h-5 items-center gap-2.5">
      <span class="shrink-0 text-xs text-muted">{{ t('settings.touch.less') }}</span>
      <USlider
        :model-value="value"
        :min="MIN"
        :max="MAX"
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
      <span class="shrink-0 text-xs text-muted">{{ t('settings.touch.more') }}</span>
    </div>
    <p id="touch-sensitivity-hint" class="text-xs text-muted">{{ hint }}</p>
  </fieldset>
</template>
