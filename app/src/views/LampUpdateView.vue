<!--
  SPDX-License-Identifier: GPL-3.0-or-later
  SPDX-FileCopyrightText: 2026 The Disled contributors

  The lamp update window. It guides the user through a firmware update, with
  the BOOT and RESET buttons and the Windows driver, while the backend
  downloads, checks and flashes the firmware.
-->
<script setup lang="ts">
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { computed, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { openLink } from '@/ipc/commands';
import { useFirmwareStore } from '@/stores/firmware';
import { useLampStore } from '@/stores/lamp';
import { stepOf, UPDATE_STEPS, type UpdateStep } from '@/utils/flash';

const STEP_ICONS: Record<UpdateStep, string> = {
  download: 'i-lucide-download',
  bootloader: 'i-lucide-usb',
  install: 'i-lucide-hard-drive-download',
  restart: 'i-lucide-rotate-ccw',
};

const { t } = useI18n();
const lamp = useLampStore();
const firmware = useFirmwareStore();

const flash = computed(() => firmware.status.flash);
const idle = computed(() => flash.value.step === 'idle');
const failed = computed(() => flash.value.step === 'failed');
const finished = computed(() => flash.value.step === 'done' || failed.value);
const waitingForBootloader = computed(() => stepOf(flash.value) === 1);

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

const summary = computed(() => {
  const { latest, error } = firmware.status;
  const version = lamp.info?.firmware_version;
  if (!latest) {
    return error ? t('lampUpdate.notFound', { detail: error }) : t('lampUpdate.noFirmware');
  }
  if (!lamp.connection.connected) {
    return t('lampUpdate.noLamp', { version: latest });
  }
  if (!version) {
    return t('lampUpdate.connecting');
  }
  if (firmware.updateForLamp) {
    return t('lampUpdate.newer', { current: version, next: firmware.updateForLamp });
  }
  return flash.value.step === 'done'
    ? t('lampUpdate.current', { version })
    : t('lampUpdate.already', { version });
});

function close(): Promise<void> {
  return getCurrentWebviewWindow().close();
}
</script>

<template>
  <main class="flex min-h-screen flex-col gap-5 p-6">
    <header class="flex flex-col gap-1">
      <h1 class="text-lg font-bold text-highlighted">{{ t('lampUpdate.title') }}</h1>
      <p class="text-sm text-muted">{{ summary }}</p>
    </header>

    <i18n-t
      v-if="idle && firmware.status.latest"
      keypath="lampUpdate.intro"
      tag="p"
      scope="global"
      class="text-sm"
    >
      <template #boot><strong>BOOT</strong></template>
      <template #reset><strong>RESET</strong></template>
    </i18n-t>

    <UTimeline :model-value="shown" :items="steps" size="sm">
      <template #step-description="{ item }">
        <UpdateStepDetail
          v-if="item.value === shown && !failed"
          :flash="flash"
          :latest="firmware.status.latest"
        />
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
        <i18n-t keypath="lampUpdate.notDamaged" tag="span" scope="global">
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

    <footer class="mt-auto flex justify-end gap-2">
      <template v-if="idle">
        <UButton
          v-if="!firmware.status.latest"
          color="neutral"
          variant="outline"
          icon="i-lucide-refresh-cw"
          :loading="firmware.status.checking"
          @click="firmware.check"
        >
          {{ t('updates.check') }}
        </UButton>
        <UButton v-else icon="i-lucide-download" @click="firmware.install">
          {{
            lamp.connection.connected && !firmware.updateForLamp
              ? t('lampUpdate.reinstall')
              : t('lampUpdate.start')
          }}
        </UButton>
      </template>
      <UButton
        v-if="waitingForBootloader"
        color="neutral"
        variant="outline"
        @click="firmware.cancel"
      >
        {{ t('common.cancel') }}
      </UButton>
      <UButton v-if="failed" icon="i-lucide-rotate-ccw" @click="firmware.install">
        {{ t('lampUpdate.retry') }}
      </UButton>
      <UButton v-if="finished" color="neutral" variant="outline" @click="close">
        {{ t('common.close') }}
      </UButton>
    </footer>
  </main>
</template>
