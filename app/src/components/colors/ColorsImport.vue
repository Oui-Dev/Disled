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
import { defaultName, useColorsStore } from '@/stores/colors';
import { BUILT_IN_COLORS } from '@/utils/lamp';

const props = defineProps<{ text: string; palette: Palette }>();
const emit = defineEmits<{ done: [] }>();

const { t } = useI18n();
const colors = useColorsStore();

const originals = computed(() =>
  BUILT_IN_COLORS.map((color, slot) => ({
    slot,
    hex: color.swatch,
    name: props.palette.names[slot] ?? defaultName(slot),
  })),
);
const added = computed(() => props.palette.custom);

/** The cycle in words; a name never breaks inside. */
const cycleText = computed(() =>
  props.palette.cycle
    .map((slot) =>
      (
        originals.value[slot]?.name
        ?? added.value.find((color) => color.slot === slot)?.name
        ?? ''
      ).replaceAll(' ', '\u00a0'),
    )
    .join(' › '),
);

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

    <section class="flex flex-col gap-1.5">
      <h3 class="text-xs font-semibold text-muted">{{ t('colors.import.originals') }}</h3>
      <ul class="flex flex-wrap gap-x-4 gap-y-1.5">
        <li v-for="color in originals" :key="color.slot" class="flex items-center gap-1.5 text-xs">
          <span
            aria-hidden="true"
            class="size-4 rounded-[20%] border border-accented"
            :style="{ background: color.hex }"
          />
          {{ color.name }}
        </li>
      </ul>
    </section>

    <section v-if="added.length" class="flex flex-col gap-1.5">
      <h3 class="text-xs font-semibold text-muted">{{ t('colors.import.custom') }}</h3>
      <ul class="flex flex-wrap gap-x-4 gap-y-1.5">
        <li v-for="color in added" :key="color.slot" class="flex items-center gap-1.5 text-xs">
          <span
            aria-hidden="true"
            class="size-4 rounded-[20%] border border-accented"
            :style="{ background: color.hex }"
          />
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
      :ui="{
        root: 'gap-2 rounded-md p-3',
        icon: 'mt-0.5 size-3.5 text-muted',
        description: 'text-xs text-muted',
      }"
    />
  </div>
</template>
