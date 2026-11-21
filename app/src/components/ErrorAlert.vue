<!--
  SPDX-License-Identifier: GPL-3.0-or-later
  SPDX-FileCopyrightText: 2026 The Disled contributors

  The last error of the window, with a button that closes it: the cause in
  the user's language, the backend's detail as it came. UAlert has no role
  of its own, so this one gets alert, for screen readers.
-->
<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { useErrorStore } from '@/stores/errors';

const { t } = useI18n();
const errors = useErrorStore();

const text = computed(() => {
  const error = errors.last;
  switch (error?.kind) {
    case 'backend':
      return t('errors.backend', { detail: error.detail });
    case 'rejected':
      return t('errors.rejected', {
        command: `0x${error.command.toString(16).padStart(2, '0')}`,
        error: t(`enums.protocolError.${error.error}`),
      });
    case 'unreadable':
      return t('errors.unreadable', { detail: error.detail });
    default:
      return null;
  }
});
</script>

<template>
  <UAlert
    v-if="text"
    role="alert"
    color="error"
    variant="subtle"
    icon="i-lucide-circle-x"
    :description="text"
    :close="{ size: 'sm' }"
    @update:open="errors.clear()"
  />
</template>
