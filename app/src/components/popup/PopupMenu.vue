<!--
  SPDX-License-Identifier: GPL-3.0-or-later
  SPDX-FileCopyrightText: 2026 The Disled contributors

  The "more" menu of the popup settings, for the rare actions: check for
  updates, open Diagnostics, reinstall a lamp firmware of the user's
  choice. Its last lines say where the updates stand.
-->
<script setup lang="ts">
import type { DropdownMenuItem } from '@nuxt/ui';
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { useUpdateStatus } from '@/composables/useUpdateStatus';
import { openDiagnostics, openLampInstall } from '@/ipc/commands';
import { useLampStore } from '@/stores/lamp';

const { t } = useI18n();
const { checking, appLine, lampLine, checkAll } = useUpdateStatus();
const lamp = useLampStore();

const items = computed(() => {
  const actions: DropdownMenuItem[] = [
    {
      label: t('updates.check'),
      icon: 'i-lucide-refresh-cw',
      loading: checking.value,
      // One check at a time, since a loading item still takes clicks.
      disabled: checking.value,
      // The menu stays open, so that its last lines show the outcome.
      onSelect: (event) => {
        event.preventDefault();
        void checkAll();
      },
    },
    {
      label: t('settings.diagnostics'),
      icon: 'i-lucide-activity',
      // A lamp the app does not drive has no diagnostics to show.
      disabled: lamp.locked,
      onSelect: () => void openDiagnostics(),
    },
    // The reinstall window takes every case: a lamp up to date, one that
    // no longer starts, which its update mode recovers, a version 1 lamp,
    // and no firmware known yet.
    {
      label: t('updates.reinstall'),
      description: t('updates.reinstallHint'),
      icon: 'i-lucide-hard-drive-download',
      onSelect: () => void openLampInstall(),
    },
  ];
  const status = [appLine.value, lampLine.value]
    .filter((line) => line !== null)
    // A literal slot name types the item in the status slot.
    .map((label) => ({ type: 'label' as const, slot: 'status' as const, label }));
  return status.length ? [actions, status] : [actions];
});
</script>

<template>
  <!-- Not modal, so that the popup behind it stays readable to screen readers. -->
  <UDropdownMenu
    :items="items"
    :modal="false"
    :content="{ align: 'end' }"
    size="sm"
    :ui="{
      content: 'max-w-72',
      label: 'font-normal text-muted',
      itemDescription: 'whitespace-normal',
    }"
  >
    <UButton
      color="neutral"
      variant="ghost"
      size="sm"
      icon="i-lucide-ellipsis-vertical"
      :aria-label="t('settings.more')"
    />
    <!-- The status lines wrap, since an error can be long. -->
    <template #status="{ item }">
      <span class="whitespace-normal">{{ item.label }}</span>
    </template>
  </UDropdownMenu>
</template>
