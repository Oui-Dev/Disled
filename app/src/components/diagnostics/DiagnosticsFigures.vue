<!--
  SPDX-License-Identifier: GPL-3.0-or-later
  SPDX-FileCopyrightText: 2026 The Disled contributors

  The last telemetry sample as four tiles: touch, light, LED current and
  level, each with a detail. Two columns, four when the panel is wide.
-->
<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import type { Telemetry } from '@/bindings/Telemetry';

const props = defineProps<{ sample: Telemetry }>();

const { t, n } = useI18n();

const figures = computed(() => {
  const s = props.sample;
  return [
    {
      label: t('telemetry.touch'),
      icon: 'i-lucide-pointer',
      value: t(`enums.touchState.${s.touch_state}`),
      detail: t('telemetry.drop', {
        value: t('telemetry.permille', { value: n(s.touch_delta_permille) }),
      }),
    },
    {
      label: t('telemetry.light'),
      icon: 'i-lucide-sun',
      value: t('telemetry.lux', { value: n(s.lux_centi / 100, 'decimal') }),
      detail: t('telemetry.raw', { value: n(s.als_raw) }),
    },
    {
      label: t('telemetry.ledCurrent'),
      icon: 'i-lucide-zap',
      value: t('telemetry.milliamps', { value: n(s.led_current_ma_x10 / 10, 'decimal') }),
      detail: t('telemetry.limit', { value: n(s.cap_ma) }),
    },
    {
      label: t('telemetry.level'),
      icon: 'i-lucide-lightbulb',
      value: n(s.level_permille / 1000, 'percent'),
      detail: t('telemetry.duty', { duty: s.duty.join(' / ') }),
    },
  ];
});
</script>

<template>
  <dl class="grid grid-cols-2 gap-3 @xl:grid-cols-4">
    <div
      v-for="figure in figures"
      :key="figure.label"
      class="flex flex-col gap-0.5 rounded-lg border border-default px-3.5 py-3"
    >
      <dt class="flex items-center gap-1.5 text-xs text-muted">
        <UIcon :name="figure.icon" class="size-3.5 shrink-0" />
        {{ figure.label }}
      </dt>
      <!-- Tabular figures, since the values change with every sample. -->
      <dd class="text-lg font-semibold text-highlighted tabular-nums first-letter:uppercase">
        {{ figure.value }}
      </dd>
      <dd class="text-xs text-muted tabular-nums">{{ figure.detail }}</dd>
    </div>
  </dl>
</template>
