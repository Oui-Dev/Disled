<!--
  SPDX-License-Identifier: GPL-3.0-or-later
  SPDX-FileCopyrightText: 2026 The Disled contributors

  Lamp tab of the Diagnostics window: off and the four colors, the
  brightness, as in the popup, and the details of the last STATE.
-->
<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import { useLampStore } from '@/stores/lamp';

const { t } = useI18n();
const lamp = useLampStore();
</script>

<template>
  <p v-if="!lamp.connection.connected" class="text-sm text-muted">
    {{ t('diagnostics.noLamp') }}
  </p>
  <p v-else-if="!lamp.state" class="text-sm text-muted">{{ t('diagnostics.waiting') }}</p>
  <div v-else class="flex flex-col gap-4">
    <LampColorPicker standalone />
    <LampBrightness />
    <p class="text-xs text-muted">
      {{
        t('diagnostics.state', {
          seq: lamp.state.state_seq,
          source: t(`enums.changeSource.${lamp.state.source}`),
          link: t(`enums.usbLink.${lamp.state.link}`),
        })
      }}
    </p>
  </div>
</template>
