<!--
  SPDX-License-Identifier: GPL-3.0-or-later
  SPDX-FileCopyrightText: 2026 The Disled contributors

  The catalog of plugins, which the project reviews at every version: one
  card a plugin, with what it can do, from its manifest only, and its
  install button, its update, or the fact that it is installed. Its texts
  show as plain text, its icon from the bundled list. An install ends with
  the approval question, which the window asks.
-->
<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import type { CatalogError } from '@/bindings/CatalogError';
import type { CatalogView } from '@/bindings/CatalogView';
import { usePluginTexts } from '@/composables/usePluginTexts';
import { usePluginsStore } from '@/stores/plugins';
import { pluginIcon } from '@/utils/pluginIcons';

const emit = defineEmits<{ installed: [id: string] }>();

const { t } = useI18n();
const plugins = usePluginsStore();
const { reason } = usePluginTexts();

function errorText(error: CatalogError): string {
  switch (error.kind) {
    case 'network':
      return t('integrations.catalog.error.network');
    case 'signature':
      return t('integrations.catalog.error.signature');
    case 'index':
      return t('integrations.catalog.error.index');
    case 'missing':
      return t('integrations.catalog.error.missing');
    case 'mismatch':
      return t('integrations.catalog.error.mismatch');
    case 'unreadable':
      return t('integrations.catalog.error.unreadable', { reason: reason(error.reason) });
  }
}

/** What a plugin can do, in short, in the order of the approval. */
function abilities(entry: CatalogView): string[] {
  const { capabilities } = entry;
  const shown: string[] = [];
  if (capabilities.run) {
    shown.push(
      entry.needs
        ? t('integrations.catalog.cap.runs', { program: entry.needs })
        : t('integrations.catalog.cap.run'),
    );
  }
  if (capabilities.color) {
    shown.push(t('integrations.catalog.cap.color'));
  }
  if (capabilities.alert) {
    shown.push(t('integrations.catalog.cap.alert'));
  }
  if (capabilities.lamp) {
    shown.push(t('integrations.catalog.cap.lamp'));
  }
  if (capabilities.events) {
    shown.push(t('integrations.catalog.cap.events', { name: entry.from ?? '' }));
  }
  return shown;
}

async function install(entry: CatalogView): Promise<void> {
  const id = await plugins.install(entry.id);
  if (id) {
    emit('installed', id);
  }
}
</script>

<template>
  <section :aria-label="t('integrations.catalog.label')" class="flex flex-col gap-4 p-6">
    <p class="text-sm text-muted">{{ t('integrations.catalog.intro') }}</p>

    <UAlert
      v-if="plugins.catalogError"
      role="alert"
      color="error"
      variant="subtle"
      icon="i-lucide-circle-x"
      :description="errorText(plugins.catalogError)"
      :actions="[
        {
          label: t('integrations.catalog.retry'),
          color: 'neutral',
          variant: 'outline',
          onClick: () => plugins.loadCatalog(),
        },
      ]"
    />

    <p
      v-if="plugins.catalogLoading && !plugins.catalog"
      class="flex items-center gap-2 text-sm text-muted"
    >
      <UIcon name="i-lucide-loader-circle" class="size-4 animate-spin" />
      {{ t('integrations.catalog.loading') }}
    </p>
    <UEmpty
      v-else-if="plugins.catalog && !plugins.catalog.length"
      variant="naked"
      size="md"
      :description="t('integrations.catalog.empty')"
    />
    <ul v-else-if="plugins.catalog" class="flex flex-col gap-3">
      <li
        v-for="entry in plugins.catalog"
        :key="entry.id"
        class="flex gap-3 rounded-lg border border-default p-4"
      >
        <UAvatar
          :icon="pluginIcon(entry.icon)"
          :ui="{ root: 'rounded-md bg-muted', icon: 'text-default' }"
        />
        <div class="flex min-w-0 flex-1 flex-col gap-2">
          <div class="flex items-start justify-between gap-3">
            <div class="min-w-0">
              <h3 class="truncate text-sm font-medium text-highlighted">{{ entry.name }}</h3>
              <p class="text-xs text-muted">
                {{
                  t('integrations.catalog.meta', { version: entry.version, author: entry.author })
                }}
              </p>
            </div>
            <UButton
              v-if="entry.installed === null || entry.update"
              size="sm"
              class="shrink-0"
              :icon="entry.installed === null ? 'i-lucide-download' : 'i-lucide-refresh-cw'"
              :loading="plugins.installing === entry.id"
              :disabled="plugins.installing !== null"
              @click="install(entry)"
            >
              {{
                entry.installed === null
                  ? t('integrations.catalog.install')
                  : t('integrations.catalog.update', { version: entry.version })
              }}
            </UButton>
            <UBadge
              v-else
              class="shrink-0"
              :color="entry.current ? 'success' : 'neutral'"
              variant="subtle"
              icon="i-lucide-check"
              :label="t('integrations.catalog.installed')"
            />
          </div>
          <p class="text-sm text-default">{{ entry.description }}</p>
          <div class="flex flex-wrap items-center gap-1.5">
            <UBadge
              v-for="ability in abilities(entry)"
              :key="ability"
              color="neutral"
              variant="soft"
              size="sm"
              :label="ability"
            />
            <UButton
              color="neutral"
              variant="link"
              size="xs"
              trailing-icon="i-lucide-external-link"
              :aria-label="t('integrations.catalog.sourceLabel', { name: entry.name })"
              @click="plugins.openSource(entry.id)"
            >
              {{ t('integrations.catalog.source') }}
            </UButton>
          </div>
        </div>
      </li>
    </ul>
  </section>
</template>
