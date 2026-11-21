<!--
  SPDX-License-Identifier: GPL-3.0-or-later
  SPDX-FileCopyrightText: 2026 The Disled contributors

  A question before a step that matters: its title, a description under
  it, then its text or a body of its own, and Cancel beside the action, red
  for a step that cannot be undone.
-->
<script setup lang="ts">
import { useI18n } from 'vue-i18n';

defineProps<{
  title: string;
  description?: string;
  /** The question, for a modal without a body of its own. */
  text?: string;
  action: string;
  danger?: boolean;
  icon?: string;
  disabled?: boolean;
}>();
const open = defineModel<boolean>('open', { required: true });
const emit = defineEmits<{ confirm: [] }>();
const slots = defineSlots<{ body?(): unknown }>();

const { t } = useI18n();
</script>

<template>
  <UModal v-model:open="open" :title="title" :description="description">
    <template v-if="slots.body || text" #body>
      <slot name="body">
        <p class="text-default text-sm">{{ text }}</p>
      </slot>
    </template>
    <template #footer>
      <div class="flex w-full justify-end gap-2">
        <UButton color="neutral" variant="outline" @click="open = false">
          {{ t('common.cancel') }}
        </UButton>
        <UButton
          :color="danger ? 'error' : 'primary'"
          :icon="icon"
          :disabled="disabled"
          @click="emit('confirm')"
        >
          {{ action }}
        </UButton>
      </div>
    </template>
  </UModal>
</template>
