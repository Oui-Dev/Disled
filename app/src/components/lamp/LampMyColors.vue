<!--
  SPDX-License-Identifier: GPL-3.0-or-later
  SPDX-FileCopyrightText: 2026 The Disled contributors

  The sixth tile of the color row and its "My colors" popover. The tile is a
  color wheel named "More", or takes the custom color that the lamp shows.
  The popover lists the custom colors in the order they were made, then
  "New" while there are fewer than five, with "Manage" toward the library.
  In the Diagnostics window, `standalone` leaves out the links to the pages
  of the popup.
-->
<script setup lang="ts">
import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { useColorsStore } from '@/stores/colors';
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

const open = ref(false);

/** The custom color the lamp shows, which the tile takes. */
const current = computed(() => {
  const state = lamp.state;
  return state?.on && state.color >= BUILT_IN_COLORS.length
    ? colors.custom.find((color) => color.slot === state.color)
    : undefined;
});

const tileLabel = computed(() =>
  current.value
    ? t('lamp.colors.moreCurrent', { name: current.value.name })
    : t('lamp.colors.moreLabel'),
);

const chosen = computed(() => current.value?.slot);

function choose(slot: number | string): void {
  if (typeof slot !== 'number') {
    return;
  }
  open.value = false;
  void lamp.choose({ on: true, slot });
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
    :ui="{ content: 'w-[308px] p-3' }"
  >
    <button
      type="button"
      class="group flex min-w-0 flex-1 flex-col items-center gap-1.5 rounded-lg py-1 disabled:cursor-not-allowed disabled:opacity-75"
      :disabled="lamp.locked"
      :aria-label="tileLabel"
      aria-haspopup="dialog"
      :aria-expanded="open"
    >
      <!-- As in a checked radio of the row: the ring and the name follow it. -->
      <span v-if="current" hidden data-state="checked" />
      <span
        aria-hidden="true"
        class="relative block size-9 rounded-[20%] border border-accented ring-2 ring-transparent ring-offset-2 ring-offset-bg transition-shadow group-has-data-[state=checked]:ring-inverted"
        :class="{ wheel: !current }"
        :style="current ? { background: current.hex } : undefined"
      >
        <span
          class="absolute -right-1.5 -bottom-1.5 flex size-4 items-center justify-center rounded-full border border-default bg-default"
        >
          <UIcon name="i-lucide-chevron-down" class="size-3 text-muted" />
        </span>
      </span>
      <LampTileName :name="current?.name ?? t('lamp.colors.more')" :narrow="narrow" />
    </button>

    <template #content>
      <div role="dialog" :aria-label="t('colors.mine.title')" class="flex flex-col gap-3">
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
          <URadioGroup
            v-if="colors.custom.length"
            :key="chosen ?? 'none'"
            :model-value="chosen"
            :items="colors.custom.map((color) => ({ value: color.slot, label: color.name }))"
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
              <LampTileName :name="item.label ?? ''" />
            </template>
          </URadioGroup>
          <button
            v-if="!standalone && !colors.full"
            type="button"
            class="group flex min-w-0 flex-col items-center gap-1.5 rounded-lg py-1"
            @click="go('create')"
          >
            <span
              aria-hidden="true"
              class="flex size-9 items-center justify-center rounded-[20%] border border-dashed border-accented"
            >
              <UIcon name="i-lucide-plus" class="size-4 text-muted" />
            </span>
            <LampTileName :name="t('colors.mine.new')" />
          </button>
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
