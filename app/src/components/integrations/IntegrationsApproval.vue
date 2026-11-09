<!--
  SPDX-License-Identifier: GPL-3.0-or-later
  SPDX-FileCopyrightText: 2026 The Disled contributors

  The question that enables a plugin, after it arrives, after an import,
  or after its files changed: what it will be able to do, and a word on
  trust. "Enable" sends back the fingerprint of the files that the
  question showed, so that a change in between asks again.
-->
<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import type { PluginView } from '@/bindings/PluginView';

defineProps<{ plugin: PluginView | null }>();
const open = defineModel<boolean>('open', { required: true });
const emit = defineEmits<{ approve: [id: string, fingerprint: string] }>();

const { t } = useI18n();
</script>

<template>
  <UModal
    v-model:open="open"
    :title="plugin ? t('integrations.approve.title', { name: plugin.name }) : ''"
    :description="
      plugin
        ? t('integrations.approve.meta', {
            version: plugin.version ?? '',
            author: plugin.author ?? '',
          })
        : ''
    "
    :close="{ 'aria-label': t('common.close') }"
  >
    <template #body>
      <div v-if="plugin" class="flex flex-col gap-4 text-sm text-default">
        <p>{{ plugin.description }}</p>
        <div class="flex flex-col gap-2">
          <p class="font-medium text-highlighted">{{ t('integrations.approve.can') }}</p>
          <IntegrationsPermissions :plugin="plugin" />
        </div>
        <p class="text-muted">{{ t('integrations.approve.trust') }}</p>
      </div>
    </template>
    <template #footer>
      <div class="flex w-full justify-end gap-2">
        <UButton color="neutral" variant="outline" @click="open = false">
          {{ t('common.cancel') }}
        </UButton>
        <UButton
          :disabled="!plugin?.fingerprint"
          @click="plugin?.fingerprint && emit('approve', plugin.id, plugin.fingerprint)"
        >
          {{ t('integrations.approve.action') }}
        </UButton>
      </div>
    </template>
  </UModal>
</template>
