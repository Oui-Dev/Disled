<!--
  SPDX-License-Identifier: GPL-3.0-or-later
  SPDX-FileCopyrightText: 2026 The Disled contributors

  What a plugin can do, from its manifest only, never from a text of the
  plugin: its program first, in the tone of a warning, then its color, its
  alerts and its events, and "Nothing else" for a plugin that runs nothing.
  The same list in the details and in the approval question.
-->
<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import type { PluginView } from '@/bindings/PluginView';

const props = defineProps<{ plugin: PluginView }>();

const { t } = useI18n();

/** The lines after that of the program, which has its own markup. */
const lines = computed(() => {
  const { capabilities, from } = props.plugin;
  const shown: { key: string; icon: string; tone: string; text: string }[] = [];
  if (capabilities.color) {
    shown.push({
      key: 'color',
      icon: 'i-lucide-palette',
      tone: 'text-muted',
      text: t('integrations.perm.color'),
    });
  }
  if (capabilities.alert) {
    shown.push({
      key: 'alert',
      icon: 'i-lucide-bell',
      tone: 'text-muted',
      text: t('integrations.perm.alert'),
    });
  }
  if (capabilities.events) {
    shown.push({
      key: 'events',
      icon: 'i-lucide-webhook',
      tone: 'text-muted',
      text: t('integrations.perm.events', { name: from ?? '' }),
    });
  }
  if (!capabilities.run) {
    shown.push({
      key: 'nothing',
      icon: 'i-lucide-shield-check',
      tone: 'text-success',
      text: t('integrations.perm.nothingElse'),
    });
  }
  return shown;
});
</script>

<template>
  <ul class="flex flex-col gap-2 text-sm text-default">
    <li v-if="plugin.capabilities.run && plugin.command" class="flex items-start gap-2">
      <UIcon name="i-lucide-triangle-alert" class="mt-0.5 size-4 shrink-0 text-warning" />
      <i18n-t keypath="integrations.perm.process" tag="span" scope="global">
        <template #command>
          <code class="rounded bg-muted px-1 py-0.5 font-mono text-xs">{{ plugin.command }}</code>
        </template>
      </i18n-t>
    </li>
    <li v-for="line in lines" :key="line.key" class="flex items-start gap-2">
      <UIcon :name="line.icon" class="mt-0.5 size-4 shrink-0" :class="line.tone" />
      <span>{{ line.text }}</span>
    </li>
  </ul>
</template>
