<!--
  SPDX-License-Identifier: GPL-3.0-or-later
  SPDX-FileCopyrightText: 2026 The Disled contributors

  The versions of the reinstall window, newest first, each on its row,
  with the link to the notes of the checked one. Past the height of its
  box, the list scrolls.
-->
<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { openLink } from '@/ipc/commands';

const props = defineProps<{
  /** Newest first. */
  versions: string[];
  /** The recommended version, the newest firmware 2.x. */
  latest: string | null;
  /** The version of the connected lamp. */
  current: string | null;
}>();

const chosen = defineModel<string | null>({ required: true });

const { t, locale } = useI18n();

const rows = computed(() => props.versions.map((version) => ({ label: version, value: version })));
</script>

<template>
  <div class="flex flex-col gap-1.5">
    <div class="flex items-center justify-between gap-3">
      <span id="version-label" class="text-default text-sm font-medium">
        {{ t('lampInstall.version') }}
      </span>
      <UButton
        v-if="chosen"
        variant="link"
        class="py-0"
        trailing-icon="i-lucide-external-link"
        :title="t('lampInstall.notesTitle')"
        @click="openLink({ release_notes: { version: chosen } })"
      >
        {{ t('lampInstall.notes', { version: chosen }) }}
      </UButton>
    </div>

    <div class="max-h-72 overflow-y-auto">
      <!-- Reka names each radio once, from its label and its badges: the
           group mounts again when they change. -->
      <URadioGroup
        :key="`${latest}|${current}|${locale}`"
        :model-value="chosen ?? undefined"
        :items="rows"
        variant="table"
        aria-labelledby="version-label"
        :ui="{
          item: 'py-2.5',
          container: 'h-6',
          wrapper: 'flex min-h-6 items-center',
          label: 'flex items-center gap-2 tabular-nums',
        }"
        @update:model-value="(version: string) => (chosen = version)"
      >
        <template #label="{ item }">
          {{ item.label }}
          <UBadge
            v-if="item.value === latest"
            color="primary"
            variant="subtle"
            :label="t('lampInstall.recommended')"
          />
          <UBadge
            v-if="item.value === current"
            color="warning"
            variant="subtle"
            :label="t('lampInstall.current')"
          />
        </template>
      </URadioGroup>
    </div>
  </div>
</template>
