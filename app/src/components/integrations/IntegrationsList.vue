<!--
  SPDX-License-Identifier: GPL-3.0-or-later
  SPDX-FileCopyrightText: 2026 The Disled contributors

  The plugins of the folder, one a row: its icon, its name, a line on how
  it stands, and its switch. An unreadable plugin shows in red under the
  name of its folder, its switch greyed.
-->
<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import type { PluginView } from '@/bindings/PluginView';
import { usePluginTexts } from '@/composables/usePluginTexts';
import { useControlStore } from '@/stores/control';
import { usePluginsStore } from '@/stores/plugins';
import { pluginIcon } from '@/utils/pluginIcons';

const selected = defineModel<string | null>({ required: true });
const emit = defineEmits<{ toggle: [plugin: PluginView, on: boolean] }>();

const { t } = useI18n();
const plugins = usePluginsStore();
const control = useControlStore();
const { subtitle } = usePluginTexts();
</script>

<template>
  <nav :aria-label="t('integrations.list')" class="flex flex-col overflow-y-auto p-2">
    <p v-if="!plugins.plugins.length" class="p-3 text-sm text-muted">
      {{ t('integrations.empty') }}
    </p>
    <ul class="flex flex-col gap-0.5">
      <li
        v-for="plugin in plugins.plugins"
        :key="plugin.id"
        class="flex items-center gap-2 rounded-lg pe-2"
        :class="selected === plugin.id ? 'bg-elevated' : 'hover:bg-elevated/50'"
      >
        <button
          type="button"
          class="flex min-w-0 flex-1 items-center gap-3 rounded-lg px-2 py-2 text-start focus-visible:outline-2 focus-visible:outline-primary"
          :aria-current="selected === plugin.id ? 'true' : undefined"
          @click="selected = plugin.id"
        >
          <span
            class="grid size-8 shrink-0 place-items-center rounded-md bg-muted"
            :class="plugin.unreadable ? 'text-error' : 'text-default'"
          >
            <UIcon
              :name="plugin.unreadable ? 'i-lucide-triangle-alert' : pluginIcon(plugin.icon)"
              class="size-4"
            />
          </span>
          <span class="flex min-w-0 flex-col">
            <span class="truncate text-sm font-medium text-highlighted">{{ plugin.name }}</span>
            <span class="truncate text-xs" :class="plugin.unreadable ? 'text-error' : 'text-muted'">
              {{ subtitle(plugin, control.status.mode === plugin.id) }}
            </span>
          </span>
        </button>
        <USwitch
          size="sm"
          :model-value="plugin.enabled"
          :disabled="!!plugin.unreadable"
          :aria-label="t('integrations.enable', { name: plugin.name })"
          @update:model-value="(on: boolean) => emit('toggle', plugin, on)"
        />
      </li>
    </ul>
  </nav>
</template>
