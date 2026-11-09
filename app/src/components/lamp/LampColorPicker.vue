<!--
  SPDX-License-Identifier: GPL-3.0-or-later
  SPDX-FileCopyrightText: 2026 The Disled contributors

  Off, then the four lamp colors, as a radio group whose choice the arrow
  keys move. The lamp owns its state, so the selection is its last STATE
  message.
-->
<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import type { LampColor } from '@/bindings/LampColor';
import { useLampStore } from '@/stores/lamp';
import { LAMP_COLORS, SWATCH_COLORS } from '@/utils/lamp';

type Choice = LampColor | 'off';

const { t } = useI18n();
const lamp = useLampStore();

// A radio group emits the type of its items' values.
const items = computed<{ value: Choice; label: string }[]>(() => [
  { value: 'off', label: t('lamp.colors.off') },
  ...LAMP_COLORS.map((color) => ({ value: color, label: t(`lamp.colors.${color}`) })),
]);

const selected = computed<Choice | undefined>(() => {
  const state = lamp.state;
  if (!state) {
    return undefined;
  }
  return state.on ? state.color : 'off';
});

function select(choice: Choice): void {
  void lamp.setLamp(choice === 'off' ? { on: false } : { on: true, color: choice });
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
    :disabled="!lamp.state"
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
        :style="item.value === 'off' ? undefined : { background: SWATCH_COLORS[item.value] }"
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
