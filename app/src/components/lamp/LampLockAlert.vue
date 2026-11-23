<!--
  SPDX-License-Identifier: GPL-3.0-or-later
  SPDX-FileCopyrightText: 2026 The Disled contributors

  The lock of a lamp that the app does not drive, at the top of the lamp
  and settings pages. An older lamp, of protocol 1 or 2, needs a firmware
  3.x, which the guided update window installs. A lamp newer than the app
  needs a newer app.
-->
<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import { openLampUpdate } from '@/ipc/commands';
import { useLampStore } from '@/stores/lamp';

const { t } = useI18n();
const lamp = useLampStore();
</script>

<template>
  <UAlert
    v-if="lamp.generation === 'older'"
    color="warning"
    variant="subtle"
    icon="i-lucide-triangle-alert"
    orientation="vertical"
    :title="t('lampOlder.title')"
    :description="t('lampOlder.text')"
    :actions="[
      {
        label: t('lampOlder.action'),
        color: 'neutral',
        variant: 'solid',
        size: 'sm',
        onClick: () => void openLampUpdate(),
      },
    ]"
  />
  <UAlert
    v-else-if="lamp.generation === 'newer'"
    color="warning"
    variant="subtle"
    icon="i-lucide-triangle-alert"
    :description="t('lampNewer.text')"
  />
</template>
