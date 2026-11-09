<!--
  SPDX-License-Identifier: GPL-3.0-or-later
  SPDX-FileCopyrightText: 2026 The Disled contributors

  The versions of the reinstall window, with the link to the notes of the
  checked one. Up to four versions, all of them show. Beyond, only the
  recommended one, the lamp's and the one picked last show, never more than
  four lines with "Other versions", whose menu lists the others by firmware
  line, newest first. A version picked there joins the list, checked.
-->
<script setup lang="ts">
import type { ListboxItem } from '@nuxt/ui';
import { computed, nextTick, ref, useTemplateRef, watch } from 'vue';
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

/** More versions than this, and the others go to the menu. */
const MAX_ROWS = 4;

const { t } = useI18n();

const crowded = computed(() => props.versions.length > MAX_ROWS);
/** The version last picked in the menu, which stays in the list. */
const picked = ref<string | null>(null);
watch(
  () => props.versions,
  (versions) => {
    if (picked.value && !versions.includes(picked.value)) {
      picked.value = null;
    }
  },
);

const rows = computed(() => {
  const shown = crowded.value
    ? props.versions.filter((version) =>
        [props.latest, props.current, picked.value, chosen.value].includes(version),
      )
    : props.versions;
  return shown.map((version) => ({ label: version, value: version }));
});

/** The versions of the menu, under "Firmware v2" then "Firmware v1". */
const others = computed(() => {
  const groups = new Map<string, ListboxItem[]>();
  for (const version of props.versions) {
    if (version === props.latest || version === props.current) {
      continue;
    }
    const major = version.split('.')[0];
    let group = groups.get(major);
    if (!group) {
      group = [{ type: 'label', label: t('lampInstall.group', { major }) }];
      groups.set(major, group);
    }
    group.push({ label: version, value: version });
  }
  return [...groups.values()];
});

/** The menu checks the chosen version when it lists it. */
const inMenu = computed(() =>
  others.value.flat().some((item) => item.value === chosen.value) ? chosen.value : undefined,
);

const open = ref(false);
const list = useTemplateRef<HTMLElement>('list');
let focusChosen = false;

function pick(value: unknown): void {
  open.value = false;
  // A click on the version already checked unchecks it in the menu, and
  // changes nothing here.
  if (typeof value !== 'string') {
    return;
  }
  picked.value = value;
  chosen.value = value;
  focusChosen = true;
}

/** The focus goes to the version picked, in the list, rather than back to the menu button. */
async function closeAutoFocus(event: Event): Promise<void> {
  if (!focusChosen) {
    return;
  }
  focusChosen = false;
  event.preventDefault();
  await nextTick();
  list.value?.querySelector<HTMLElement>('[role="radio"][data-state="checked"]')?.focus();
}
</script>

<template>
  <div class="flex flex-col gap-1.5">
    <div class="flex items-center justify-between gap-3">
      <span id="version-label" class="text-sm font-medium text-default">
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

    <div ref="list" class="flex flex-col">
      <URadioGroup
        :model-value="chosen ?? undefined"
        :items="rows"
        variant="table"
        aria-labelledby="version-label"
        :ui="{
          item: ['py-2.5', crowded && 'last-of-type:rounded-b-none'],
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

      <UPopover
        v-if="crowded"
        v-model:open="open"
        :content="{ align: 'start', onCloseAutoFocus: closeAutoFocus }"
      >
        <button
          type="button"
          class="-mt-px flex w-full items-center gap-2 rounded-b-lg border border-default px-3.5 py-3 text-sm font-medium text-muted transition-colors hover:bg-elevated/50 focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-primary aria-expanded:bg-elevated/50"
        >
          {{ t('lampInstall.others') }}
          <UIcon
            name="i-lucide-chevron-down"
            class="size-4 text-dimmed transition-transform"
            :class="{ 'rotate-180': open }"
          />
        </button>

        <template #content>
          <UListbox
            :items="others"
            value-key="value"
            :model-value="inMenu"
            :aria-label="t('lampInstall.others')"
            class="w-50"
            :ui="{ root: 'rounded-md ring-0', label: 'text-xs' }"
            @update:model-value="pick"
          />
        </template>
      </UPopover>
    </div>
  </div>
</template>
