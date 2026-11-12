<!--
  SPDX-License-Identifier: GPL-3.0-or-later
  SPDX-FileCopyrightText: 2026 The Disled contributors

  The sixth tile of the color row and its "My colors" popover. The tile is a
  color wheel named "More", or takes the custom color that the lamp shows.
  The popover lists the custom colors in the order they were made, then
  "New" while there are fewer than five, with "Manage" toward the library.
  In the Diagnostics window, `standalone` leaves out the links to the pages
  of the popup. While an integration picks the color, which only that window
  shows, the tile takes no color of the lamp, and a choice takes the lamp
  back to the manual mode.
-->
<script setup lang="ts">
import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { useArrowChoice } from '@/composables/useArrowChoice';
import { useColorsStore } from '@/stores/colors';
import { useControlStore } from '@/stores/control';
import { useLampStore } from '@/stores/lamp';
import { BUILT_IN_COLORS } from '@/utils/lamp';

withDefaults(defineProps<{ standalone?: boolean; narrow?: boolean }>(), {
  standalone: false,
  narrow: false,
});
const emit = defineEmits<{ manage: []; create: [] }>();

const { t } = useI18n();
const lamp = useLampStore();
const colors = useColorsStore();
const control = useControlStore();

const open = ref(false);
const { listeners, closes } = useArrowChoice(open);

/** The custom color the lamp shows, which the tile takes, in manual mode. */
const current = computed(() => {
  const state = lamp.state;
  return control.status.mode === null && state?.on && state.color >= BUILT_IN_COLORS.length
    ? colors.custom.find((color) => color.slot === state.color)
    : undefined;
});

const tileLabel = computed(() =>
  current.value
    ? t('lamp.colors.moreCurrent', { name: current.value.name })
    : t('lamp.colors.moreLabel'),
);

const chosen = computed(() => current.value?.slot);

// Null among the values, which the group takes when no color is chosen.
const items = computed<{ value: number | null; label: string }[]>(() =>
  colors.custom.map((color) => ({ value: color.slot, label: color.name })),
);

async function choose(slot: number | null): Promise<void> {
  if (typeof slot !== 'number') {
    return;
  }
  if (closes()) {
    open.value = false;
  }
  if (control.status.mode !== null) {
    await control.setMode(null);
  }
  await lamp.choose({ on: true, slot });
}

function go(page: 'manage' | 'create'): void {
  open.value = false;
  if (page === 'manage') {
    emit('manage');
  } else {
    emit('create');
  }
}
</script>

<template>
  <UPopover
    v-model:open="open"
    arrow
    :content="{ side: 'bottom', align: 'end', sideOffset: 6, collisionPadding: 16 }"
    :ui="{ content: 'w-77 p-3' }"
  >
    <!-- A tile like those of the row: no background on hover. -->
    <UButton
      color="neutral"
      variant="ghost"
      class="group flex min-w-0 flex-1 flex-col gap-1.5 rounded-lg p-0 py-1 font-normal hover:bg-transparent active:bg-transparent"
      :disabled="lamp.locked"
      :aria-label="tileLabel"
    >
      <!-- As in a checked radio of the row: the ring and the name follow it. -->
      <span v-if="current" hidden data-state="checked" />
      <!-- The chevron in a chip on the corner of the swatch, 3/8 of the chip
           outside it. -->
      <UChip
        position="bottom-right"
        :ui="{
          base: 'size-4 translate-x-3/8 translate-y-3/8 bg-default ring-1 ring-default ring-inset',
        }"
      >
        <span
          aria-hidden="true"
          class="block size-9 rounded-[20%] border border-accented ring-2 ring-transparent ring-offset-2 ring-offset-bg transition-shadow group-has-data-[state=checked]:ring-inverted"
          :class="{ wheel: !current }"
          :style="current ? { background: current.hex } : undefined"
        />
        <template #content>
          <UIcon name="i-lucide-chevron-down" class="size-3 text-muted" />
        </template>
      </UChip>
      <LampTileName :name="current?.name ?? t('lamp.colors.more')" :narrow="narrow" />
    </UButton>

    <template #content>
      <div
        role="dialog"
        :aria-label="t('colors.mine.title')"
        class="flex flex-col gap-3"
        v-bind="listeners"
      >
        <div class="flex items-center justify-between gap-2">
          <p class="text-sm font-semibold text-highlighted">{{ t('colors.mine.title') }}</p>
          <UButton
            v-if="!standalone"
            color="primary"
            variant="link"
            size="sm"
            class="px-0"
            @click="go('manage')"
          >
            {{ t('colors.mine.manage') }}
          </UButton>
        </div>
        <div class="grid grid-cols-4 gap-x-2 gap-y-3">
          <!-- Never without a value, null when the lamp shows none of these
               colors: a radio group mounted without one keeps its own
               selection, through Reka's passive v-model. -->
          <URadioGroup
            v-if="colors.custom.length"
            :model-value="chosen ?? null"
            :items="items"
            :legend="t('colors.mine.title')"
            orientation="horizontal"
            indicator="hidden"
            :disabled="!lamp.ready"
            class="contents"
            :ui="{
              root: 'contents',
              fieldset: 'contents',
              legend: 'sr-only',
              item: 'group min-w-0 rounded-lg px-0 py-1',
              wrapper: 'w-full',
              // A class of ours would hide the cursor of a disabled group.
              label: `flex w-full flex-col items-center gap-1.5 font-normal text-muted ${
                lamp.ready ? 'cursor-pointer' : 'cursor-not-allowed'
              }`,
            }"
            @update:model-value="choose($event)"
          >
            <template #label="{ item }">
              <span
                aria-hidden="true"
                class="block size-9 rounded-[20%] border border-accented ring-2 ring-transparent ring-offset-2 ring-offset-bg transition-shadow group-has-data-[state=checked]:ring-inverted"
                :style="{ background: colors.color(Number(item.value))?.hex }"
              />
              <LampTileName :name="item.label ?? ''" :named="false" />
            </template>
          </URadioGroup>
          <UButton
            v-if="!standalone && !colors.full"
            color="neutral"
            variant="ghost"
            class="group flex min-w-0 flex-col gap-1.5 rounded-lg p-0 py-1 font-normal hover:bg-transparent active:bg-transparent"
            @click="go('create')"
          >
            <span
              aria-hidden="true"
              class="flex size-9 items-center justify-center rounded-[20%] border border-dashed border-accented"
            >
              <UIcon name="i-lucide-plus" class="size-4 text-muted" />
            </span>
            <LampTileName :name="t('colors.mine.new')" />
          </UButton>
        </div>
      </div>
    </template>
  </UPopover>
</template>

<style scoped>
/* The tile of the other colors: a color wheel. */
.wheel {
  background: conic-gradient(#ff4d4d, #ffd84d, #4dff88, #4dd8ff, #6b4dff, #ff4dd8, #ff4d4d);
}
</style>
