<!--
  SPDX-License-Identifier: GPL-3.0-or-later
  SPDX-FileCopyrightText: 2026 The Disled contributors

  The reinstall window, opened from the settings menu only: a firmware
  version of the user's choice, 2.x or the last 1.x to go back to Disled v1,
  then the four steps of the guided update. A version 1 lamp gets the same
  window, without the warning that the popup already shows. Without a list
  of versions, it looks for one as it opens.
-->
<script setup lang="ts">
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { computed, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { useFirmwareStore } from '@/stores/firmware';
import { useLampStore } from '@/stores/lamp';
import { stepOf } from '@/utils/flash';

const { t } = useI18n();
const lamp = useLampStore();
const firmware = useFirmwareStore();

const flash = computed(() => firmware.status.flash);
const idle = computed(() => flash.value.step === 'idle');
const failed = computed(() => flash.value.step === 'failed');
const finished = computed(() => flash.value.step === 'done' || failed.value);
const waitingForBootloader = computed(() => stepOf(flash.value) === 1);
const versions = computed(() => firmware.status.versions);

/** The checked version, the newest firmware 2.x when the window opens. */
const chosen = ref<string | null>(null);
watch(
  versions,
  (next) => {
    if (!chosen.value || !next.includes(chosen.value)) {
      chosen.value = firmware.status.latest ?? next[0] ?? null;
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

/** Back to Disled v1, from a lamp that is not there already. */
const rollback = computed(
  () => idle.value && chosen.value?.split('.')[0] === '1' && lamp.generation !== 'v1',
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
if (firmware.loaded) {
  lookForVersions();
} else {
  watch(() => firmware.loaded, lookForVersions, { once: true });
}

function install(): void {
  if (version.value) {
    void firmware.installManual(version.value);
  }
}

function close(): Promise<void> {
  return getCurrentWebviewWindow().close();
}
</script>

<template>
  <main class="flex min-h-screen flex-col gap-5 p-6">
    <header class="flex flex-col gap-1">
      <h1 class="text-lg font-bold text-highlighted">{{ t('lampInstall.title') }}</h1>
      <p v-if="summary" class="text-sm text-muted">{{ summary }}</p>
    </header>

    <template v-if="idle">
      <UpdateWaiting
        v-if="!versions.length && firmware.status.checking"
        :text="t('updates.checking')"
      />
      <template v-else-if="versions.length">
        <UpdateVersionPicker
          v-model="chosen"
          :versions="versions"
          :latest="firmware.status.latest"
          :current="current"
        />
        <UAlert
          v-if="rollback"
          color="warning"
          variant="subtle"
          icon="i-lucide-triangle-alert"
          :title="t('lampInstall.rollback.title')"
          :description="t('lampInstall.rollback.text')"
        />
        <p class="text-sm">{{ t('lampInstall.intro') }}</p>
      </template>
    </template>

    <UpdateProgress v-else />

    <footer class="mt-auto flex justify-end gap-2">
      <template v-if="idle">
        <UButton
          v-if="!versions.length"
          color="neutral"
          variant="outline"
          icon="i-lucide-refresh-cw"
          :loading="firmware.status.checking"
          @click="firmware.check"
        >
          {{ t('updates.check') }}
        </UButton>
        <UButton v-else-if="chosen" icon="i-lucide-download" @click="install">
          {{
            firmware.onLamp(chosen)
              ? t('lampInstall.reinstall', { version: chosen })
              : t('lampInstall.install', { version: chosen })
          }}
        </UButton>
      </template>
      <UButton
        v-if="waitingForBootloader"
        color="neutral"
        variant="outline"
        @click="firmware.cancel"
      >
        {{ t('common.cancel') }}
      </UButton>
      <UButton v-if="failed" icon="i-lucide-rotate-ccw" @click="install">
        {{ t('lampUpdate.retry') }}
      </UButton>
      <UButton v-if="finished" color="neutral" variant="outline" @click="close">
        {{ t('common.close') }}
      </UButton>
    </footer>
  </main>
</template>
