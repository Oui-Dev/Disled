<!--
  SPDX-License-Identifier: GPL-3.0-or-later
  SPDX-FileCopyrightText: 2026 The Disled contributors

  The preview of an import: everything the code holds, before it replaces
  the names, the colors and the cycle of this computer. The page imports
  through `apply`, which the popup footer calls.
-->
<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import type { Palette } from '@/bindings/Palette';
import { builtInColors, customColors, cycleWords, useColorsStore } from '@/stores/colors';
import { NOTE_UI } from '@/utils/ui';

const props = defineProps<{ text: string; palette: Palette }>();
const emit = defineEmits<{ done: [] }>();

const { t } = useI18n();
const colors = useColorsStore();

const groups = computed(() =>
  [
    { title: t('colors.import.originals'), colors: builtInColors(props.palette) },
    { title: t('colors.import.custom'), colors: customColors(props.palette) },
  ].filter((group) => group.colors.length),
);

const cycleText = computed(() => {
  const shown = groups.value.flatMap((group) => group.colors);
  return cycleWords(
    props.palette.cycle.map((slot) => shown.find((color) => color.slot === slot)?.name ?? ''),
  );
});

async function apply(): Promise<void> {
  await colors.importCode(props.text);
  emit('done');
}

defineExpose({ apply });
</script>

<template>
  <div class="flex flex-col gap-4">
    <h2
      data-page-heading
      tabindex="-1"
      class="text-sm font-semibold text-highlighted focus:outline-none"
    >
      {{ t('colors.import.title') }}
    </h2>

    <section v-for="group in groups" :key="group.title" class="flex flex-col gap-1.5">
      <h3 class="text-xs font-semibold text-muted">{{ group.title }}</h3>
      <ul class="flex flex-wrap gap-x-4 gap-y-1.5">
        <li
          v-for="color in group.colors"
          :key="color.slot"
          class="flex items-center gap-1.5 text-xs"
        >
          <span aria-hidden="true" class="swatch size-4" :style="{ background: color.hex }" />
          {{ color.name }}
        </li>
      </ul>
    </section>

    <section class="flex flex-col gap-1.5">
      <h3 class="text-xs font-semibold text-muted">{{ t('colors.import.cycle') }}</h3>
      <p class="text-xs text-default">{{ cycleText }}</p>
    </section>

    <UAlert
      color="neutral"
      variant="outline"
      icon="i-lucide-info"
      :description="t('colors.import.replaces')"
      :ui="NOTE_UI"
    />
  </div>
</template>
