<!--
  SPDX-License-Identifier: GPL-3.0-or-later
  SPDX-FileCopyrightText: 2026 The Disled contributors

  Health tab of the Diagnostics window: the lamp diagnostics (GET_DIAGNOSTICS)
  by theme, one value per row, and a warning on the rows that need a look.
  The diagnostics store keeps them current.
-->
<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { useFaultText } from '@/composables/useFaultText';
import { useDiagnosticsStore } from '@/stores/diagnostics';
import { useLampStore } from '@/stores/lamp';
import { useSignedPercent } from '@/utils/format';
import { healthIssues } from '@/utils/health';

interface Row {
  label: string;
  value: string;
  detail?: string;
  issue?: boolean;
}

const { t, n } = useI18n();
const lamp = useLampStore();
const diagnostics = useDiagnosticsStore();
const faultText = useFaultText();
const signedPercent = useSignedPercent();

const groups = computed<{ title: string; rows: Row[] }[]>(() => {
  const d = diagnostics.diagnostics;
  if (!d) {
    return [];
  }
  const issues = healthIssues(d);
  return [
    {
      title: t('health.stability'),
      rows: [
        { label: t('health.boots'), value: n(d.boots) },
        {
          label: t('health.watchdogResets'),
          value: n(d.watchdog_resets),
          issue: issues.watchdogResets,
        },
        {
          label: t('health.faults'),
          value: n(d.faults),
          issue: issues.faults,
          detail: d.last_fault
            ? t('health.lastFault', { fault: faultText(d.last_fault) })
            : undefined,
        },
        { label: t('health.stackMargin'), value: t('health.bytes', d.stack_free_min) },
      ],
    },
    {
      title: t('health.sensors'),
      rows: [
        {
          label: t('health.lightSensor'),
          value: d.light_sensor_present ? t('health.present') : t('health.absent'),
          issue: issues.lightSensor,
          detail: `${t('health.probes', d.light_sensor_probes)} · ${t('health.i2cErrors', d.i2c_errors)}`,
        },
        {
          label: t('health.touchSamplesLost'),
          value: n(d.touch_samples_lost),
          issue: issues.touchSamplesLost,
          detail: t('health.rebaselines', d.touch_recalibrations),
        },
      ],
    },
    {
      title: t('health.usbClock'),
      rows: [
        { label: t('health.usbSuspends'), value: n(d.usb_suspends) },
        {
          label: t('health.crystal'),
          value: d.crystal_running ? t('health.crystalRunning') : t('health.crystalFailed'),
          issue: issues.crystal,
        },
      ],
    },
    {
      title: t('health.brightness'),
      rows: [
        { label: t('health.correction'), value: signedPercent(d.auto_bias) },
        { label: t('health.touchCorrection'), value: signedPercent(d.touch_sensitivity) },
        { label: t('health.uptime'), value: n(d.uptime_ms / 60_000, 'minutes') },
      ],
    },
  ];
});
</script>

<template>
  <!-- The tab names the panel; this h2 names it for screen readers too, above the h3 of the groups. -->
  <h2 class="sr-only">{{ t('health.title') }}</h2>
  <p v-if="!lamp.connection.connected" class="text-sm text-muted">
    {{ t('diagnostics.noLamp') }}
  </p>
  <p v-else-if="!groups.length" class="text-sm text-muted">{{ t('health.reading') }}</p>
  <div v-else class="flex flex-col gap-5">
    <section v-for="group in groups" :key="group.title" class="flex flex-col gap-1">
      <h3 class="text-xs font-semibold text-muted">{{ group.title }}</h3>
      <dl class="flex flex-col divide-y divide-default text-sm">
        <div
          v-for="row in group.rows"
          :key="row.label"
          class="grid grid-cols-[1fr_auto] items-center gap-x-3 gap-y-0.5 py-2"
        >
          <dt>{{ row.label }}</dt>
          <dd class="flex items-center gap-1.5 font-medium text-highlighted tabular-nums">
            {{ row.value }}
            <template v-if="row.issue">
              <UIcon name="i-lucide-triangle-alert" class="size-4 shrink-0 text-warning" />
              <span class="sr-only">{{ t('health.issue') }}</span>
            </template>
          </dd>
          <dd v-if="row.detail" class="col-span-2 text-xs text-muted">{{ row.detail }}</dd>
        </div>
      </dl>
    </section>
  </div>
</template>
