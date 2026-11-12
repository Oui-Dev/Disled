<!--
  SPDX-License-Identifier: GPL-3.0-or-later
  SPDX-FileCopyrightText: 2026 The Disled contributors

  A live chart of the telemetry history, newest sample on the right, drawn at most
  every REFRESH_MS, with its legend under it. The plot takes the height its parent
  gives, down to a minimum, and the tooltip gives the lamp's uptime and the values.
-->
<script setup lang="ts">
import { useDocumentVisibility, useElementSize, useThrottleFn } from '@vueuse/core';
import { computed, shallowRef, useTemplateRef, watch } from 'vue';
import { useTelemetryStore } from '@/stores/telemetry';
import { type ChartRow, type ChartSeries, chartRows } from '@/utils/chart';

const props = defineProps<{
  title: string;
  description: string;
  series: ChartSeries[];
  /** The smallest plot, px. */
  minHeight: number;
  /** The values on the axis and in the tooltip, with their unit. */
  format: (value: number) => string;
}>();

const REFRESH_MS = 100;

const telemetry = useTelemetryStore();
const rows = shallowRef<ChartRow[]>([]);
const categories = computed(() =>
  props.series.map(({ key, label, color }) => ({ key, label, color })),
);

// The plot box gets its height from the layout, never from the chart in it.
const { height: boxHeight } = useElementSize(useTemplateRef('box'));
const height = computed(() => Math.max(Math.floor(boxHeight.value), props.minHeight));

const refresh = useThrottleFn(
  () => {
    rows.value = chartRows(telemetry.history, props.series);
  },
  REFRESH_MS,
  true,
);
// Nothing to draw in a hidden or minimized window; it catches up when shown.
const visibility = useDocumentVisibility();
watch(
  [() => telemetry.history, () => props.series, visibility],
  () => {
    if (visibility.value === 'visible') {
      void refresh();
    }
  },
  { immediate: true },
);
</script>

<template>
  <figure class="flex flex-col gap-1">
    <figcaption class="text-xs font-medium text-muted">{{ title }}</figcaption>
    <div ref="box" class="relative flex-1" :style="{ minHeight: `${minHeight}px` }">
      <UChart
        :data="rows"
        index="uptime"
        :categories="categories"
        :height="height"
        :x-axis="false"
        :legend="false"
        :format="format"
        :aria-label="title"
        :aria-description="description"
        class="absolute inset-0"
      />
    </div>
    <!-- Ours rather than the chart's, which sits on top, centered. -->
    <ul class="flex flex-wrap justify-end gap-x-4 gap-y-1 text-xs text-muted">
      <li v-for="line in series" :key="line.key" class="flex items-center gap-1.5">
        <span
          aria-hidden="true"
          class="h-0.5 w-3 rounded-full"
          :style="{ background: line.color }"
        />
        {{ line.label }}
      </li>
    </ul>
  </figure>
</template>
