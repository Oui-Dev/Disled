<!--
  SPDX-License-Identifier: GPL-3.0-or-later
  SPDX-FileCopyrightText: 2026 The Disled contributors

  Top of the Diagnostics window: connection, firmware and identity of the
  lamp, its restart, the last crash and the last error.
-->
<script setup lang="ts">
import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { useErrorText } from '@/composables/useErrorText';
import { useFaultText } from '@/composables/useFaultText';
import { useErrorStore } from '@/stores/errors';
import { useLampStore } from '@/stores/lamp';

const { t } = useI18n();
const lamp = useLampStore();
const errors = useErrorStore();
const errorText = useErrorText();
const faultText = useFaultText();

const connected = computed(() => lamp.connection.connected);
const crash = computed(() => (lamp.info?.fault ? faultText(lamp.info.fault) : null));

/** Compact alerts, as in the popup, so the header stays a bar. */
const alert = { root: 'gap-2 p-3', icon: 'size-4', description: 'text-sm wrap-break-word' };

const confirmingRestart = ref(false);

function restart(): Promise<void> {
  confirmingRestart.value = false;
  return lamp.restart();
}
</script>

<template>
  <header class="flex flex-col gap-3">
    <div class="flex items-start justify-between gap-4">
      <div class="flex min-w-0 flex-col gap-1">
        <div class="flex flex-wrap items-center gap-3">
          <h1 class="text-lg font-bold tracking-wide text-highlighted">DISLED</h1>
          <UBadge
            :color="connected ? 'primary' : 'neutral'"
            variant="subtle"
            :icon="connected ? 'i-lucide-usb' : 'i-lucide-unplug'"
            :label="connected ? t('diagnostics.connected') : t('status.noLamp')"
          />
          <template v-if="connected">
            <span class="text-sm text-muted">
              {{ t('diagnostics.firmware', { version: lamp.info?.firmware_version ?? '…' }) }}
            </span>
            <!-- Neutral text, because amber text on its tint is not readable; the icon carries the color. -->
            <UBadge
              v-if="lamp.connection.debug_build"
              color="neutral"
              variant="subtle"
              icon="i-lucide-bug"
              :label="t('diagnostics.debugBuild')"
              :ui="{ leadingIcon: 'text-warning' }"
            />
          </template>
        </div>
        <p v-if="connected" class="text-xs text-muted">
          {{ t('diagnostics.serial', { serial: lamp.connection.serial ?? '?' }) }}
          <template v-if="lamp.info">
            ·
            {{
              t('diagnostics.resetCause', {
                cause: t(`enums.resetCause.${lamp.info.reset_cause}`),
              })
            }}
          </template>
        </p>
      </div>

      <UButton
        v-if="connected"
        color="neutral"
        variant="outline"
        size="sm"
        icon="i-lucide-rotate-ccw"
        class="shrink-0"
        :disabled="lamp.locked"
        @click="confirmingRestart = true"
      >
        {{ t('diagnostics.restart') }}
      </UButton>
    </div>

    <UAlert
      v-if="crash"
      color="warning"
      variant="subtle"
      icon="i-lucide-triangle-alert"
      :description="t('diagnostics.crashed', { fault: crash })"
      :ui="alert"
    />
    <UAlert
      v-if="errors.last"
      color="error"
      variant="subtle"
      icon="i-lucide-circle-x"
      :description="errorText(errors.last)"
      :close="{ size: 'sm' }"
      :ui="alert"
      @update:open="errors.clear()"
    />

    <UModal
      v-model:open="confirmingRestart"
      :title="t('diagnostics.restartTitle')"
      :description="t('diagnostics.restartDescription')"
    >
      <template #footer>
        <div class="flex w-full justify-end gap-2">
          <UButton color="neutral" variant="outline" @click="confirmingRestart = false">
            {{ t('common.cancel') }}
          </UButton>
          <UButton icon="i-lucide-rotate-ccw" @click="restart">
            {{ t('diagnostics.restart') }}
          </UButton>
        </div>
      </template>
    </UModal>
  </header>
</template>
