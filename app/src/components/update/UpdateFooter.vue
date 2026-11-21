<!--
  SPDX-License-Identifier: GPL-3.0-or-later
  SPDX-FileCopyrightText: 2026 The Disled contributors

  The buttons of both update windows. Before an update, the check while no
  firmware is known, the install button of the slot once one is; Cancel
  while the update waits for the bootloader; Try again, which the window
  answers, and Close once it ended.
-->
<script setup lang="ts">
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { useI18n } from 'vue-i18n';
import { useFlashSteps } from '@/composables/useFlashSteps';
import { useFirmwareStore } from '@/stores/firmware';

const emit = defineEmits<{ retry: [] }>();

const { t } = useI18n();
const firmware = useFirmwareStore();
const { idle, failed, finished, waitingForBootloader } = useFlashSteps();

function close(): Promise<void> {
  return getCurrentWebviewWindow().close();
}
</script>

<template>
  <footer class="mt-auto flex justify-end gap-2">
    <template v-if="idle">
      <UButton
        v-if="!firmware.latest"
        color="neutral"
        variant="outline"
        icon="i-lucide-refresh-cw"
        :loading="firmware.status.checking"
        @click="firmware.check"
      >
        {{ t('updates.check') }}
      </UButton>
      <slot v-else />
    </template>
    <UButton v-if="waitingForBootloader" color="neutral" variant="outline" @click="firmware.cancel">
      {{ t('common.cancel') }}
    </UButton>
    <UButton v-if="failed" icon="i-lucide-rotate-ccw" @click="emit('retry')">
      {{ t('lampUpdate.retry') }}
    </UButton>
    <UButton v-if="finished" color="neutral" variant="outline" @click="close">
      {{ t('common.close') }}
    </UButton>
  </footer>
</template>
