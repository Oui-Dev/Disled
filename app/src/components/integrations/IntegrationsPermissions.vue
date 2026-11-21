<!--
  SPDX-License-Identifier: GPL-3.0-or-later
  SPDX-FileCopyrightText: 2026 The Disled contributors

  What a plugin can do, from its manifest only, never from a text of the
  plugin: its program first, in the tone of a warning, then its color, its
  alerts, what it reads of the lamp and its events, and "Nothing else" for a
  plugin that runs nothing. The same list in the details and in the approval
  question.
-->
<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import type { PluginView } from '@/bindings/PluginView';
import { CAPABILITIES } from '@/utils/capabilities';

const props = defineProps<{ plugin: PluginView }>();

const { t } = useI18n();

const shown = computed(() => CAPABILITIES.filter(({ key }) => props.plugin.capabilities[key]));
</script>

<template>
  <ul class="text-default flex flex-col gap-2 text-sm">
    <li v-if="plugin.capabilities.run && plugin.command" class="flex items-start gap-2">
      <UIcon name="i-lucide-triangle-alert" class="text-warning mt-0.5 size-4 shrink-0" />
      <i18n-t keypath="integrations.perm.process" tag="span" scope="global">
        <template #command>
          <code class="bg-muted rounded px-1 py-0.5 font-mono text-xs">{{ plugin.command }}</code>
        </template>
      </i18n-t>
    </li>
    <li v-for="{ key, icon } in shown" :key="key" class="flex items-start gap-2">
      <UIcon :name="icon" class="text-muted mt-0.5 size-4 shrink-0" />
      <span>{{ t(`integrations.perm.${key}`, { name: plugin.from ?? '' }) }}</span>
    </li>
    <li v-if="!plugin.capabilities.run" class="flex items-start gap-2">
      <UIcon name="i-lucide-shield-check" class="text-success mt-0.5 size-4 shrink-0" />
      <span>{{ t('integrations.perm.nothingElse') }}</span>
    </li>
  </ul>
</template>
