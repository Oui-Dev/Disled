<!--
  SPDX-License-Identifier: GPL-3.0-or-later
  SPDX-FileCopyrightText: 2026 The Disled contributors

  One page for every window. The window label picks the view, loaded on its
  own, and the stores it follows; main.ts has started the settings.
-->
<script setup lang="ts">
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { useColorMode } from '@vueuse/core';
import { type Component, computed, defineAsyncComponent, onMounted, watch, watchEffect } from 'vue';
import { nuxtUiLocales, setLocale } from '@/i18n';
import { useColorsStore } from '@/stores/colors';
import { useDiagnosticsStore } from '@/stores/diagnostics';
import { useFirmwareStore } from '@/stores/firmware';
import { useLampStore } from '@/stores/lamp';
import { useSettingsStore } from '@/stores/settings';
import { useTelemetryStore } from '@/stores/telemetry';
import { useUpdatesStore } from '@/stores/updates';

interface WindowSetup {
  view: Component;
  stores: (() => { start(): Promise<void> })[];
}

/** The windows of src-tauri/src/windows.rs. */
const WINDOWS: Record<string, WindowSetup> = {
  popup: {
    view: defineAsyncComponent(() => import('@/views/PopupView.vue')),
    stores: [useLampStore, useColorsStore, useFirmwareStore, useUpdatesStore],
  },
  diagnostics: {
    view: defineAsyncComponent(() => import('@/views/DiagnosticsView.vue')),
    stores: [useLampStore, useColorsStore, useTelemetryStore, useDiagnosticsStore],
  },
  'lamp-update': {
    view: defineAsyncComponent(() => import('@/views/LampUpdateView.vue')),
    stores: [useLampStore, useFirmwareStore],
  },
};

const current = WINDOWS[getCurrentWebviewWindow().label] ?? WINDOWS.diagnostics;
const settings = useSettingsStore();

// The color mode and the language of the settings, in this window. Nuxt UI
// follows the `dark` class on <html>; the backend applies the color mode to
// the window frames.
const { store: colorMode } = useColorMode();
watchEffect(() => {
  const mode = settings.settings?.color_mode;
  if (mode) {
    colorMode.value = mode === 'system' ? 'auto' : mode;
  }
});
watch(() => settings.locale, setLocale, { immediate: true });
const nuxtUiLocale = computed(() => nuxtUiLocales[settings.locale]);

onMounted(() => {
  for (const useStore of current.stores) {
    void useStore().start();
  }
});
</script>

<template>
  <UApp :locale="nuxtUiLocale" :toaster="null">
    <component :is="current.view" />
  </UApp>
</template>
