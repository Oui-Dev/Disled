<!--
  SPDX-License-Identifier: GPL-3.0-or-later
  SPDX-FileCopyrightText: 2026 The Disled contributors

  The active step of a lamp update: what it does, or what it needs from the
  user, such as the BOOT and RESET buttons, the Windows driver or a device
  to unplug.
-->
<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import type { Flash } from '@/bindings/Flash';
import { openLink } from '@/ipc/commands';

defineProps<{ flash: Flash; version: string | null }>();

const { t } = useI18n();
</script>

<template>
  <div class="flex flex-col gap-2 pt-1 text-sm text-default">
    <p v-if="flash.step === 'downloading'" class="text-muted">
      {{ t('lampUpdate.downloading', { version }) }}
    </p>

    <template v-else-if="flash.step === 'waiting_for_bootloader'">
      <i18n-t keypath="lampUpdate.buttons" tag="p" scope="global">
        <template #boot><strong>BOOT</strong></template>
        <template #reset><strong>RESET</strong></template>
      </i18n-t>
      <UpdateWaiting :text="t('lampUpdate.waitingForBootloader')" />
    </template>

    <template v-else-if="flash.step === 'driver_missing'">
      <p>{{ t('lampUpdate.driver.intro') }}</p>
      <ol class="flex list-decimal flex-col gap-1 ps-5">
        <i18n-t keypath="lampUpdate.driver.download" tag="li" scope="global">
          <template #zadig>
            <UButton
              variant="link"
              class="p-0"
              trailing-icon="i-lucide-external-link"
              @click="openLink('zadig')"
            >
              Zadig
            </UButton>
          </template>
        </i18n-t>
        <i18n-t keypath="lampUpdate.driver.options" tag="li" scope="global">
          <template #options><em>Options</em></template>
          <template #listAll><em>List All Devices</em></template>
        </i18n-t>
        <i18n-t keypath="lampUpdate.driver.select" tag="li" scope="global">
          <template #id><code class="whitespace-nowrap">4348 55E0</code></template>
          <template #otherId><code class="whitespace-nowrap">1A86 55E0</code></template>
        </i18n-t>
        <i18n-t keypath="lampUpdate.driver.install" tag="li" scope="global">
          <template #winusb><em>WinUSB</em></template>
          <template #install><em>Install Driver</em></template>
          <template #replace><em>Replace Driver</em></template>
        </i18n-t>
      </ol>
      <UpdateWaiting :text="t('lampUpdate.driver.waiting')" />
    </template>

    <p v-else-if="flash.step === 'no_permission'">{{ t('lampUpdate.noPermission') }}</p>
    <p v-else-if="flash.step === 'several_bootloaders'">
      {{ t('lampUpdate.severalBootloaders') }}
    </p>
    <p v-else-if="flash.step === 'other_chip'">
      {{ t('lampUpdate.otherChip', { chip: flash.chip }) }}
    </p>

    <template v-else-if="flash.step === 'flashing'">
      <UpdateWaiting :text="t(`lampUpdate.stages.${flash.stage}`)" />
      <p class="font-medium">{{ t('lampUpdate.doNotUnplug') }}</p>
    </template>

    <UpdateWaiting
      v-else-if="flash.step === 'waiting_for_lamp'"
      :text="t('lampUpdate.waitingForLamp')"
    />
  </div>
</template>
