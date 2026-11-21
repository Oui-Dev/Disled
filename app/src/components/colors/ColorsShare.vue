<!--
  SPDX-License-Identifier: GPL-3.0-or-later
  SPDX-FileCopyrightText: 2026 The Disled contributors

  Sharing the colors by a code. "Send" shows the code of this computer and
  copies it; "Receive" reads the code the user pastes into its field, never
  the clipboard. A code that reads opens the preview of the import.
-->
<script setup lang="ts">
import { ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import type { Palette } from '@/bindings/Palette';
import { useCopy } from '@/composables/useCopy';
import { colorsReadCode, colorsShareCode } from '@/ipc/commands';
import { useColorsStore } from '@/stores/colors';

const emit = defineEmits<{ received: [text: string, palette: Palette] }>();

const { t } = useI18n();
const colors = useColorsStore();
const { copied, copy } = useCopy();

const code = ref('');
const received = ref('');
const unreadable = ref(false);

async function loadCode(): Promise<void> {
  code.value = await colorsShareCode();
}
// The code follows the colors, changed here or in another window, which
// the store replaces whole.
watch(() => colors.palette, loadCode, { immediate: true });

/** The field is read at every change; an empty one says nothing. */
async function read(text: string): Promise<void> {
  received.value = text;
  if (text.trim() === '') {
    unreadable.value = false;
    return;
  }
  const palette = await colorsReadCode(text);
  // A newer text may have come meanwhile.
  if (text !== received.value) {
    return;
  }
  unreadable.value = palette === null;
  if (palette) {
    emit('received', text, palette);
  }
}
</script>

<template>
  <div class="flex flex-col gap-4">
    <h2
      data-page-heading
      tabindex="-1"
      class="text-highlighted text-sm font-semibold focus:outline-none"
    >
      {{ t('colors.share.title') }}
    </h2>

    <section class="flex flex-col gap-2">
      <h3 class="text-muted text-xs font-semibold">{{ t('colors.share.send') }}</h3>
      <p class="text-default text-xs">{{ t('colors.share.sendHint') }}</p>
      <p class="bg-elevated text-default truncate rounded-md px-2 py-1.5 font-mono text-xs">
        {{ code }}
      </p>
      <div class="flex items-center gap-3">
        <UButton
          color="neutral"
          variant="outline"
          size="sm"
          icon="i-lucide-copy"
          @click="copy(code)"
        >
          {{ t('colors.share.copy') }}
        </UButton>
        <span role="status" class="text-muted flex items-center gap-1 text-xs">
          <template v-if="copied">
            <UIcon name="i-lucide-circle-check" class="text-success size-3.5" />
            {{ t('common.copied') }}
          </template>
        </span>
      </div>
    </section>

    <USeparator />

    <!-- The label looks like the headings of the page. -->
    <UFormField
      :label="t('colors.share.receive')"
      :error="unreadable ? t('colors.share.invalid') : false"
      :ui="{ label: 'text-xs font-semibold text-muted', container: 'mt-2', error: 'text-xs' }"
    >
      <UInput
        :model-value="received"
        :placeholder="t('colors.share.placeholder')"
        class="w-full"
        @update:model-value="read(String($event))"
      />
      <!-- Said by screen readers as soon as it shows. -->
      <template #error="{ error }">
        <span role="alert">{{ error }}</span>
      </template>
    </UFormField>
  </div>
</template>
