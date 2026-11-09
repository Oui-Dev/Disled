<!--
  SPDX-License-Identifier: GPL-3.0-or-later
  SPDX-FileCopyrightText: 2026 The Disled contributors

  Off, then the four built-in colors, as a radio group whose choice the
  arrow keys move. The lamp owns its state, so the selection is its last
  STATE message. A color is a palette slot.
-->
<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { useLampStore } from '@/stores/lamp';
import { BUILT_IN_COLORS } from '@/utils/lamp';

/** Off, a built-in color by slot, or another color, which checks no item. */
type Choice = number | 'off' | 'other';

const { t } = useI18n();
const lamp = useLampStore();

// A radio group emits the type of its items' values.
const items = computed<{ value: Choice; label: string }[]>(() => [
  { value: 'off', label: t('lamp.colors.off') },
  ...BUILT_IN_COLORS.map((color, slot) => ({ value: slot, label: t(`lamp.colors.${color.key}`) })),
]);

const selected = computed<Choice | undefined>(() => {
  const state = lamp.state;
  if (!state) {
    return undefined;
  }
  if (!state.on) {
    return 'off';
  }
  return state.color < BUILT_IN_COLORS.length ? state.color : 'other';
});

/** The swatch of a built-in color; off draws its own. */
function swatch(choice: Choice): { background: string } | undefined {
  const color = typeof choice === 'number' ? BUILT_IN_COLORS[choice] : undefined;
  return color ? { background: color.swatch } : undefined;
}

function select(choice: Choice): void {
  if (choice === 'off') {
    void lamp.setLamp({ on: false });
  } else if (typeof choice === 'number') {
    void lamp.setLamp({ on: true, color: choice });
  }
}
</script>

<template>
  <!-- A radio group mounted without a value keeps its own selection for
       good, through Reka's passive v-model. It mounts again once the state
       is known, so a command the lamp did not apply never shows as chosen. -->
  <URadioGroup
    :key="selected === undefined ? 'unknown' : 'known'"
    :model-value="selected"
    :items="items"
    :legend="t('lamp.colors.legend')"
    orientation="horizontal"
    indicator="hidden"
    :disabled="!lamp.ready"
    :ui="{
      legend: 'sr-only',
      fieldset: 'flex-nowrap gap-x-1',
      item: 'group min-w-0 flex-1 rounded-lg p-1',
      wrapper: 'w-full',
      label: 'flex cursor-pointer flex-col items-center gap-1.5 text-xs font-normal text-muted',
    }"
    @update:model-value="select($event)"
  >
    <template #label="{ item }">
      <span
        aria-hidden="true"
        class="block size-9 rounded-[20%] border border-accented ring-2 ring-transparent ring-offset-2 ring-offset-bg transition-shadow group-has-data-[state=checked]:ring-inverted"
        :class="{ off: item.value === 'off' }"
        :style="swatch(item.value)"
      />
      <span class="max-w-full text-center leading-tight">
        {{ item.label }}
      </span>
    </template>
  </URadioGroup>
</template>

<style scoped>
/* Off is an unlit grey square, struck through, light grey in light mode
   and dark grey in dark mode. */
.off {
  --off-fill: var(--ui-color-neutral-200);
  --off-line: var(--ui-color-neutral-400);
  background:
    linear-gradient(
      to top right,
      transparent calc(50% - 0.75px),
      var(--off-line) calc(50% - 0.75px) calc(50% + 0.75px),
      transparent calc(50% + 0.75px)
    ),
    var(--off-fill);
}

/* The whole selector is global, because Vue keeps only the class of
   `:global(.dark) .off`. */
:global(.dark .off) {
  --off-fill: var(--ui-color-neutral-800);
  --off-line: var(--ui-color-neutral-600);
}
</style>
