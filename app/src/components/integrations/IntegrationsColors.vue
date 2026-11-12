<!--
  SPDX-License-Identifier: GPL-3.0-or-later
  SPDX-FileCopyrightText: 2026 The Disled contributors

  The color of each state of a plugin, when the lamp follows it: the four
  built-in colors, a separator, then the colors of the user. A state whose
  color the user deleted takes its default color again.
-->
<script setup lang="ts">
import type { SelectItem } from '@nuxt/ui';
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import type { PluginView } from '@/bindings/PluginView';
import { useColorsStore } from '@/stores/colors';
import { usePluginsStore } from '@/stores/plugins';

const props = defineProps<{ plugin: PluginView }>();

const { t } = useI18n();
const colors = useColorsStore();
const plugins = usePluginsStore();

const items = computed<SelectItem[][]>(() => {
  const shown = (list: typeof colors.builtIn) =>
    list.map((color) => ({ label: color.name, value: color.slot, hex: color.hex }));
  const groups = [shown(colors.builtIn)];
  if (colors.custom.length) {
    groups.push(shown(colors.custom));
  }
  return groups;
});

function hexOf(slot: number): string | undefined {
  return colors.color(slot)?.hex;
}

function choose(state: string, slot: unknown): void {
  if (typeof slot === 'number') {
    void plugins.setColor(props.plugin.id, state, slot);
  }
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
          :aria-label="t('lamp.colors.title')"
          size="sm"
          @update:model-value="(slot: unknown) => choose(state.id, slot)"
        >
          <template #leading>
            <span
              class="block size-3.5 rounded-[20%] border border-accented"
              :style="{ background: hexOf(state.slot) }"
            />
          </template>
          <template #item-leading="{ item }">
            <span
              class="block size-3.5 rounded-[20%] border border-accented"
              :style="{ background: (item as { hex?: string }).hex }"
            />
          </template>
        </USelect>
      </template>
    </div>
  </section>
</template>
