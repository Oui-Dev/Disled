<!--
  SPDX-License-Identifier: GPL-3.0-or-later
  SPDX-FileCopyrightText: 2026 The Disled contributors

  The details of a plugin: who made it, how it stands, its installation
  note, what it can do, the colors of its states, its token and its
  removal. A plugin never approved shows only what it will be able to do
  and its removal, until its first approval; an unreadable one, why and its
  removal.
-->
<script setup lang="ts">
import { useIntervalFn } from '@vueuse/core';
import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import type { PluginView } from '@/bindings/PluginView';
import { usePluginTexts } from '@/composables/usePluginTexts';
import { useControlStore } from '@/stores/control';
import { usePluginsStore } from '@/stores/plugins';
import { pluginIcon } from '@/utils/pluginIcons';

const props = defineProps<{ plugin: PluginView }>();
const emit = defineEmits<{ toggle: [plugin: PluginView, on: boolean] }>();

const { t } = useI18n();
const plugins = usePluginsStore();
const control = useControlStore();
const { status } = usePluginTexts();

/** The time of "3 s ago", every second. */
const now = ref(Date.now() / 1000);
useIntervalFn(() => {
  now.value = Date.now() / 1000;
}, 1000);

const driving = computed(() => control.status.mode === props.plugin.id);
const line = computed(() => status(props.plugin, now.value));
/** Never approved: the details wait for the first approval. */
const fresh = computed(() => !props.plugin.approved && !props.plugin.unreadable);
/** The dot of the status line: red for a program stopped for good, orange
 * while it waits to start again or for its port, green once it was heard,
 * grey otherwise. */
const dot = computed<'error' | 'warning' | 'success' | 'neutral'>(() => {
  const plugin = props.plugin;
  if (!plugin.enabled) {
    return 'neutral';
  }
  if (plugin.program?.kind === 'failed') {
    return 'error';
  }
  if (
    plugin.port_taken
    || plugin.program?.kind === 'stopped'
    || plugin.program?.kind === 'not_found'
  ) {
    return 'warning';
  }
  return plugin.last_event ? 'success' : 'neutral';
});

const confirming = ref<'regenerate' | 'remove' | null>(null);
const regenerating = computed({
  get: () => confirming.value === 'regenerate',
  set: (open) => {
    confirming.value = open ? 'regenerate' : null;
  },
});
const removing = computed({
  get: () => confirming.value === 'remove',
  set: (open) => {
    confirming.value = open ? 'remove' : null;
  },
});

async function regenerate(): Promise<void> {
  confirming.value = null;
  await plugins.regenerateToken(props.plugin.id);
}

async function remove(): Promise<void> {
  confirming.value = null;
  await plugins.remove(props.plugin.id);
}
</script>

<template>
  <section :aria-label="t('integrations.details')" class="flex flex-col gap-6 p-6">
    <div class="flex items-start gap-3">
      <UAvatar
        :icon="plugin.unreadable ? 'i-lucide-triangle-alert' : pluginIcon(plugin.icon)"
        size="xl"
        :ui="{
          root: 'rounded-lg bg-muted',
          icon: plugin.unreadable ? 'text-error' : 'text-default',
        }"
      />
      <div class="flex min-w-0 flex-1 flex-col gap-0.5">
        <div class="flex flex-wrap items-center gap-2">
          <h2 class="truncate text-base font-semibold text-highlighted">{{ plugin.name }}</h2>
          <UBadge
            v-if="driving"
            color="primary"
            variant="subtle"
            icon="i-lucide-palette"
            :label="t('integrations.driving')"
          />
        </div>
        <p v-if="plugin.version" class="text-sm text-muted">
          {{ t('integrations.meta', { version: plugin.version, author: plugin.author ?? '' }) }}
        </p>
      </div>
      <USwitch
        :model-value="plugin.enabled"
        :disabled="!!plugin.unreadable"
        :label="plugin.enabled ? t('integrations.on') : t('integrations.off')"
        @update:model-value="(on: boolean) => emit('toggle', plugin, on)"
      />
    </div>

    <p v-if="plugin.description" class="text-sm text-default">{{ plugin.description }}</p>

    <!-- On the baseline, the chip sits level with the lower case letters,
         below the middle of the line, and beside the first line of a long text. -->
    <p
      v-if="line"
      role="status"
      class="flex items-baseline gap-2 text-sm"
      :class="plugin.unreadable ? 'text-error' : 'text-muted'"
    >
      <!-- An empty chip, silent for screen readers; inset, or it keeps the
           shift of a chip on a corner. The neutral chip of Nuxt UI is the
           inverted color: ours is grey. -->
      <UChip
        standalone
        inset
        :color="dot"
        :ui="{ base: dot === 'neutral' ? 'bg-accented' : undefined }"
      />
      {{ line }}
    </p>

    <IntegrationsNote v-if="!fresh && plugin.note" :plugin="plugin" />

    <section v-if="!plugin.unreadable" class="flex flex-col gap-3">
      <h3 class="text-sm font-semibold text-highlighted">{{ t('integrations.can') }}</h3>
      <IntegrationsPermissions :plugin="plugin" />
    </section>

    <IntegrationsColors v-if="!fresh && plugin.capabilities.color" :plugin="plugin" />

    <footer class="mt-auto flex items-center gap-4 border-t border-default pt-4">
      <template v-if="!fresh && !plugin.unreadable && plugin.capabilities.events">
        <span class="text-xs text-muted">{{ t('integrations.tokenOwn') }}</span>
        <UButton color="neutral" variant="link" size="sm" class="p-0" @click="regenerating = true">
          {{ t('integrations.regenerate') }}
        </UButton>
      </template>
      <UButton
        color="error"
        variant="link"
        size="sm"
        class="ms-auto p-0"
        icon="i-lucide-trash-2"
        @click="removing = true"
      >
        {{ t('integrations.remove') }}
      </UButton>
    </footer>

    <IntegrationsConfirm
      v-model:open="regenerating"
      :title="t('integrations.regenerateConfirm.title', { name: plugin.name })"
      :text="t('integrations.regenerateConfirm.text')"
      :action="t('integrations.regenerateConfirm.action')"
      @confirm="regenerate"
    />
    <IntegrationsConfirm
      v-model:open="removing"
      :title="t('integrations.removeConfirm.title', { name: plugin.name })"
      :text="t('integrations.removeConfirm.text')"
      :action="t('integrations.removeConfirm.action')"
      danger
      @confirm="remove"
    />
  </section>
</template>
