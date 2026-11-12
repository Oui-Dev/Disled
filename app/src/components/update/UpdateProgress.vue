<!--
  SPDX-License-Identifier: GPL-3.0-or-later
  SPDX-FileCopyrightText: 2026 The Disled contributors

  The course of a lamp update, the same in both update windows: its four
  steps, the detail of the current one, then how the update ended, with the
  update guide when it stopped.
-->
<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { openLink } from '@/ipc/commands';
import { useFirmwareStore } from '@/stores/firmware';
import { stepOf, UPDATE_STEPS, type UpdateStep } from '@/utils/flash';

const STEP_ICONS: Record<UpdateStep, string> = {
  download: 'i-lucide-download',
  bootloader: 'i-lucide-usb',
  install: 'i-lucide-hard-drive-download',
  restart: 'i-lucide-rotate-ccw',
};

const { t } = useI18n();
const firmware = useFirmwareStore();

const flash = computed(() => firmware.status.flash);
const failed = computed(() => flash.value.step === 'failed');
/** The version on its way to the lamp, or the one the guided window would install. */
const version = computed(() => firmware.status.target ?? firmware.status.latest);

/** The step on show: the current one, or the one a failed update stopped at. */
const shown = ref(-1);
watch(
  flash,
  (next) => {
    const step = stepOf(next);
    if (step !== null) {
      shown.value = step;
    } else if (next.step === 'idle') {
      shown.value = -1;
    }
  },
  { immediate: true },
);

const steps = computed(() =>
  UPDATE_STEPS.map((name, index) => {
    const stopped = failed.value && index === shown.value;
    return {
      value: index,
      title: t(`lampUpdate.steps.${name}`),
      icon: stopped ? 'i-lucide-x' : index < shown.value ? 'i-lucide-check' : STEP_ICONS[name],
      slot: 'step' as const,
      ui: stopped ? { indicator: 'group-data-[state=active]:bg-error' } : undefined,
    };
  }),
);
</script>

<template>
  <UTimeline :model-value="shown" :items="steps" size="sm">
    <template #step-description="{ item }">
      <UpdateStepDetail v-if="item.value === shown && !failed" :flash="flash" :version="version" />
    </template>
  </UTimeline>

  <UAlert
    v-if="flash.step === 'done'"
    color="success"
    variant="subtle"
    icon="i-lucide-circle-check"
    :description="t('lampUpdate.done', { version: flash.version })"
  />
  <UAlert
    v-if="flash.step === 'failed'"
    color="error"
    variant="subtle"
    icon="i-lucide-circle-x"
    :title="t('lampUpdate.failed', { detail: flash.error })"
    :ui="{ title: 'wrap-break-word' }"
  >
    <template #description>
      <i18n-t keypath="lampUpdate.retryOrGuide" tag="span" scope="global">
        <template #guide>
          <UButton
            variant="link"
            class="p-0"
            trailing-icon="i-lucide-external-link"
            @click="openLink('flashing_guide')"
          >
            {{ t('lampUpdate.guide') }}
          </UButton>
        </template>
      </i18n-t>
    </template>
  </UAlert>
</template>
