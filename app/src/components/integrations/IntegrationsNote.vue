<!--
  SPDX-License-Identifier: GPL-3.0-or-later
  SPDX-FileCopyrightText: 2026 The Disled contributors

  The installation note of a plugin, shown as the plugin wrote it, in plain
  text, where only a passage between backquotes shows as code. Its block
  comes from the backend with the address and the token filled in, which
  stand out; "Copy" writes it to the clipboard. Under the block of a plugin
  with events, the promise of the app that it never changes the settings
  of the program. A first event folds the note.
-->
<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import type { FilledBlock } from '@/bindings/FilledBlock';
import type { PluginView } from '@/bindings/PluginView';
import { useCopy } from '@/composables/useCopy';
import { pluginNote } from '@/ipc/commands';
import { usePluginsStore } from '@/stores/plugins';

const props = defineProps<{ plugin: PluginView }>();

const { t } = useI18n();
const plugins = usePluginsStore();
const { copied, copy } = useCopy();

const unfolded = ref(false);
const folded = computed(() => props.plugin.installed && !unfolded.value);

/**
 * The filled block, asked for again when the files or the token of the
 * plugin change. The window keys the details by plugin, so the id stays.
 */
const block = ref<FilledBlock | null>(null);
watch(
  [() => props.plugin.fingerprint, () => plugins.tokenChanges],
  async () => {
    block.value = props.plugin.note?.block ? await pluginNote(props.plugin.id) : null;
  },
  { immediate: true },
);

/** A paragraph cut at its backquotes: every other piece is code. */
function pieces(paragraph: string): { code: boolean; text: string }[] {
  return paragraph.split('`').map((text, index) => ({ code: index % 2 === 1, text }));
}

/** A text that a regular expression matches as it is. */
function literal(text: string): string {
  return text.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

/** The block cut at the address and the token, which stand out. */
const blockPieces = computed(() => {
  const filled = block.value;
  if (!filled) {
    return [];
  }
  const marks = [filled.url, filled.token].filter((mark): mark is string => !!mark);
  const pattern = new RegExp(`(${marks.map(literal).join('|')})`);
  return filled.text
    .split(pattern)
    .map((text) => ({ text, marked: marks.includes(text) }))
    .filter((piece) => piece.text);
});
</script>

<template>
  <section class="flex flex-col gap-3">
    <div class="flex items-center justify-between gap-3">
      <h3 class="text-highlighted text-sm font-semibold">{{ t('integrations.install') }}</h3>
      <span class="text-muted text-xs">{{ t('integrations.noteSource') }}</span>
    </div>

    <div v-if="folded" class="flex items-center justify-between gap-3 text-sm">
      <span class="text-default flex items-center gap-2">
        <UIcon name="i-lucide-circle-check" class="text-success size-4" />
        {{ t('integrations.installed', { name: plugin.from ?? plugin.name }) }}
      </span>
      <UButton variant="link" size="sm" class="p-0" @click="unfolded = true">
        {{ t('integrations.reviewNote') }}
      </UButton>
    </div>

    <template v-else>
      <p
        v-for="(paragraph, index) in plugin.note?.paragraphs ?? []"
        :key="index"
        class="text-default text-sm"
      >
        <template v-for="(piece, at) in pieces(paragraph)" :key="at">
          <code v-if="piece.code" class="bg-muted rounded px-1 py-0.5 font-mono text-xs">{{
            piece.text
          }}</code>
          <template v-else>{{ piece.text }}</template>
        </template>
      </p>

      <UCard
        v-if="block"
        :ui="{
          header: 'flex items-center gap-2 bg-muted px-3 py-2 text-xs sm:px-3',
          body: 'p-0 sm:p-0',
        }"
      >
        <template #header>
          <UIcon name="i-lucide-file-json" class="text-muted size-4" />
          <span class="text-default font-medium">{{ plugin.note?.file }}</span>
          <i18n-t keypath="integrations.filled" tag="span" scope="global" class="text-muted">
            <template #address>
              <mark class="text-default bg-primary/10 rounded px-0.5">{{
                t('integrations.address')
              }}</mark>
            </template>
            <template #token>
              <mark class="text-default bg-primary/10 rounded px-0.5">{{
                t('integrations.token')
              }}</mark>
            </template>
          </i18n-t>
          <UButton
            color="neutral"
            variant="outline"
            size="xs"
            class="ms-auto"
            :icon="copied ? 'i-lucide-check' : 'i-lucide-copy'"
            @click="block && copy(block.text)"
          >
            {{ copied ? t('common.copied') : t('common.copy') }}
          </UButton>
        </template>
        <!-- Spans keep the spaces of the block, as a <pre> would. -->
        <figure
          class="text-default max-h-72 overflow-auto p-3 font-mono text-xs whitespace-pre"
          :aria-label="t('integrations.blockLabel', { file: plugin.note?.file ?? '' })"
        >
          <span
            v-for="(piece, at) in blockPieces"
            :key="at"
            :class="piece.marked ? 'text-highlighted bg-primary/10 rounded' : undefined"
            >{{
              piece.text
            }}</span
          >
        </figure>
      </UCard>

      <p
        v-if="block && plugin.capabilities.events"
        class="text-muted flex items-start gap-2 text-xs"
      >
        <UIcon name="i-lucide-lock" class="mt-0.5 size-3.5 shrink-0" />
        {{ t('integrations.neverEdits', { name: plugin.from ?? plugin.name }) }}
      </p>
    </template>
  </section>
</template>
