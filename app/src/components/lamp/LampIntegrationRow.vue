<!--
  SPDX-License-Identifier: GPL-3.0-or-later
  SPDX-FileCopyrightText: 2026 The Disled contributors

  The color of the lamp while an integration picks it, in place of the
  tiles: the "Off" tile, which stays the switch in every mode, then the card
  of the integration, its color and its state. A click on the card turns a
  lamp that is off back on, with the color of the state; one on a lit lamp
  does nothing. Without a lamp, the card keeps the state, greyed like the
  tiles.
-->
<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { useColorsStore } from '@/stores/colors';
import { useControlStore } from '@/stores/control';
import { useLampStore } from '@/stores/lamp';
import { pluginIcon } from '@/utils/pluginIcons';

const { t, d } = useI18n();
const lamp = useLampStore();
const control = useControlStore();
const colors = useColorsStore();

const name = computed(() => control.integration?.name ?? '');
const icon = computed(() => pluginIcon(control.integration?.icon));
const state = computed(() => control.status.state);
/** Off as the lamp last said; without a lamp, the card shows the state. */
const off = computed(() => lamp.state !== null && !lamp.state.on);
const swatch = computed(() => {
  const slot = state.value?.slot;
  return slot === undefined ? undefined : colors.color(slot)?.hex;
});

const lines = computed(() => {
  const label = state.value?.label;
  if (off.value) {
    return [t('modes.clickToTurnOn'), label ?? t('integrations.noEvent')];
  }
  if (!state.value) {
    return [name.value, t('integrations.noEvent')];
  }
  const since = d(new Date(state.value.since * 1000), 'time');
  return [state.value.label, t('modes.since', { time: since })];
});

function turnOff(): void {
  if (!off.value) {
    void lamp.choose({ on: false });
  }
}

/** The backend gives the lamp the color of the state. */
function turnOn(): void {
  if (off.value) {
    void lamp.choose({ on: true });
  }
}
</script>

<template>
  <div class="flex flex-col gap-2">
    <span aria-hidden="true" class="text-sm font-semibold text-highlighted">
      {{ t('lamp.colors.title') }}
    </span>
    <div class="-mx-2 flex items-center">
      <button
        type="button"
        :disabled="!lamp.ready"
        :aria-pressed="off"
        class="group flex w-1/6 shrink-0 flex-col items-center gap-1.5 rounded-lg py-1 text-muted disabled:cursor-not-allowed disabled:opacity-75"
        @click="turnOff"
      >
        <span
          aria-hidden="true"
          :data-state="off ? 'checked' : 'unchecked'"
          class="swatch-off block size-9 rounded-[20%] border border-accented ring-2 ring-transparent ring-offset-2 ring-offset-bg transition-shadow group-has-data-[state=checked]:ring-inverted"
        />
        <LampTileName :name="t('lamp.colors.off')" />
      </button>
      <button
        type="button"
        :disabled="!lamp.ready"
        :aria-pressed="!off"
        :aria-label="off ? t('modes.turnOn', { name }) : t('modes.on', { name })"
        class="flex min-w-0 flex-1 items-center gap-3 rounded-lg px-1.5 py-1 text-start disabled:cursor-not-allowed disabled:opacity-75"
        @click="turnOn"
      >
        <span
          aria-hidden="true"
          class="relative block size-9 shrink-0 rounded-[20%] border border-accented ring-2 ring-offset-2 ring-offset-bg"
          :class="off ? 'ring-transparent' : 'ring-inverted'"
        >
          <span
            class="absolute inset-0 rounded-[20%]"
            :class="{ 'opacity-40': off }"
            :style="swatch ? { background: swatch } : undefined"
          />
          <span
            class="absolute -end-1.25 -bottom-1.25 grid size-3.75 place-items-center rounded-full bg-default ring-1 ring-accented"
          >
            <UIcon :name="icon" class="size-2.5 text-primary" />
          </span>
        </span>
        <span class="flex min-w-0 flex-col text-xs">
          <b class="truncate font-medium text-highlighted">{{ lines[0] }}</b>
          <span class="truncate text-muted">{{ lines[1] }}</span>
        </span>
      </button>
    </div>
  </div>
</template>
