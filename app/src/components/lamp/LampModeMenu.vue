<!--
  SPDX-License-Identifier: GPL-3.0-or-later
  SPDX-FileCopyrightText: 2026 The Disled contributors

  The source the lamp follows, at the top right of the lamp page: the user
  in manual mode, or a single integration that picks the color, which tints
  the button. Its menu also holds the calls, which follow the microphone in
  manual mode only, where this system can tell a call, and the way to the
  integrations window. A setting of the app, so active without a lamp; a
  lamp the app does not drive locks it.
-->
<script setup lang="ts">
import type { DropdownMenuItem } from '@nuxt/ui';
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { openIntegrations } from '@/ipc/commands';
import { useControlStore } from '@/stores/control';
import { useLampStore } from '@/stores/lamp';
import { useSettingsStore } from '@/stores/settings';
import { pluginIcon } from '@/utils/pluginIcons';

const { t } = useI18n();
const lamp = useLampStore();
const control = useControlStore();
const settingsStore = useSettingsStore();

const integration = computed(() => control.integration);
const mic = computed(() => settingsStore.settings?.calls.mic ?? false);
const source = computed(() => integration.value?.name ?? t('modes.manual'));
const icon = computed(() =>
  integration.value ? pluginIcon(integration.value.icon) : 'i-lucide-hand',
);

const items = computed(() => {
  const manual = control.status.mode === null;
  // Checkbox items, one checked, since the menu has no radio items.
  const sources: DropdownMenuItem[] = [
    { type: 'label', label: t('modes.label') },
    {
      type: 'checkbox',
      label: t('modes.manual'),
      description: t('modes.manualHint'),
      icon: 'i-lucide-hand',
      checked: manual,
      onSelect: () => void control.setMode(null),
    },
  ];
  if (control.status.calls.available) {
    sources.push(
      {
        type: 'label',
        label: t('calls.title'),
        description: manual ? undefined : t('calls.manualOnly'),
        class: 'ps-8',
      },
      {
        type: 'checkbox',
        label: t('calls.mic'),
        icon: 'i-lucide-mic',
        checked: mic.value,
        disabled: !manual,
        // A literal slot name draws the switch on the right.
        slot: 'mic' as const,
        class: 'ps-8',
        ui: { itemTrailingIcon: 'hidden' },
        // The menu stays open, so that the switch shows the change.
        onSelect: (event) => {
          event.preventDefault();
          void settingsStore.update({ calls_mic: !mic.value });
        },
      },
    );
  }
  const groups = [sources];
  if (control.status.integrations.length) {
    groups.push([
      { type: 'label', label: t('modes.integrations') },
      ...control.status.integrations.map((known) => ({
        type: 'checkbox' as const,
        label: known.name,
        icon: pluginIcon(known.icon),
        checked: control.status.mode === known.id,
        slot: 'integration' as const,
        onSelect: () => void control.setMode(known.id),
      })),
    ]);
  }
  groups.push([
    {
      label: t('modes.manage'),
      icon: 'i-lucide-settings-2',
      onSelect: () => void openIntegrations(),
    },
  ]);
  return groups;
});
</script>

<template>
  <!-- Not modal, as the settings menu, so that the popup behind it stays readable. -->
  <UDropdownMenu
    :items="items"
    :modal="false"
    :content="{ align: 'end' }"
    size="sm"
    :ui="{ content: 'w-70', label: 'font-semibold', itemDescription: 'whitespace-normal' }"
  >
    <!-- Tinted primary while an integration drives the lamp. -->
    <UButton
      :color="integration ? 'primary' : 'neutral'"
      :variant="integration ? 'subtle' : 'outline'"
      size="xs"
      :icon="icon"
      :label="source"
      trailing-icon="i-lucide-chevron-down"
      :disabled="lamp.locked"
      :aria-label="t('modes.button', { source })"
      class="-mt-0.5 h-7 shrink-0 gap-1.5 ps-2 pe-1.5 whitespace-nowrap"
      :class="{ 'text-highlighted ring-primary/35': integration }"
      :ui="{
        leadingIcon: integration ? 'text-primary' : 'text-muted',
        trailingIcon: 'size-3.5 text-muted',
      }"
    />

    <template #mic-trailing>
      <USwitch
        :model-value="mic"
        size="xs"
        tabindex="-1"
        aria-hidden="true"
        class="pointer-events-none"
      />
    </template>
    <template #integration-trailing>
      <UBadge color="neutral" variant="outline" size="sm" :label="t('modes.pluginTag')" />
    </template>
  </UDropdownMenu>
</template>
