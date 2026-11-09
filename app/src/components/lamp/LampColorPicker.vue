<!--
  SPDX-License-Identifier: GPL-3.0-or-later
  SPDX-FileCopyrightText: 2026 The Disled contributors

  The color of the lamp: off and the four built-in colors under the names
  the user gave them, as a radio group whose choice the arrow keys move,
  then the sixth tile of "My colors". Six cells of 54 px over the content,
  the row 8 px wider on each side. The lamp owns its state, so the selection
  is its last STATE message: a custom color checks no radio, and the sixth
  tile takes it. In the Diagnostics window, `standalone` leaves out the links
  to the pages of the popup.
-->
<script setup lang="ts">
import { computed, useTemplateRef } from 'vue';
import { useI18n } from 'vue-i18n';
import { useTileNames } from '@/composables/useTileNames';
import { useColorsStore } from '@/stores/colors';
import { useLampStore } from '@/stores/lamp';
import { BUILT_IN_COLORS } from '@/utils/lamp';

withDefaults(defineProps<{ standalone?: boolean }>(), { standalone: false });
const emit = defineEmits<{ manage: []; create: [] }>();

/** Off, a built-in color by slot, or another color, which checks no item. */
type Choice = number | 'off' | 'other';

const { t } = useI18n();
const lamp = useLampStore();
const colors = useColorsStore();

// A radio group emits the type of its items' values.
const items = computed<{ value: Choice; label: string }[]>(() => [
  { value: 'off', label: t('lamp.colors.off') },
  ...colors.builtIn.map((color) => ({ value: color.slot, label: color.name })),
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

/** The name of the sixth tile: the custom color the lamp shows, or "More". */
const moreName = computed(() => {
  const state = lamp.state;
  const shown =
    state?.on && state.color >= BUILT_IN_COLORS.length
      ? colors.custom.find((color) => color.slot === state.color)
      : undefined;
  return shown?.name ?? t('lamp.colors.more');
});

const row = useTemplateRef<HTMLElement>('row');
const narrow = useTileNames(
  row,
  computed(() => [...items.value.map((item) => item.label), moreName.value]),
);

function narrowAt(choice: Choice): boolean {
  return narrow.value[items.value.findIndex((item) => item.value === choice)] ?? false;
}

/** The swatch of a built-in color; off draws its own. */
function swatch(choice: Choice): { background: string } | undefined {
  const color = typeof choice === 'number' ? BUILT_IN_COLORS[choice] : undefined;
  return color ? { background: color.swatch } : undefined;
}

function select(choice: Choice): void {
  if (choice === 'off') {
    void lamp.choose({ on: false });
  } else if (typeof choice === 'number') {
    void lamp.choose({ on: true, slot: choice });
  }
}
</script>

<template>
  <div class="flex flex-col gap-2">
    <span aria-hidden="true" class="text-sm font-semibold text-highlighted">
      {{ t('lamp.colors.title') }}
    </span>
    <div ref="row" class="-mx-2 flex">
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
        class="min-w-0 flex-5"
        :ui="{
          legend: 'sr-only',
          fieldset: 'min-w-0 flex-nowrap gap-x-0',
          item: 'group min-w-0 flex-1 rounded-lg px-0 py-1',
          wrapper: 'w-full',
          label: 'flex w-full cursor-pointer flex-col items-center gap-1.5 font-normal text-muted',
        }"
        @update:model-value="select($event)"
      >
        <template #label="{ item }">
          <span
            aria-hidden="true"
            class="block size-9 rounded-[20%] border border-accented ring-2 ring-transparent ring-offset-2 ring-offset-bg transition-shadow group-has-data-[state=checked]:ring-inverted"
            :class="{ 'swatch-off': item.value === 'off' }"
            :style="swatch(item.value)"
          />
          <LampTileName :name="item.label" :narrow="narrowAt(item.value)" />
        </template>
      </URadioGroup>
      <LampMyColors
        :standalone="standalone"
        :narrow="narrow[items.length] ?? false"
        @manage="emit('manage')"
        @create="emit('create')"
      />
    </div>
  </div>
</template>
