<!--
  SPDX-License-Identifier: GPL-3.0-or-later
  SPDX-FileCopyrightText: 2026 The Disled contributors

  Self-test tab of the Diagnostics window (SELF_TEST): the lamp shows its
  colors, checks its light sensor and its touch, and reports. The checks are
  listed before the run, then marked.
-->
<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { useDiagnosticsStore } from '@/stores/diagnostics';
import { useLampStore } from '@/stores/lamp';

type Status = 'pending' | 'passed' | 'failed';

const STATUS_ICONS: Record<Status, string> = {
  pending: 'i-lucide-circle-dashed',
  passed: 'i-lucide-circle-check',
  failed: 'i-lucide-circle-x',
};
const STATUS_COLORS: Record<Status, string> = {
  pending: 'text-dimmed',
  passed: 'text-success',
  failed: 'text-error',
};

const { t, n } = useI18n();
const lamp = useLampStore();
const diagnostics = useDiagnosticsStore();

const checks = computed(() => {
  const r = diagnostics.selfTestResult;
  const status = (ok: boolean | undefined): Status =>
    ok === undefined ? 'pending' : ok ? 'passed' : 'failed';
  return [
    {
      label: t('selfTest.lightSensor'),
      status: status(r?.light_sensor_ok),
      detail: r ? t('telemetry.lux', { value: n(r.lux_centi / 100, 'decimal') }) : '',
    },
    {
      label: t('selfTest.touchIdle'),
      status: status(r?.touch_idle_ok),
      detail: r ? t('selfTest.baseline', { value: n(r.touch_baseline) }) : '',
    },
    {
      label: t('selfTest.touchPressed'),
      status: status(r?.touch_pressed),
      detail: r
        ? t('selfTest.peak', {
            value: t('telemetry.permille', { value: n(r.touch_peak_permille) }),
          })
        : '',
    },
    { label: t('selfTest.completed'), status: status(r?.completed), detail: '' },
  ];
});
</script>

<template>
  <p v-if="!lamp.connection.connected" class="text-sm text-muted">
    {{ t('diagnostics.noLamp') }}
  </p>
  <div v-else class="flex flex-col gap-4">
    <p class="text-sm text-muted">{{ t('selfTest.instructions') }}</p>
    <ul aria-live="polite" class="flex flex-col divide-y divide-default">
      <li v-for="check in checks" :key="check.label" class="flex items-center gap-2.5 py-2 text-sm">
        <UIcon
          :name="STATUS_ICONS[check.status]"
          class="size-4 shrink-0"
          :class="STATUS_COLORS[check.status]"
        />
        <span>{{ check.label }}</span>
        <span class="sr-only">{{ t(`selfTest.${check.status}`) }}</span>
        <span class="ms-auto text-end text-xs text-muted tabular-nums">{{ check.detail }}</span>
      </li>
    </ul>
    <UButton
      color="neutral"
      variant="outline"
      size="sm"
      icon="i-lucide-play"
      class="self-start"
      :disabled="!lamp.info"
      :loading="diagnostics.selfTestRunning"
      @click="diagnostics.startSelfTest"
    >
      {{ diagnostics.selfTestRunning ? t('selfTest.running') : t('selfTest.run') }}
    </UButton>
  </div>
</template>
