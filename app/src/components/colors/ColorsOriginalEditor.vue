<!--
  SPDX-License-Identifier: GPL-3.0-or-later
  SPDX-FileCopyrightText: 2026 The Disled contributors

  The editor of a built-in color: its name only, since its light never
  changes. A name equal to the default one is saved as none, and then
  follows the language of the app. The page saves through `save`, which the
  popup footer calls.
-->
<script setup lang="ts">
import { computed, onMounted, ref, useTemplateRef } from 'vue';
import { useI18n } from 'vue-i18n';
import { defaultName, useColorsStore } from '@/stores/colors';

const props = defineProps<{ slot: number }>();
const emit = defineEmits<{ done: [] }>();

const { t } = useI18n();
const colors = useColorsStore();

const color = computed(() => colors.builtIn[props.slot]);
const original = computed(() => defaultName(props.slot));
const name = ref(color.value?.name ?? '');
const canSave = computed(() => name.value.trim().length > 0);

/** The page opens with the focus in the name. */
const nameInput = useTemplateRef<{ inputRef: HTMLInputElement | null }>('nameInput');
onMounted(() => nameInput.value?.inputRef?.focus());

async function save(): Promise<void> {
  if (!canSave.value) {
    return;
  }
  const trimmed = name.value.trim();
  await colors.renameOriginal(props.slot, trimmed === original.value ? null : trimmed);
  emit('done');
}

defineExpose({ save, canSave });
</script>

<template>
  <div class="flex flex-col gap-4">
    <h2
      data-page-heading
      tabindex="-1"
      class="flex items-center gap-2 text-sm font-semibold text-highlighted focus:outline-none"
    >
      <span
        aria-hidden="true"
        class="size-5 shrink-0 rounded-[20%] border border-accented"
        :style="{ background: color?.hex }"
      />
      {{ color?.name }}
    </h2>

    <div class="flex flex-col gap-1.5">
      <div class="flex items-center justify-between gap-2">
        <label for="original-name" class="text-sm text-default">{{
          t('colors.editor.name')
        }}</label>
        <UButton
          v-if="name.trim() !== original"
          color="primary"
          variant="link"
          size="xs"
          icon="i-lucide-rotate-ccw"
          class="px-0"
          @click="name = original"
        >
          {{ t('colors.editor.restore', { name: original }) }}
        </UButton>
      </div>
      <UInput
        id="original-name"
        ref="nameInput"
        v-model="name"
        maxlength="16"
        @keydown.enter="save"
      />
    </div>

    <p class="flex items-start gap-2 rounded-md border border-default p-3 text-xs text-muted">
      <UIcon name="i-lucide-lock" class="mt-0.5 size-3.5 shrink-0" />
      <span>{{ t('colors.editor.fixed') }}</span>
    </p>
  </div>
</template>
