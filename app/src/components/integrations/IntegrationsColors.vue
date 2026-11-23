<!--
  SPDX-License-Identifier: GPL-3.0-or-later
  SPDX-FileCopyrightText: 2026 The Disled contributors

  The color of each state of a plugin, when the lamp follows it: the color
  of the plugin itself, when it gives the state one, then the four built-in
  colors, a separator, and the colors of the user. A state whose color the
  user deleted takes the color of the plugin again.
-->
<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import type { PluginView } from '@/bindings/PluginView';
import type { StateView } from '@/bindings/StateView';
import { type ShownColor, useColorsStore } from '@/stores/colors';
import { usePluginsStore } from '@/stores/plugins';
import { cssOf } from '@/utils/color';

const props = defineProps<{ plugin: PluginView }>();

const { t } = useI18n();
const colors = useColorsStore();
const plugins = usePluginsStore();

/** The value of the color of the plugin itself in a menu, beside the slots. */
const OWN = 'own';

interface Item {
  label: string;
  value: number | typeof OWN;
  swatch: string;
}

const palette = computed(() => {
  const shown = (list: ShownColor[]): Item[] =>
    list.map((color) => ({ label: color.name, value: color.slot, swatch: color.swatch }));
  return colors.custom.length
    ? [shown(colors.builtIn), shown(colors.custom)]
    : [shown(colors.builtIn)];
});

/** The menu of a state: the color of the plugin first, when it is none of the palette. */
function items(state: StateView): Item[][] {
  const own = state.default;
  if (!('xy' in own)) {
    return palette.value;
  }
  return [
    [{ label: t('integrations.ownColor'), value: OWN, swatch: cssOf(own.xy) }],
    ...palette.value,
  ];
}

function shown(state: StateView): number | typeof OWN {
  return 'xy' in state.color ? OWN : state.color.slot;
}

/** A slot of the palette, or the color of the plugin again. */
function choose(state: string, value: number | typeof OWN): void {
  void plugins.setColor(props.plugin.id, state, value === OWN ? null : value);
}
</script>

<template>
  <section class="flex flex-col gap-3">
    <h3 class="text-highlighted text-sm font-semibold">{{ t('integrations.colors') }}</h3>
    <p class="text-muted text-sm">
      {{ t('integrations.colorsHint', { name: plugin.name }) }}
    </p>
    <div class="grid grid-cols-[minmax(0,1fr)_14rem] items-center gap-x-4 gap-y-2">
      <template v-for="state in plugin.states" :key="state.id">
        <span class="text-default truncate text-sm">{{ state.label }}</span>
        <USelect
          :model-value="shown(state)"
          :items="items(state)"
          :aria-label="state.label"
          size="sm"
          @update:model-value="(value) => choose(state.id, value)"
        >
          <template #leading="{ item }">
            <span class="swatch block size-3.5" :style="{ background: item?.swatch }" />
          </template>
          <template #item-leading="{ item }">
            <span class="swatch block size-3.5" :style="{ background: item.swatch }" />
          </template>
        </USelect>
      </template>
    </div>
  </section>
</template>
