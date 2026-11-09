<!--
  SPDX-License-Identifier: GPL-3.0-or-later
  SPDX-FileCopyrightText: 2026 The Disled contributors

  Live telemetry for tuning the touch, the light sensor and the heat: the
  stream period, the last figures, three charts, and CSV recordings at the
  bottom. The stream starts with the window.
-->
<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { useLampStore } from '@/stores/lamp';
import { useTelemetryStore } from '@/stores/telemetry';
import type { ChartSeries } from '@/utils/chart';

/** Stream periods offered, ms; 0 stops the stream. */
const PERIODS = [0, 50, 100, 250, 1000];

const { t, n } = useI18n();
const lamp = useLampStore();
const telemetry = useTelemetryStore();

const periods = computed(() =>
  PERIODS.map((ms) => ({
    value: ms,
    label:
      ms === 0 ? t('telemetry.off') : ms < 1000 ? n(ms, 'milliseconds') : n(ms / 1000, 'seconds'),
  })),
);

/** What stands in for the figures and the charts before the first sample. */
const placeholder = computed(() => {
  if (!lamp.connection.connected) {
    return { icon: 'i-lucide-unplug', description: t('diagnostics.noLamp') };
  }
  if (lamp.info && !lamp.info.telemetry) {
    return { icon: 'i-lucide-activity', description: t('telemetry.unsupported') };
  }
  if (telemetry.periodMs === 0) {
    return {
      icon: 'i-lucide-circle-pause',
      title: t('telemetry.stopped'),
      description: t('telemetry.stoppedHint'),
    };
  }
  return { loading: true, description: t('telemetry.waiting') };
});

// The samples are whole numbers, but the axis of a small range has ticks
// every 0.5, which one decimal at most keeps apart.
const permille = (value: number) => t('telemetry.permille', { value: n(value, 'tick') });
const lux = (value: number) => t('telemetry.lux', { value: n(value, 'decimal') });
const count = (value: number) => n(value, 'tick');

const charts = computed<
  {
    title: string;
    description: string;
    /** Its share of the height left for the charts, and its smallest plot, px. */
    grow: number;
    minHeight: number;
    series: ChartSeries[];
    format: (value: number) => string;
  }[]
>(() => [
  {
    title: t('telemetry.touchChart'),
    description: t('telemetry.touchDescription'),
    grow: 1.4,
    minHeight: 100,
    format: count,
    series: [
      {
        key: 'touch',
        label: t('telemetry.touchAverage'),
        color: 'var(--ui-chart-2)',
        value: (s) => s.touch_value,
      },
      {
        key: 'baseline',
        label: t('telemetry.baseline'),
        color: 'var(--ui-text-dimmed)',
        value: (s) => s.touch_baseline,
      },
    ],
  },
  {
    title: t('telemetry.deltaChart'),
    description: t('telemetry.deltaDescription'),
    grow: 1,
    minHeight: 70,
    format: permille,
    series: [
      {
        key: 'delta',
        label: t('telemetry.deltaSeries'),
        color: 'var(--ui-chart-4)',
        value: (s) => s.touch_delta_permille,
      },
    ],
  },
  {
    title: t('telemetry.lightChart'),
    description: t('telemetry.lightDescription'),
    grow: 1,
    minHeight: 70,
    format: lux,
    series: [
      {
        key: 'lux',
        label: t('telemetry.lightSeries'),
        color: 'var(--color-yellow-500)',
        value: (s) => s.lux_centi / 100,
      },
    ],
  },
]);
</script>

<template>
  <section class="flex flex-col">
    <div class="@container flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto p-5">
      <div class="flex flex-wrap items-center justify-between gap-3">
        <div class="flex items-center gap-2">
          <h2 class="font-semibold text-highlighted">{{ t('telemetry.title') }}</h2>
          <!-- The figures and charts of a stream turned off stay on show, frozen. -->
          <UBadge
            v-if="telemetry.latest && telemetry.periodMs === 0"
            color="neutral"
            variant="subtle"
            icon="i-lucide-circle-pause"
            :label="t('telemetry.stopped')"
          />
        </div>
        <USelect
          :model-value="telemetry.periodMs"
          :items="periods"
          :disabled="!telemetry.supported"
          :aria-label="t('telemetry.period')"
          icon="i-lucide-timer"
          size="sm"
          class="w-32"
          @update:model-value="telemetry.setPeriod($event)"
        />
      </div>

      <template v-if="telemetry.latest">
        <DiagnosticsFigures :sample="telemetry.latest" />
        <!-- The charts share the height left; below their minimum, the panel scrolls. -->
        <div class="flex flex-1 flex-col gap-4">
          <DiagnosticsChart
            v-for="chart in charts"
            :key="chart.title"
            :title="chart.title"
            :description="chart.description"
            :min-height="chart.minHeight"
            :series="chart.series"
            :format="chart.format"
            class="basis-0"
            :style="{ flexGrow: chart.grow }"
          />
        </div>
      </template>
      <UEmpty v-else v-bind="placeholder" variant="naked" class="flex-1" />
    </div>

    <!-- The recording, at hand whatever the scroll. -->
    <footer class="flex flex-wrap items-center gap-3 border-t border-default px-5 py-3">
      <UButton
        v-if="!telemetry.recording"
        color="neutral"
        variant="outline"
        size="sm"
        icon="i-lucide-circle-dot"
        :disabled="!telemetry.latest || telemetry.periodMs === 0"
        @click="telemetry.startRecording"
      >
        {{ t('telemetry.record') }}
      </UButton>
      <UButton
        v-else
        color="error"
        size="sm"
        icon="i-lucide-square"
        @click="telemetry.stopRecording"
      >
        {{ t('telemetry.stop') }}
      </UButton>
      <span class="text-xs text-muted tabular-nums">
        {{ t('telemetry.rows', telemetry.recordedRows) }}
      </span>
      <UButton
        color="neutral"
        variant="outline"
        size="sm"
        icon="i-lucide-file-down"
        class="ms-auto"
        :disabled="telemetry.recording || telemetry.recordedRows === 0"
        @click="telemetry.exportRecording(t('telemetry.saveTitle'))"
      >
        {{ t('telemetry.export') }}
      </UButton>
    </footer>
  </section>
</template>
