<!--
  SPDX-License-Identifier: GPL-3.0-or-later
  SPDX-FileCopyrightText: 2026 The Disled contributors

  The line of the calls, under the tiles, in manual mode while "Microphone
  in use" is on, where this system can tell a call. At rest it names the
  color of the calls, which its popover changes: the four built-in colors,
  then the custom ones under a line. During a call whose color the app
  shows, it says so, tinted with that color, and names the color that comes
  back after. A choice of the user during the call takes the hand, and the
  line its rest form.
-->
<script setup lang="ts">
import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { useColorsStore } from '@/stores/colors';
import { useControlStore } from '@/stores/control';
import { useLampStore } from '@/stores/lamp';
import { useSettingsStore } from '@/stores/settings';

const { t } = useI18n();
const lamp = useLampStore();
const control = useControlStore();
const colors = useColorsStore();
const settingsStore = useSettingsStore();

const open = ref(false);

const calls = computed(() => settingsStore.settings?.calls);
const shown = computed(
  () => calls.value?.mic === true && control.status.calls.available && !control.status.mode,
);
const callColor = computed(() => (calls.value ? colors.color(calls.value.color) : undefined));

/** While the call color shows: the color that comes back after the call. */
const back = computed(() => {
  const status = control.status.calls;
  return status.imposed && status.back !== null ? colors.color(status.back) : undefined;
});

const items = computed(() => {
  const firstCustom = colors.custom[0]?.slot;
  return [...colors.builtIn, ...colors.custom].map((color) => ({
    value: color.slot,
    label: color.name,
    // The custom colors start a row of their own, under a line across the popover.
    class:
      color.slot === firstCustom
        ? 'col-start-1 before:absolute before:-top-[9px] before:-left-0.5 before:w-[292px] before:border-t before:border-default'
        : undefined,
  }));
});

/** Saves the color of the calls, and closes the popover, as "My colors" does. */
function choose(slot: unknown): void {
  if (typeof slot !== 'number') {
    return;
  }
  open.value = false;
  void settingsStore.update({ calls_color: slot });
}
</script>

<template>
  <template v-if="shown && calls">
    <p
      v-if="back"
      class="in-call -mx-1.5 flex h-7 items-center gap-1.5 rounded-md px-1.5 text-xs whitespace-nowrap text-muted"
      :style="{ '--call': callColor?.hex }"
      :title="t('calls.detected')"
    >
      <UIcon name="i-lucide-mic" class="size-3.5 shrink-0 text-(--call)" />
      <span class="sr-only">{{ t('calls.micPrefix') }} </span>
      <b class="font-semibold text-highlighted">{{ t('calls.inCall') }}</b>
      {{ ' ' }}
      <span class="truncate">{{ t('calls.then', { name: back.name }) }}</span>
    </p>
    <div v-else class="flex h-7 min-w-0 items-center gap-1.5 text-xs whitespace-nowrap text-muted">
      <UIcon name="i-lucide-phone" class="size-3.5 shrink-0" />
      <span>{{ t('calls.during') }}</span>
      <UPopover
        v-model:open="open"
        arrow
        :content="{ side: 'bottom', align: 'start', sideOffset: 6, collisionPadding: 16 }"
        :ui="{ content: 'w-[308px] p-2' }"
      >
        <button
          type="button"
          class="inline-flex min-w-0 cursor-pointer items-center gap-0.5 rounded font-medium text-highlighted focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary disabled:cursor-not-allowed disabled:opacity-75"
          :disabled="lamp.locked"
          :aria-label="t('calls.colorLabel', { name: callColor?.name ?? '' })"
          aria-haspopup="dialog"
          :aria-expanded="open"
        >
          <span class="truncate">{{ callColor?.name }}</span>
          <UIcon name="i-lucide-chevron-down" class="size-3.5 shrink-0 text-muted" />
        </button>

        <template #content>
          <div role="dialog" :aria-label="t('calls.popoverLabel')">
            <div class="flex h-6 items-center justify-between gap-2 px-1 text-xs font-medium">
              <span class="text-highlighted">{{ t('calls.popoverTitle') }}</span>
              <span class="truncate text-muted">{{ callColor?.name }}</span>
            </div>
            <URadioGroup
              :model-value="calls.color"
              :items="items"
              :legend="t('calls.popoverLabel')"
              orientation="horizontal"
              indicator="hidden"
              :ui="{
                legend: 'sr-only',
                fieldset:
                  'grid grid-cols-[repeat(7,2rem)] gap-x-1.5 gap-y-[17px] px-0.5 pt-1.5 pb-0.5',
                item: 'group relative rounded-lg p-0.5',
                wrapper: 'block',
                label: 'block cursor-pointer',
              }"
              @update:model-value="choose($event)"
            >
              <template #label="{ item }">
                <span class="sr-only">{{ item.label }}</span>
                <span
                  aria-hidden="true"
                  :title="item.label"
                  class="block size-7 rounded-[20%] border border-accented ring-2 ring-transparent ring-offset-2 ring-offset-bg transition-shadow group-has-data-[state=checked]:ring-inverted"
                  :style="{ background: colors.color(Number(item.value))?.hex }"
                />
              </template>
            </URadioGroup>
          </div>
        </template>
      </UPopover>
    </div>
  </template>
</template>

<style scoped>
/* The line of a call: tinted with the color of the calls. */
.in-call {
  background: color-mix(in srgb, var(--call) 10%, var(--ui-bg));
  box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--call) 30%, transparent);
}
</style>
