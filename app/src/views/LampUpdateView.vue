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
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { useFlashSteps } from '@/composables/useFlashSteps';
import { useFirmwareStore } from '@/stores/firmware';
import { useLampStore } from '@/stores/lamp';

const { t } = useI18n();
const lamp = useLampStore();
const firmware = useFirmwareStore();
const { flash, idle } = useFlashSteps();

const summary = computed(() => {
  const { latest } = firmware;
  const { error } = firmware.status;
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

function install(): void {
  if (firmware.latest) {
    void firmware.install(firmware.latest);
  }
}
</script>

<template>
  <main class="flex min-h-screen flex-col gap-5 p-6">
    <header class="flex flex-col gap-1">
      <h1 class="text-lg font-bold text-highlighted">{{ t('lampUpdate.title') }}</h1>
      <p class="text-sm text-muted">{{ summary }}</p>
    </header>

    <i18n-t
      v-if="idle && firmware.latest"
      keypath="lampUpdate.intro"
      tag="p"
      scope="global"
      class="text-sm"
    >
      <template #boot><strong>BOOT</strong></template>
      <template #reset><strong>RESET</strong></template>
    </i18n-t>

    <UpdateProgress />

    <UpdateFooter @retry="install">
      <UButton icon="i-lucide-download" @click="install">
        {{
          lamp.connection.connected && !firmware.newerForLamp
            ? t('lampUpdate.reinstall')
            : t('lampUpdate.start')
        }}
      </UButton>
    </UpdateFooter>
  </main>
</template>
