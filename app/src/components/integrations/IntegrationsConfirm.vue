<!--
  SPDX-License-Identifier: GPL-3.0-or-later
  SPDX-FileCopyrightText: 2026 The Disled contributors

  A short question before a step that cannot be undone: a new token, or
  the removal of a plugin.
-->
<script setup lang="ts">
import { useI18n } from 'vue-i18n';

defineProps<{ title: string; text: string; action: string; danger?: boolean }>();
const open = defineModel<boolean>('open', { required: true });
const emit = defineEmits<{ confirm: [] }>();

const { t } = useI18n();
</script>

<template>
  <UModal v-model:open="open" :title="title" :close="{ 'aria-label': t('common.close') }">
    <template #body>
      <p class="text-sm text-default">{{ text }}</p>
    </template>
    <template #footer>
      <div class="flex w-full justify-end gap-2">
        <UButton color="neutral" variant="outline" @click="open = false">
          {{ t('common.cancel') }}
        </UButton>
        <UButton :color="danger ? 'error' : 'primary'" @click="emit('confirm')">
          {{ action }}
        </UButton>
      </div>
    </template>
  </UModal>
</template>
