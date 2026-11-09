<!--
  SPDX-License-Identifier: GPL-3.0-or-later
  SPDX-FileCopyrightText: 2026 The Disled contributors

  The Diagnostics window: the lamp's identity and its restart on top; its
  controls, its health and the self-test in tabs on the left; the live
  telemetry on the right, in view whatever the tab.
-->
<script setup lang="ts">
import type { TabsItem } from '@nuxt/ui';
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { useDiagnosticsStore } from '@/stores/diagnostics';
import { useLampStore } from '@/stores/lamp';
import { issueCount } from '@/utils/health';

const { t } = useI18n();
const diagnostics = useDiagnosticsStore();
const lamp = useLampStore();

/** Why a lamp that the app does not drive shows no tabs. */
const lockedLine = computed(() => {
  if (lamp.generation === 'v1') {
    return t('lampV1.line');
  }
  return lamp.generation === 'newer' ? t('lampNewer.text') : null;
});

const issues = computed(() => (diagnostics.diagnostics ? issueCount(diagnostics.diagnostics) : 0));

const tabs = computed<TabsItem[]>(() => [
  { value: 'lamp', slot: 'lamp', label: t('diagnostics.lamp'), icon: 'i-lucide-lamp-desk' },
  { value: 'health', slot: 'health', label: t('health.title'), icon: 'i-lucide-heart-pulse' },
  {
    value: 'self-test',
    slot: 'self-test',
    label: t('selfTest.title'),
    icon: 'i-lucide-list-checks',
  },
]);
</script>

<template>
  <main class="flex h-screen flex-col">
    <DiagnosticsHeader class="shrink-0 border-b border-default px-5 py-3" />

    <p v-if="lockedLine" class="p-5 text-sm text-muted">{{ lockedLine }}</p>
    <div v-else class="flex min-h-0 flex-1">
      <UTabs
        :items="tabs"
        default-value="lamp"
        variant="link"
        size="sm"
        class="w-80 shrink-0 border-e border-default"
        :ui="{ root: 'gap-0', list: 'px-3 pt-2', content: 'min-h-0 flex-1 overflow-y-auto p-4' }"
      >
        <!-- The number goes through the label, which takes the text color of
             every badge; the sentence beside it is for screen readers. -->
        <template #trailing="{ item }">
          <template v-if="item.value === 'health' && issues">
            <UBadge
              color="warning"
              variant="subtle"
              size="sm"
              :label="String(issues)"
              aria-hidden="true"
            />
            <span class="sr-only">{{ t('health.issues', issues) }}</span>
          </template>
        </template>
        <template #lamp><DiagnosticsLamp /></template>
        <template #health><DiagnosticsHealth /></template>
        <template #self-test><DiagnosticsSelfTest /></template>
      </UTabs>

      <DiagnosticsTelemetry class="min-w-0 flex-1" />
    </div>
  </main>
</template>
