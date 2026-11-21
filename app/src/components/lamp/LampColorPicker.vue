<!--
  SPDX-License-Identifier: GPL-3.0-or-later
  SPDX-FileCopyrightText: 2026 The Disled contributors

  The color of the lamp: off and the four built-in colors under the names
  the user gave them, as a radio group whose choice the arrow keys move,
  then the sixth tile of "My colors". Six cells of 54 px over the content,
  the row 8 px wider on each side. The lamp owns its state, so the selection
  is its last STATE message: a custom color checks no radio, and the sixth
  tile takes it. In the Diagnostics window, `standalone` leaves out the links
  to the pages of the popup. That window also shows the tiles while an
  integration picks the color: then no tile is checked, and a choice, off
  included, takes the lamp back to the manual mode.
-->
<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { useColorsStore } from '@/stores/colors';
import { useControlStore } from '@/stores/control';
import { useLampStore } from '@/stores/lamp';
import { BUILT_IN_COLORS } from '@/utils/lamp';

defineProps<{ standalone?: boolean }>();
const emit = defineEmits<{ manage: []; create: [] }>();

/** Off, a built-in color by slot, or another color, which checks no item. */
type Choice = number | 'off' | 'other';

const { t } = useI18n();
const lamp = useLampStore();
const colors = useColorsStore();
const control = useControlStore();

/** An integration picks the color. */
const following = computed(() => control.status.mode !== null);

/** The custom color the lamp shows in manual mode, which the sixth tile takes. */
const current = computed(() => {
  const state = lamp.state;
  return !following.value && state?.on && state.color >= BUILT_IN_COLORS.length
    ? colors.custom.find((color) => color.slot === state.color)
    : undefined;
});

// A radio group emits the type of its items' values. Off draws its own swatch.
const items = computed<{ value: Choice; label: string; hex?: string }[]>(() => [
  { value: 'off', label: t('lamp.colors.off') },
  ...colors.builtIn.map((color) => ({ value: color.slot, label: color.name, hex: color.hex })),
]);
const names = computed(() => items.value.map((item) => item.label).join('\n'));

const selected = computed<Choice | undefined>(() => {
  const state = lamp.state;
  if (!state) {
    return undefined;
  }
  // The color is the integration's: every tile, the lit one included, is a
  // choice that takes the lamp back.
  if (following.value) {
    return 'other';
  }
  if (!state.on) {
    return 'off';
  }
  return state.color < BUILT_IN_COLORS.length ? state.color : 'other';
});

async function select(choice: Choice): Promise<void> {
  if (choice === 'other') {
    return;
  }
  if (following.value) {
    await control.setMode(null);
  }
  if (choice === 'off') {
    await lamp.choose({ on: false });
  } else {
    await lamp.choose({ on: true, slot: choice });
  }
}
</script>

<template>
  <div class="flex flex-col gap-2">
    <span aria-hidden="true" class="text-highlighted text-sm font-semibold">
      {{ t('lamp.colors.title') }}
    </span>
    <div class="-mx-2 flex">
      <!-- A radio group mounted without a value keeps its own selection for
           good, through Reka's passive v-model. It mounts again once the state
           is known, so a command the lamp did not apply never shows as chosen.
           It also mounts again with new names, since Reka names each radio
           once, from its label. -->
      <URadioGroup
        :key="`${selected === undefined ? 'unknown' : 'known'}:${names}`"
        :model-value="selected"
        :items="items"
        :legend="t('lamp.colors.legend')"
        orientation="horizontal"
        indicator="hidden"
        :disabled="!lamp.ready"
        class="min-w-0 flex-5"
        :ui="{
          legend: 'sr-only',
          fieldset: 'min-w-0 flex-nowrap gap-x-0',
          item: 'group min-w-0 flex-1 rounded-lg px-0 py-1',
          wrapper: 'w-full',
          label: 'flex w-full flex-col items-center gap-1.5 font-normal text-muted',
        }"
        @update:model-value="select($event)"
      >
        <template #label="{ item }">
          <span
            aria-hidden="true"
            class="swatch swatch-ring block size-9"
            :class="{ 'swatch-off': item.value === 'off' }"
            :style="{ background: item.hex }"
          />
          <LampTileName :name="item.label" :named="false" />
        </template>
      </URadioGroup>
      <LampMyColors
        :standalone="standalone"
        :current="current"
        @manage="emit('manage')"
        @create="emit('create')"
      />
    </div>
  </div>
</template>
