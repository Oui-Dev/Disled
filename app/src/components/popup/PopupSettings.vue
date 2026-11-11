<!--
  SPDX-License-Identifier: GPL-3.0-or-later
  SPDX-FileCopyrightText: 2026 The Disled contributors

  The settings of the popup: start at login, daily update checks, the touch
  sensitivity, color mode, language, and the versions under them. The rare actions are in the
  menu next to the title, PopupMenu.
-->
<script setup lang="ts">
import { getVersion } from '@tauri-apps/api/app';
import { computed, onMounted, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import type { ColorMode } from '@/bindings/ColorMode';
import type { Language } from '@/bindings/Language';
import { useLampStore } from '@/stores/lamp';
import { useSettingsStore } from '@/stores/settings';

const { t } = useI18n();
const lamp = useLampStore();
const settingsStore = useSettingsStore();
const settings = computed(() => settingsStore.settings);

const appVersion = ref('');
onMounted(async () => {
  appVersion.value = await getVersion();
});
const versions = computed(() =>
  t('settings.versions', { app: appVersion.value, firmware: lamp.info?.firmware_version ?? '–' }),
);

// A radio group emits the type of its items' values.
const colorModes = computed<{ value: ColorMode; label: string; icon: string }[]>(() => [
  { value: 'system', label: t('settings.colorModes.system'), icon: 'i-lucide-monitor' },
  { value: 'light', label: t('settings.colorModes.light'), icon: 'i-lucide-sun' },
  { value: 'dark', label: t('settings.colorModes.dark'), icon: 'i-lucide-moon' },
]);
// Each language under its own name, as language menus do.
const languages = computed<{ value: Language; label: string }[]>(() => [
  { value: 'system', label: t('settings.systemLanguage') },
  { value: 'en', label: 'English' },
  { value: 'fr', label: 'Français' },
]);

/** A few short choices side by side, as cards sharing the width. */
const cards = { legend: 'text-sm', fieldset: 'flex-nowrap gap-2', item: 'min-w-0 flex-1' };
</script>

<template>
  <div class="flex flex-col gap-4">
    <div class="flex flex-col gap-3">
      <USwitch
        :model-value="settingsStore.autostart"
        :label="t('settings.autostart')"
        size="sm"
        @update:model-value="settingsStore.setAutostart"
      />
      <USwitch
        :model-value="settings?.check_updates ?? true"
        :label="t('settings.checkUpdates')"
        :disabled="!settings"
        size="sm"
        @update:model-value="settingsStore.update({ check_updates: $event })"
      />
    </div>
    <PopupTouchSensitivity />

    <!-- Reka names each radio once, from its label: the group mounts again
         in a new language. Not the group of the languages, which would lose
         the focus of the keyboard that changes it; only its "System" keeps
         the former language, until the page closes. -->
    <URadioGroup
      :key="settingsStore.locale"
      :model-value="settings?.color_mode"
      :items="colorModes"
      :legend="t('settings.colorMode')"
      variant="card"
      orientation="horizontal"
      indicator="hidden"
      size="sm"
      :disabled="!settings"
      :ui="cards"
      @update:model-value="settingsStore.update({ color_mode: $event })"
    />
    <URadioGroup
      :model-value="settings?.language"
      :items="languages"
      :legend="t('settings.language')"
      variant="card"
      orientation="horizontal"
      indicator="hidden"
      size="sm"
      :disabled="!settings"
      :ui="cards"
      @update:model-value="settingsStore.update({ language: $event })"
    />

    <!-- A development firmware has a long version, so the line truncates and shows it whole on hover. -->
    <p class="truncate text-xs text-muted" :title="versions">{{ versions }}</p>
  </div>
</template>
