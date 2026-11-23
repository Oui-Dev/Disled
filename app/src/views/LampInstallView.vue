<!--
  SPDX-License-Identifier: GPL-3.0-or-later
  SPDX-FileCopyrightText: 2026 The Disled contributors

  The reinstall window, opened from the settings menu only: a firmware 3.x
  of the user's choice, then the four steps of the guided update. Without a
  list of versions, it looks for one as it opens.
-->
<script setup lang="ts">
import { whenever } from '@vueuse/core';
import { computed, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { useFlashSteps } from '@/composables/useFlashSteps';
import { useFirmwareStore } from '@/stores/firmware';
import { useLampStore } from '@/stores/lamp';

const { t } = useI18n();
const lamp = useLampStore();
const firmware = useFirmwareStore();
const { flash, idle } = useFlashSteps();
const versions = computed(() => firmware.status.versions);

/** The checked version, the newest firmware 3.x when the window opens. */
const chosen = ref<string | null>(null);
watch(
  versions,
  (next) => {
    if (!chosen.value || !next.includes(chosen.value)) {
      chosen.value = next[0] ?? null;
    }
  },
  { immediate: true },
);

/**
 * The version on its way to the lamp once the update starts, the checked
 * one before, and after an update that the backend refused to start.
 */
const version = computed(() =>
  idle.value ? chosen.value : (firmware.status.target ?? chosen.value),
);

/** The version of the connected lamp, among those of the list. */
const current = computed(
  () => versions.value.find((candidate) => firmware.onLamp(candidate)) ?? null,
);

const summary = computed(() => {
  const { error, checking } = firmware.status;
  const next = version.value;
  if (!next) {
    if (checking) {
      return null;
    }
    return error ? t('lampUpdate.notFound', { detail: error }) : t('lampUpdate.noFirmware');
  }
  const installed = lamp.info?.firmware_version;
  if (!lamp.connection.connected) {
    return t('lampUpdate.noLamp', { version: next });
  }
  if (!installed) {
    return t('lampUpdate.connecting');
  }
  if (flash.value.step === 'done') {
    return t('lampUpdate.current', { version: installed });
  }
  return firmware.onLamp(next)
    ? t('lampUpdate.already', { version: installed })
    : t('lampUpdate.newer', { current: installed, next });
});

/**
 * Without a list, because the daily check is off or has not run yet, the
 * window looks for one as it opens, once it has the status. A debug build,
 * which never looks by itself, gets its list so too.
 */
function lookForVersions(): void {
  if (!versions.value.length && !firmware.status.checking) {
    void firmware.check();
  }
}
whenever(() => firmware.loaded, lookForVersions, { immediate: true, once: true });

function install(): void {
  if (version.value) {
    void firmware.install(version.value);
  }
}
</script>

<template>
  <main class="flex min-h-screen flex-col gap-5 p-6">
    <header class="flex flex-col gap-1">
      <h1 class="text-highlighted text-lg font-bold">{{ t('lampInstall.title') }}</h1>
      <p v-if="summary" class="text-muted text-sm">{{ summary }}</p>
    </header>

    <template v-if="idle">
      <WaitingLine
        v-if="!versions.length && firmware.status.checking"
        :text="t('updates.checking')"
      />
      <template v-else-if="versions.length">
        <UpdateVersionPicker
          v-model="chosen"
          :versions="versions"
          :latest="firmware.latest"
          :current="current"
        />
        <p class="text-sm">{{ t('lampInstall.intro') }}</p>
      </template>
    </template>

    <UpdateProgress v-else />

    <UpdateFooter @retry="install">
      <UButton v-if="chosen" icon="i-lucide-download" @click="install">
        {{
          firmware.onLamp(chosen)
            ? t('lampInstall.reinstall', { version: chosen })
            : t('lampInstall.install', { version: chosen })
        }}
      </UButton>
    </UpdateFooter>
  </main>
</template>
