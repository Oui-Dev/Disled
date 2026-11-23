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
import { type ShownColor, useColorsStore } from '@/stores/colors';
import { useControlStore } from '@/stores/control';
import { useLampStore } from '@/stores/lamp';

const props = defineProps<{
  standalone?: boolean;
  /** The custom color the lamp shows in manual mode, which the tile takes. */
  current?: ShownColor;
}>();
const emit = defineEmits<{ manage: []; create: [] }>();

const { t } = useI18n();
const lamp = useLampStore();
const colors = useColorsStore();
const control = useControlStore();

const open = ref(false);
const { listeners, closes } = useArrowChoice(open);

const tileLabel = computed(() =>
  props.current
    ? t('lamp.colors.moreCurrent', { name: props.current.name })
    : t('lamp.colors.moreLabel'),
);

// Null among the values, which the group takes when no color is chosen.
const items = computed<{ value: number | null; label: string; swatch: string }[]>(() =>
  colors.custom.map((color) => ({ value: color.slot, label: color.name, swatch: color.swatch })),
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
      <!-- The chevron in a chip on the corner of the swatch, 3/8 of the chip
           outside it. -->
      <UChip
        position="bottom-right"
        :ui="{
          base: 'size-4 translate-x-3/8 translate-y-3/8 bg-default ring-1 ring-default ring-inset',
        }"
      >
        <!-- Checked as a radio of the row is: the ring and the name follow it. -->
        <span
          aria-hidden="true"
          :data-state="current ? 'checked' : undefined"
          class="swatch swatch-ring block size-9"
          :class="{ wheel: !current }"
          :style="current ? { background: current.swatch } : undefined"
        />
        <template #content>
          <UIcon name="i-lucide-chevron-down" class="text-muted size-3" />
        </template>
      </UChip>
      <LampTileName :name="current?.name ?? t('lamp.colors.more')" />
    </UButton>

    <template #content>
      <div class="flex flex-col gap-3" v-bind="listeners">
        <div class="flex items-center justify-between gap-2">
          <p class="text-highlighted text-sm font-semibold">{{ t('colors.mine.title') }}</p>
          <UButton
            v-if="!standalone"
            color="primary"
            variant="link"
            size="sm"
            class="px-0"
            @click="
              open = false;
              emit('manage');
            "
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
            :model-value="current?.slot ?? null"
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
              label: 'flex w-full flex-col items-center gap-1.5 font-normal text-muted',
            }"
            @update:model-value="choose($event)"
          >
            <template #label="{ item }">
              <span
                aria-hidden="true"
                class="swatch swatch-ring block size-9"
                :style="{ background: item.swatch }"
              />
              <LampTileName :name="item.label ?? ''" :named="false" />
            </template>
          </URadioGroup>
          <UButton
            v-if="!standalone && !colors.full"
            color="neutral"
            variant="ghost"
            class="group flex min-w-0 flex-col gap-1.5 rounded-lg p-0 py-1 font-normal hover:bg-transparent active:bg-transparent"
            @click="
              open = false;
              emit('create');
            "
          >
            <span
              aria-hidden="true"
              class="border-accented flex size-9 items-center justify-center rounded-[20%] border border-dashed"
            >
              <UIcon name="i-lucide-plus" class="text-muted size-4" />
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
