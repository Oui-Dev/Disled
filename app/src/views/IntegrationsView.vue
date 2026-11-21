<!--
  SPDX-License-Identifier: GPL-3.0-or-later
  SPDX-FileCopyrightText: 2026 The Disled contributors

  The integrations window: the plugins of the folder of the plugins, which
  bring every integration, and the catalog, in a tab of its own, read from
  the network only when the tab opens. A plugin comes disabled; turning it
  on for the first time, or after its files changed, asks the question of
  what it will be able to do, as an import or an install from the catalog
  does. The folder is read again whenever the window gets the focus.
-->
<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import type { PluginView } from '@/bindings/PluginView';
import { usePluginTexts } from '@/composables/usePluginTexts';
import { usePluginsStore } from '@/stores/plugins';

const { t } = useI18n();
const plugins = usePluginsStore();
const { reason } = usePluginTexts();

/** The tab shown: the plugins installed, or the catalog. */
const tab = ref<string>('installed');
const tabs = computed(() => [
  { label: t('integrations.tabs.installed'), value: 'installed', icon: 'i-lucide-puzzle' },
  { label: t('integrations.tabs.catalog'), value: 'catalog', icon: 'i-lucide-store' },
]);
watch(tab, (shown) => {
  if (shown === 'catalog') {
    void plugins.loadCatalog();
  }
});

const selected = ref<string | null>(null);
const current = computed(
  () => plugins.plugins.find((plugin) => plugin.id === selected.value) ?? null,
);
// The first plugin is selected, and a plugin that left gives its place.
watch(
  () => plugins.plugins,
  (list) => {
    if (!list.some((plugin) => plugin.id === selected.value)) {
      selected.value = list[0]?.id ?? null;
    }
  },
  { immediate: true },
);

/**
 * The plugin whose question is open, once it is listed: the answer of an
 * import or an install may come before the list that holds the plugin, or
 * after it.
 */
const approving = ref<string | null>(null);
const asked = computed(
  () => plugins.plugins.find((plugin) => plugin.id === approving.value) ?? null,
);
const asking = computed({
  get: () => asked.value !== null,
  set: (open) => {
    if (!open) {
      approving.value = null;
    }
  },
});
// A plugin that leaves the list takes its question along, or importing it
// again would reopen that question.
watch(asked, (now, before) => {
  if (before && !now) {
    approving.value = null;
  }
});

/** On wants the question unless the user approved these files already. */
function toggle(plugin: PluginView, on: boolean): void {
  if (on && (!plugin.approved || plugin.modified)) {
    approving.value = plugin.id;
    return;
  }
  void plugins.setEnabled(plugin.id, on);
}

async function approve(id: string, fingerprint: string): Promise<void> {
  approving.value = null;
  await plugins.approve(id, fingerprint);
}

/** A plugin that arrived, from an import or the catalog: its question follows. */
function review(id: string): void {
  tab.value = 'installed';
  selected.value = id;
  approving.value = id;
}

async function importPlugin(): Promise<void> {
  const id = await plugins.importPlugin();
  if (id) {
    review(id);
  }
}
</script>

<template>
  <main class="flex h-screen flex-col">
    <header class="border-default flex items-start justify-between gap-4 border-b px-6 py-4">
      <div>
        <h1 class="text-highlighted text-lg font-bold">{{ t('integrations.title') }}</h1>
        <p class="text-muted text-sm">{{ t('integrations.subtitle') }}</p>
      </div>
      <div class="flex shrink-0 gap-2">
        <UButton
          color="neutral"
          variant="outline"
          size="sm"
          icon="i-lucide-folder-open"
          @click="plugins.openFolder"
        >
          {{ t('integrations.openFolder') }}
        </UButton>
        <UButton
          color="neutral"
          variant="outline"
          size="sm"
          icon="i-lucide-import"
          @click="importPlugin"
        >
          {{ t('integrations.import') }}
        </UButton>
      </div>
    </header>

    <UAlert
      v-if="plugins.importRefused"
      role="alert"
      color="error"
      variant="subtle"
      icon="i-lucide-circle-x"
      :title="t('integrations.importFailed')"
      :description="reason(plugins.importRefused)"
      class="mx-6 mt-4 w-auto"
    />
    <ErrorAlert class="mx-6 mt-4 w-auto" />

    <UTabs
      v-model="tab"
      :items="tabs"
      :content="false"
      variant="link"
      size="sm"
      class="border-default shrink-0 border-b px-4"
      :ui="{ list: 'border-0' }"
    />

    <div v-if="tab === 'installed'" class="flex min-h-0 flex-1">
      <IntegrationsList
        v-model="selected"
        class="border-default w-72 shrink-0 border-e"
        @toggle="toggle"
      />
      <div class="min-w-0 flex-1 overflow-y-auto">
        <IntegrationsDetail v-if="current" :key="current.id" :plugin="current" @toggle="toggle" />
      </div>
    </div>
    <IntegrationsCatalog v-else class="min-h-0 flex-1 overflow-y-auto" @installed="review" />

    <IntegrationsApproval v-model:open="asking" :plugin="asked" @approve="approve" />
  </main>
</template>
