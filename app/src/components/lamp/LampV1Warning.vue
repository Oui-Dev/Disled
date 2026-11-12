<!--
  SPDX-License-Identifier: GPL-3.0-or-later
  SPDX-FileCopyrightText: 2026 The Disled contributors

  The lock of a lamp that the app does not drive, at the top of the lamp
  and settings pages. A version 1 lamp needs the firmware v2, which the
  guided update window installs. A lamp newer than the app needs a newer app.
-->
<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import { openLampUpdate } from '@/ipc/commands';
import { useLampStore } from '@/stores/lamp';

const { t } = useI18n();
const lamp = useLampStore();

const ui = { root: 'gap-2 p-3', icon: 'size-4', title: 'text-sm', description: 'text-xs' };
</script>

<template>
  <UAlert
    v-if="lamp.generation === 'v1'"
    color="warning"
    variant="subtle"
    icon="i-lucide-triangle-alert"
    orientation="vertical"
    :title="t('lampV1.title')"
    :description="t('lampV1.text')"
    :actions="[
      {
        label: t('lampV1.action'),
        color: 'neutral',
        variant: 'solid',
        size: 'sm',
        onClick: () => void openLampUpdate(),
      },
    ]"
    :ui="ui"
  />
  <UAlert
    v-else-if="lamp.generation === 'newer'"
    color="warning"
    variant="subtle"
    icon="i-lucide-triangle-alert"
    :description="t('lampNewer.text')"
    :ui="ui"
  />
</template>
