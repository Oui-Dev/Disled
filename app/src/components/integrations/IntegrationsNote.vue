<!--
  SPDX-License-Identifier: GPL-3.0-or-later
  SPDX-FileCopyrightText: 2026 The Disled contributors

  The installation note of a plugin, shown as the plugin wrote it, in plain
  text, where only a passage between backquotes shows as code. Its block
  comes from the backend with the address and the token filled in, which
  stand out; "Copy" has the backend write the clipboard. Under the block of
  a plugin with events, the promise of the app that it never changes the
  settings of the program. A first event folds the note.
-->
<script setup lang="ts">
import { useTimeoutFn } from '@vueuse/core';
import { computed, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import type { FilledBlock } from '@/bindings/FilledBlock';
import type { PluginView } from '@/bindings/PluginView';
import { pluginNote } from '@/ipc/commands';
import { usePluginsStore } from '@/stores/plugins';

const props = defineProps<{ plugin: PluginView }>();

const { t } = useI18n();
const plugins = usePluginsStore();

const unfolded = ref(false);
const folded = computed(() => props.plugin.installed && !unfolded.value);

/** The filled block, asked for anew when the plugin or its token change. */
const block = ref<FilledBlock | null>(null);
watch(
  () => [props.plugin.id, props.plugin.installed, props.plugin.fingerprint] as const,
  async ([id]) => {
    block.value = props.plugin.note?.block ? await pluginNote(id) : null;
  },
  { immediate: true },
);
watch(
  () => props.plugin.id,
  () => {
    unfolded.value = false;
  },
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

const copied = ref(false);
const { start: forgetCopied } = useTimeoutFn(
  () => {
    copied.value = false;
  },
  2000,
  { immediate: false },
);

async function copy(): Promise<void> {
  if (await plugins.copyNote(props.plugin.id)) {
    copied.value = true;
    forgetCopied();
  }
}
</script>

<template>
  <section class="flex flex-col gap-3">
    <div class="flex items-center justify-between gap-3">
      <h3 class="text-sm font-semibold text-highlighted">{{ t('integrations.install') }}</h3>
      <span class="text-xs text-muted">{{ t('integrations.noteSource') }}</span>
    </div>

    <div v-if="folded" class="flex items-center justify-between gap-3 text-sm">
      <span class="flex items-center gap-2 text-default">
        <UIcon name="i-lucide-circle-check" class="size-4 text-success" />
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
        class="text-sm text-default"
      >
        <template v-for="(piece, at) in pieces(paragraph)" :key="at">
          <code v-if="piece.code" class="rounded bg-muted px-1 py-0.5 font-mono text-xs">{{
            piece.text
          }}</code>
          <template v-else>{{ piece.text }}</template>
        </template>
      </p>

      <div v-if="block" class="overflow-hidden rounded-lg border border-default">
        <div class="flex items-center gap-2 border-b border-default bg-muted px-3 py-2 text-xs">
          <UIcon name="i-lucide-file-json" class="size-4 text-muted" />
          <span class="font-medium text-default">{{ plugin.note?.file }}</span>
          <i18n-t keypath="integrations.filled" tag="span" scope="global" class="text-muted">
            <template #address>
              <mark class="rounded bg-primary/10 px-0.5 text-default">{{
                t('integrations.address')
              }}</mark>
            </template>
            <template #token>
              <mark class="rounded bg-primary/10 px-0.5 text-default">{{
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
            @click="copy"
          >
            {{ copied ? t('common.copied') : t('common.copy') }}
          </UButton>
        </div>
        <!-- Spans keep the spaces of the block, as a <pre> would. -->
        <figure
          class="max-h-72 overflow-auto p-3 font-mono text-xs whitespace-pre text-default"
          :aria-label="t('integrations.blockLabel', { file: plugin.note?.file ?? '' })"
        >
          <span
            v-for="(piece, at) in blockPieces"
            :key="at"
            :class="piece.marked ? 'rounded bg-primary/10 text-highlighted' : undefined"
            >{{
              piece.text
            }}</span
          >
        </figure>
      </div>

      <p
        v-if="block && plugin.capabilities.events"
        class="flex items-start gap-2 text-xs text-muted"
      >
        <UIcon name="i-lucide-lock" class="mt-0.5 size-3.5 shrink-0" />
        {{ t('integrations.neverEdits', { name: plugin.from ?? plugin.name }) }}
      </p>
    </template>
  </section>
</template>
