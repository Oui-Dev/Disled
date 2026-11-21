<!--
  SPDX-License-Identifier: GPL-3.0-or-later
  SPDX-FileCopyrightText: 2026 The Disled contributors

  The color of each state of a plugin, when the lamp follows it: the four
  built-in colors, a separator, then the colors of the user. A state whose
  color the user deleted takes its default color again.
-->
<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import type { PluginView } from '@/bindings/PluginView';
import { type ShownColor, useColorsStore } from '@/stores/colors';
import { usePluginsStore } from '@/stores/plugins';

const props = defineProps<{ plugin: PluginView }>();

const { t } = useI18n();
const colors = useColorsStore();
const plugins = usePluginsStore();

const items = computed(() => {
  const shown = (list: ShownColor[]) =>
    list.map((color) => ({ label: color.name, value: color.slot, hex: color.hex }));
  return colors.custom.length
    ? [shown(colors.builtIn), shown(colors.custom)]
    : [shown(colors.builtIn)];
});

function choose(state: string, slot: number): void {
  void plugins.setColor(props.plugin.id, state, slot);
}
</script>

<template>
  <section class="flex flex-col gap-3">
    <h3 class="text-sm font-semibold text-highlighted">{{ t('integrations.colors') }}</h3>
    <p class="text-sm text-muted">
      {{ t('integrations.colorsHint', { name: plugin.name }) }}
    </p>
    <div class="grid grid-cols-[minmax(0,1fr)_14rem] items-center gap-x-4 gap-y-2">
      <template v-for="state in plugin.states" :key="state.id">
        <span class="truncate text-sm text-default">{{ state.label }}</span>
        <USelect
          :model-value="state.slot"
          :items="items"
          :aria-label="state.label"
          size="sm"
          @update:model-value="(slot) => choose(state.id, slot)"
        >
          <template #leading="{ item }">
            <span class="swatch block size-3.5" :style="{ background: item?.hex }" />
          </template>
          <template #item-leading="{ item }">
            <span class="swatch block size-3.5" :style="{ background: item.hex }" />
          </template>
        </USelect>
      </template>
    </div>
  </section>
</template>
