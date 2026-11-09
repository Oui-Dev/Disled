<!--
  SPDX-License-Identifier: GPL-3.0-or-later
  SPDX-FileCopyrightText: 2026 The Disled contributors

  The guided lamp update, opened by the two "Update" buttons: the offer to a
  version 2 lamp and the warning of a version 1 lamp. It installs the newest
  firmware 2.x and guides the user through it, with the BOOT and RESET
  buttons and the Windows driver, while the backend downloads, checks,
  erases and flashes.
-->
<script setup lang="ts">
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { useFirmwareStore } from '@/stores/firmware';
import { useLampStore } from '@/stores/lamp';
import { stepOf } from '@/utils/flash';

const { t } = useI18n();
const lamp = useLampStore();
const firmware = useFirmwareStore();

const flash = computed(() => firmware.status.flash);
const idle = computed(() => flash.value.step === 'idle');
const failed = computed(() => flash.value.step === 'failed');
const finished = computed(() => flash.value.step === 'done' || failed.value);
const waitingForBootloader = computed(() => stepOf(flash.value) === 1);

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
  if (firmware.newerForLamp) {
    return t('lampUpdate.newer', { current: version, next: firmware.newerForLamp });
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

    <UpdateProgress />

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
            lamp.connection.connected && !firmware.newerForLamp
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
