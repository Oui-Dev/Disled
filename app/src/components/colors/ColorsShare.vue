<!--
  SPDX-License-Identifier: GPL-3.0-or-later
  SPDX-FileCopyrightText: 2026 The Disled contributors

  Sharing the colors by a code. "Send" shows the code of this computer and
  copies it; "Receive" reads the code the user pastes into its field, never
  the clipboard. A code that reads opens the preview of the import.
-->
<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import type { Palette } from '@/bindings/Palette';
import { colorsCopyCode, colorsReadCode, colorsShareCode } from '@/ipc/commands';
import { useColorsStore } from '@/stores/colors';
import { useErrorStore } from '@/stores/errors';

const emit = defineEmits<{ received: [text: string, palette: Palette] }>();

/** How long "Copied" shows. */
const COPIED_MS = 2000;

const { t } = useI18n();
const colors = useColorsStore();
const errors = useErrorStore();

const code = ref('');
const copied = ref(false);
const received = ref('');
const unreadable = ref(false);
let copiedTimer: ReturnType<typeof setTimeout> | undefined;

async function loadCode(): Promise<void> {
  code.value = await colorsShareCode();
}
onMounted(loadCode);
// The code follows the colors, changed here or in another window.
watch(() => colors.palette, loadCode, { deep: true });

async function copy(): Promise<void> {
  try {
    await colorsCopyCode();
    copied.value = true;
    clearTimeout(copiedTimer);
    copiedTimer = setTimeout(() => {
      copied.value = false;
    }, COPIED_MS);
  } catch (error) {
    errors.reportFailure(error);
  }
}
onBeforeUnmount(() => clearTimeout(copiedTimer));

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
      class="text-sm font-semibold text-highlighted focus:outline-none"
    >
      {{ t('colors.share.title') }}
    </h2>

    <section class="flex flex-col gap-2">
      <h3 class="text-xs font-semibold text-muted">{{ t('colors.share.send') }}</h3>
      <p class="text-xs text-default">{{ t('colors.share.sendHint') }}</p>
      <p class="truncate rounded-md bg-elevated px-2 py-1.5 font-mono text-xs text-default">
        {{ code }}
      </p>
      <div class="flex items-center gap-3">
        <UButton color="neutral" variant="outline" size="sm" icon="i-lucide-copy" @click="copy">
          {{ t('colors.share.copy') }}
        </UButton>
        <span role="status" class="flex items-center gap-1 text-xs text-muted">
          <template v-if="copied">
            <UIcon name="i-lucide-circle-check" class="size-3.5 text-success" />
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
